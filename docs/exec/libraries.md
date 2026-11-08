# Libraries, interfaces and the raw layer

This page covers how Rust code reaches AmigaOS libraries, what the raw
`sys` bindings are, and the low-level machine helpers:

| Module | What it gives you | Cargo feature |
|---|---|---|
| `library` | `Library`, `Interface<I>`, `open_resource` | none |
| (internal) `iface` | How safe modules open their own interfaces | (automatic) |
| `sys` | Raw bindings to every SDK interface | exec always; one feature per interface |
| `expansion` | `machine_info()` | `expansion` |
| `arch` | PowerPC cache, barrier, timebase and MMIO instructions | none (AmigaOS target only) |

## AmigaOS terms used here

- **Library**: a shared module in memory (`exec.library`,
  `utility.library`, ...). You open it with `OpenLibrary` and close it
  with `CloseLibrary`. Opening by name may load it from disk.
- **Interface**: on AmigaOS 4, a library's functions are called through an
  **interface**, a table of function pointers (a vtable) you get with
  `GetInterface` and give back with `DropInterface`. Most libraries call
  their interface `"main"`.
- **Interface global**: by convention a program keeps each interface in a
  global variable: `IExec`, `IDOS`, `IUtility`, ... The SDK's C headers,
  and the raw `sys` wrappers here, call through those globals.
- **Resource**: a library-like module that is opened but never closed.

---

## `library`: opening a library yourself

`Library::open(name, version)` calls `OpenLibrary`; dropping the
`Library` calls `CloseLibrary`. `library.interface::<I>(name, version)`
calls `GetInterface` and returns an `Interface<I>` that **borrows** the
library, so the compiler makes sure the interface is dropped
(`DropInterface`) before the library is closed.

```rust
use amigaos4::library::Library;
use amigaos4::sys;

fn utility_version() -> amigaos4::Result<(u16, u16)> {
    let lib = Library::open("utility.library", 50)?;  // &str, no NUL needed
    let iface = lib.interface::<sys::UtilityIFace>("main", 1)?;
    let _raw: *mut sys::UtilityIFace = iface.as_ptr();
    // Calls through `_raw` are unsafe: each must meet its autodoc.
    Ok((lib.version(), lib.revision()))
}
```

| Item | Meaning |
|---|---|
| `Library::open(name, version)` | `Err(NullPointer)` if the library cannot be found or is older than `version`. |
| `version()`, `revision()` | `lib_Version`, `lib_Revision`. |
| `id_string()` | `lib_IdString` (`"name ver.rev (date)"`), if set and UTF-8. |
| `interface::<I>(name, version)` | The interface as vtable type `I` (`sys::ExecIFace`, `sys::UtilityIFace`, ...). You must name the right type for the library. |
| `as_ptr()` | The raw library base. |
| `Interface::as_ptr()` | `*mut I`, for calls through the vtable. |
| `library::open_resource(name)` | `OpenResource`: `Some(pointer)` or `None`. Resources are never closed. |

`Library` is `Send` and `Sync`. Interface calls are raw: with an
`Interface`, you call `((*i).Function)(i, args...)` inside `unsafe`, as
the C SDK does. The `sys` wrapper functions do **not** use your
`Interface`; they use the globals (see below).

Most of the time you do not need this module: the crate's safe modules
open what they use.

---

## How safe modules get their interfaces

A safe module such as `utility`, `pci` or `expansion` needs an interface
other than exec's. It cannot rely on the program's global being set: a
`#![no_std]` driver may never set `IUtility`, for example. So each safe
module asks an internal helper (the crate-private `iface` module) for its
interface:

1. If the program's global (for example `IPCI`) is set, and is at least
   the interface version the module needs, that one is used.
2. Otherwise the crate opens the library and gets the interface itself,
   the first time it is needed, and keeps it for the rest of the program.
   It is never closed, so it can never be dropped while a call is using
   it. (On a ROM library that costs one open count.)

Device-based interfaces such as timer.device's `ITimer` work the same
way: the program's global if set, else the device opened once by the
crate.

What this means for you:

- In a std program, and in most drivers, you do not set any interface
  global for the safe modules. They just work.
- `IExec` must always be set (clib4's start-up code does it in a std
  program; the C glue does it in a no-C-runtime build). The same goes for
  `IDOS`: clib4 defines it, and in a no-C-runtime build your glue must
  define it (NULL is fine if the code never calls DOS through `sys`).
  Every other global the crate reads has a
  **weak** NULL definition in the crate, so your program links even if it
  never defines it. A strong definition (clib4's, or your glue's)
  replaces it.
- If a safe call returns `Err(NullPointer)` or `Err(Unsupported(..))`, the
  library could not be opened or is too old.

---

## `sys`: the raw bindings

`amigaos4::sys` is the crate's raw layer, generated from the AmigaOS 4.1
SDK headers (what a separate `-sys` crate would hold). It has:

- **Types and constants**: `TagItem`, `Task`, `MsgPort`, `IORequest`,
  `MEMF_*`, `SIGBREAKF_*`, `ASOT_*`, ... (`sys::types`, re-exported at
  `sys::`).
- **Interface vtables**: `ExecIFace`, `DOSIFace`, `UtilityIFace`,
  `PCIIFace`, ..., one struct per SDK interface.
- **Interface globals**: `IExec`, `IDOS`, `IUtility`, ...
- **Wrapper functions**: one per interface method, named
  `<library>_<method in snake_case>`, which call through the global. For
  example `sys::exec_alloc_vec_tag_list(size, tags)` is
  `IExec->AllocVecTagList(size, tags)`, and `sys::dos_put_str(s)` is
  `IDOS->PutStr(s)`.
- **C glue** (`sys::glue`) for the few varargs-only methods with no tag-list
  partner (`amiga_debug_*` for `DebugPrintF`, `amiga_do_method_*` for
  BOOPSI).

Exec is always built. Every other interface sits behind a Cargo feature
of the same name (`dos`, `utility`, `timer`, `pci`, `expansion`, `graphics`,
`intuition`, ... `full-sdk` turns them all on).

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", default-features = false, features = ["dos"] }
```

```rust
use amigaos4::sys;

fn free_shared() -> u32 {
    // SAFETY: IExec is set by the start-up code; AvailMem only reads.
    unsafe { sys::exec_avail_mem(sys::MEMF_SHARED) }
}
```

### The unsafe contract

Every `sys` wrapper is `unsafe`, with one contract stated once for all of
them:

- the interface global it calls through (`IExec`, `IDOS`, ...) is set to an
  interface obtained with `GetInterface` and not yet dropped;
- every argument meets the SDK autodoc of the function (pointers valid for
  what it reads or writes, strings NUL-terminated, objects of the right
  type, tag lists ending in `TAG_DONE`);
- the call is allowed from the calling context (task, process or
  interrupt) that the autodoc names.

### When to use `sys`

- When a safe module does not cover the call you need.
- Mind the globals: a `sys` wrapper for a non-exec library needs **its**
  global set. The crate's weak definitions are NULL, so calling, say,
  `sys::utility_...` with `IUtility` never set dereferences NULL. Either
  set the global yourself (in a no-C-runtime build, in your glue), or call
  through an `Interface` from `library` (`((*i).Method)(i, ...)`).
- clib4's start-up code defines `IExec`, `IDOS`, `IUtility`, `IElf` and
  `IClib4` in a std program; others are yours to set if you use their
  `sys` wrappers.
- Prefer the safe modules where they exist: they meet the contract for
  you and argue it in a `// SAFETY:` comment on each block.

---

## `expansion`: what machine is this?

Feature `expansion`. `expansion::machine_info()` calls expansion.library's
`GetMachineInfo` and returns a `MachineInfo`:

| Field / method | Meaning |
|---|---|
| `machine` | `MACHINETYPE_*` (in `sys`: `MACHINETYPE_AMIGAONE`, `MACHINETYPE_PEGASOS2`, `MACHINETYPE_SAM460`, `MACHINETYPE_X5000_20`, ...). |
| `name` | The machine's name. |
| `memory` | Memory available to AmigaOS, in bytes. |
| `total_physical` | All physical memory (`Option<u64>`; `None` on older versions). |
| `extensions` | Space-separated feature words such as `"bus.pci"`. |
| `has_extension(word)` | Whether `word` is one of them. |
| `tags_understood` | How many tags the call understood. |

```toml
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["expansion"] }
```

```rust
fn machine() -> amigaos4::Result<()> {
    let m = amigaos4::expansion::machine_info()?;
    amigaos4::serial_println!("{} with {} MB", m.name, m.memory / (1024 * 1024));
    if m.has_extension("bus.pci") {
        // there is a PCI bus
    }
    Ok(())
}
```

The rest of expansion.library (Zorro autoconfiguration, DOS mount nodes)
is system-only and left in `sys`. PCI devices are in the `pci` module
(see [Writing device drivers](../devices/writing-devices.md#pci)).

---

## `arch`: PowerPC helpers

`arch` holds a few PowerPC instructions as Rust functions. It is only
compiled for the AmigaOS target (`target_os = "amigaos"`), so it does not
appear in a host `cargo doc` and code using it does not build on the host.

### Barriers and the timebase (safe)

| Function | Instruction | Use |
|---|---|---|
| `sync_barrier()` | `sync` | Full memory barrier. |
| `lwsync_barrier()` | `lwsync` | Lightweight: orders load-store and store-store, not load-load. |
| `eieio_barrier()` | `eieio` | Keeps I/O accesses in order. |
| `mbar_barrier()` | `mbar` | Barrier after MMIO writes. |
| `timebase()` | `mfspr 268/269` | The 64-bit timebase counter. Its frequency is `sysinfo::cpu().timebase_speed`. Safe in interrupts. |
| `clear_reservation()` | `stwcx.` | Clears a leftover `lwarx` reservation. Call it at the exit of an interrupt server that uses atomics. |

```rust
fn time_it() -> u64 {
    let hz = amigaos4::sysinfo::cpu().timebase_speed.max(1);
    let t0 = amigaos4::arch::timebase();
    // ... work ...
    let ticks = amigaos4::arch::timebase() - t0;
    ticks * 1_000_000 / hz // microseconds
}
```

`timebase()` reads the SPRs rather than using `mftb`, because `mftb` is
not implemented on every core AmigaOS 4 runs on.

### Cache maintenance (unsafe)

| Function | Instruction | Effect |
|---|---|---|
| `cache_flush(addr, len)` | `dcbst` | Write dirty cache lines to RAM; they stay cached. |
| `cache_invalidate(addr, len)` | `dcbf` | Write back and invalidate, so the next read comes from RAM. |

Both are user-mode safe (the supervisor-only `dcbi` is never used), and
end with a `sync`.

Caveats:

- They step through memory by `arch::CACHE_LINE`, a fixed 32 bytes. The
  real line size of the running CPU is in `sysinfo::cpu().cache_line`.
- **Do not use these for device DMA.** DMA must go through
  `mem::DmaBuffer` / `mem::DmaMapping` (exec's `StartDMA`/`EndDMA`), which
  does the cache work correctly for the machine and gives physical
  addresses. Raw cache operations plus a CPU address are wrong on some
  machines.

### Byte-reversed MMIO (unsafe)

PCI registers are little-endian; the PowerPC CPU is big-endian. These
functions read and write a register at a CPU address with byte reversal
(`lhbrx`/`lwbrx`, `sthbrx`/`stwbrx`), so you get native values. Writes are
followed by `mbar`.

| Read | Write | Width |
|---|---|---|
| `mmio_r8(addr)` | `mmio_w8(addr, v)` | 8 bits (no reversal needed) |
| `mmio_r16(addr)` | `mmio_w16(addr, v)` | 16 bits, 2-byte aligned |
| `mmio_r32(addr)` | `mmio_w32(addr, v)` | 32 bits, 4-byte aligned |

Caveat from the source: the CPU cannot reach memory BARs behind every PCI
bridge. It works on the Pegasos II; on the AmigaOne (Articia S bridge) a
load faults, and I/O-port access through expansion.library must be used
instead. Prefer `pci::IoRegion`, or `pci::PciLock::mmio` after checking
the device answers; both are covered in
[Writing device drivers](../devices/writing-devices.md#pci).
