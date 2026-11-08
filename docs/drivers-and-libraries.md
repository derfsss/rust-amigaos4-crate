# Drivers, libraries and devices (no C runtime)

Some AmigaOS code must not use a C library: shared libraries (`.library`),
exec devices (`.device`), handlers, resources and Kickstart modules. They
are loaded by the OS, called from other programs' tasks, and may run before
DOS is up. The crate supports building this kind of code in Rust with no
C runtime at all: no clib4, no `std`, only `core`, `alloc` and the crate.

You can also build ordinary Shell programs this way. Several of the
crate's own test programs and `apps/imgview` are built like this.

| Kind | Start from | Reference |
|---|---|---|
| Handler, driver or Shell program with its own `_start` | `templates/driver` | `examples/hello-driver` |
| Shared library with a Resident and interface vectors | `templates/library` | `examples/hello-library` |
| Exec device with BeginIO, AbortIO and quick I/O | copy `examples/ram-device` | `examples/ram-device` |
| Kickstart module | copy `testprogs/kick-test` | `testprogs/kick-test` |

## How such a package is built

rustc does not link the final file. The package is a **static library**
(`staticlib`); a **Makefile** then links it with a small C file (the
"glue") and the crate's shared C files, using
`ppc-amigaos-gcc -nostartfiles -nodefaultlibs` and `-lgcc`, so clib4 never
enters the binary.

`tools/build-package.sh` does both steps: `cargo +amigaos build --release`,
then `make` inside the cross-compiler Docker image:

```bash
tools/build-package.sh examples/ram-device
# -> examples/ram-device/ram.device
```

`MAKE_ARGS="NAME=value"` passes variables through to make.

### Cargo.toml

```toml
[package]
name = "my-handler"
version = "0.1.0"
edition = "2021"
autobins = false

[lib]
name = "my_handler"
crate-type = ["staticlib"]
path = "src/main.rs"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
opt-level = "z"
lto = true

[dependencies]
amigaos4 = { path = "../..", default-features = false, features = ["dos"] }
```

- `default-features = false` turns off `std`. Then add only the features
  you use (see [features.md](features.md)).
- `panic = "abort"`: there is no unwinder. Panics go to your panic handler.

### The Rust side

Every no-C-runtime package starts the same way:

```rust
#![no_std]
#![no_main]

extern crate alloc; // Vec, String, Box from the global allocator

use core::panic::PanicInfo;
use amigaos4::allocator::SharedExecAllocator;

#[global_allocator]
// MEMF_SHARED: BeginIO, Open and Close run in the callers' tasks.
static ALLOCATOR: SharedExecAllocator = SharedExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

/// Called from the C glue once IExec is set.
#[no_mangle]
pub extern "C" fn rust_handler_main(_sysbase: *mut u8) -> i32 {
    amigaos4::serial_println!("Hello from a Rust driver!");
    0
}
```

The crate never sets a global allocator or a panic handler itself. You
choose them.

## Choosing the allocator

The global allocator gives `Vec`, `String` and `Box` their memory. Both
crate allocators call exec's `AllocVecTagList` and `FreeVec`; they differ
only in the memory type:

| Allocator | Memory | Use for |
|---|---|---|
| `allocator::ExecAllocator` | `MEMF_PRIVATE`: visible only to the allocating task | A Shell program whose heap only its own task uses |
| `allocator::SharedExecAllocator` | `MEMF_SHARED`: visible to all tasks | Devices, libraries, handlers; programs whose child tasks or processes share heap data |

A library's functions and a device's `BeginIO`, `Open` and `Close` run in
the **caller's** task and may allocate there, so a device's or library's
heap must not be one task's private memory. A device built with
`MEMF_PRIVATE` has crashed DOS when a caller's process exited. When in
doubt, use `SharedExecAllocator`.

The same applies to closures passed to `task::spawn` and
`dos::process::spawn`: what they capture lives wherever it was allocated,
so with `ExecAllocator` the child should not use it.

`IExec` must be set before the first allocation. The glue sets it before
calling any Rust code.

For DMA buffers use `mem::DmaBuffer`, which is always shared memory,
rather than the global allocator.

## The panic handler

`panic::default_panic_handler(info)` prints the panic message, the task
and a backtrace to the serial debug output (and to `T:<name>.crash` in a
DOS process), releases what the task had registered with the crate's
resource tracker, and returns to the task's **catch point**. A task with
no catch point (a device's unit task, for example) then sleeps for good
instead of spinning. See [debugging.md](debugging.md).

`panic::print_and_sleep(info)` is the simpler handler: print the message,
then sleep. Use it for code that must not touch the tracker.

## The C glue

Some things are easier in C, and some must be C:

- the entry point (`_start`) and setting `IExec` before any Rust runs;
- the **Resident** structure (the "ROM tag" AmigaOS scans for to find a
  library or device) and the interface vector tables;
- defining the interface globals the crate reads.

Each package has its own glue file in `src/`:

| Package | Glue | What it provides |
|---|---|---|
| `templates/driver` | `driver_glue.c` | `_start`, sets `IExec`, calls `rust_handler_main` |
| `templates/library` | `library_glue.c` | Resident, manager interface (Open, Close, Expunge), main vector table calling `rust_lib_*` |
| `examples/ram-device` | `device_glue.c` | Resident, device manager interface; calls `rust_dev_init`, `rust_dev_open`, `rust_dev_close`, `rust_dev_begin_io`, `rust_dev_abort_io` |
| `testprogs/exectest` (a Shell program) | `driver_glue.c` | `_start`, opens dos.library and utility.library, sets `IDOS` and `IUtility`, calls `rust_main(args, arglen)` |

Two shared files in the repository's `glue/` folder are linked into every
package:

- `amiga_glue.c`: C wrappers for AmigaOS functions that only exist in a
  variable-argument form (BOOPSI method calls such as `IDoMethod`).
- `freestanding_mem.c`: `memcpy`, `memmove`, `memset`, `memcmp`, `bcmp`
  and `strlen`. Rust's `core` calls them, and with no C library nothing
  else defines them. They are weak, so another definition in the link wins.

### Interface globals in the glue

The crate reads interface globals such as `IExec`, `IDOS` and
`IIntuition`. Most have a weak NULL definition in the crate, and a NULL
global makes the safe module open the library itself. **`IExec` and `IDOS`
have no weak definition** on purpose, so that a start-up that forgot them
fails to link instead of running with NULL:

- the glue must always define and set `IExec`;
- if your Rust code uses the `dos` module, the glue must also define
  `IDOS`, either set (as exectest's glue does) or NULL (the module then
  opens dos.library itself).

```c
struct ExecIFace *IExec = NULL;
struct DOSIFace  *IDOS  = NULL;
```

### The Makefile

The templates' Makefiles compile the glue and link:

```make
CFLAGS  = -O2 -Wall -D__AMIGAOS4__ -U__USE_INLINE__ -I$(SDK_INC) \
          -fno-tree-loop-distribute-patterns -ffunction-sections -fdata-sections
LDFLAGS = -nostartfiles -nodefaultlibs -Wl,-z,noexecstack -Wl,--gc-sections
LIBS    = -lgcc
```

`freestanding_mem.c` is compiled with `-ffreestanding -fno-builtin` as
well, so GCC does not turn its loops back into calls to themselves.

A library's or device's Makefile adds `-Wl,--undefined=RomTag`. Nothing
in the program refers to the Resident (AmigaOS finds it by scanning the
file), so without this `--gc-sections` would remove it and the OS would
not recognise the file.

## Shell programs without a C runtime

A Shell program can be built like a driver: the glue's `_start` receives
the argument string, opens what it needs, and calls a Rust function. Wrap
the body in `track::main`, which is the program's catch point: a panic
returns there with `RETURN_FAIL` (20), and anything the program leaked is
reported and released when the body returns.

```rust
#[no_mangle]
pub extern "C" fn rust_main(args: *const u8, arglen: i32) -> i32 {
    // SAFETY: the Shell's argument string, `arglen` bytes.
    let text = unsafe { core::slice::from_raw_parts(args, arglen.max(0) as usize) };
    let mode = core::str::from_utf8(text).unwrap_or("").trim();
    amigaos4::track::main(|| {
        amigaos4::serial_println!("arguments: {}", mode);
        0 // the return code
    })
}
```

`testprogs/exectest`, `testprogs/libtest`, `testprogs/panictest` and
`apps/imgview` are built this way. For `dos::args::Args` the program needs
`IDOS` set by the glue.

A program that uses Vulkan must be a `std` program instead; see
[vulkan.md](vulkan.md).

## Shared libraries

`templates/library` makes `PROJNAME.library`. Rename `PROJNAME` in
`Cargo.toml` and the Makefile. The Rust side provides lifecycle callbacks
and the library's functions:

```rust
/// Called once when the library is first loaded. Return 0 for success.
#[no_mangle]
pub extern "C" fn rust_lib_init() -> i32 { 0 }

/// Called each time a program opens the library. Return 0 for success.
#[no_mangle]
pub extern "C" fn rust_lib_open() -> i32 { 0 }

/// Called each time a program closes the library.
#[no_mangle]
pub extern "C" fn rust_lib_close() {}

/// A library function.
#[no_mangle]
pub extern "C" fn rust_lib_add(a: u32, b: u32) -> u32 { a + b }
```

To add a function:

1. Write it in Rust as `#[no_mangle] pub extern "C" fn rust_lib_xxx(...)`.
2. Declare it in `library_glue.c`: `extern uint32 rust_lib_xxx(...);`.
3. Add `(void *)rust_lib_xxx` to `mainVectors[]` in `library_glue.c`.

Programs call it through the library's `"main"` interface; the order in
`mainVectors[]` is the interface's function order. Remember that every
function runs in the caller's task, so any shared state needs a
`sync::Mutex` or `sync::Semaphore` (the crate never uses Forbid).

## Devices

`examples/ram-device` is a complete exec device: one unit backed by a
64 KB buffer, with `CMD_READ`, `CMD_WRITE`, `CMD_CLEAR` and `CMD_RESET`,
bounds checks, quick I/O and `io_Error`/`io_Actual` reporting. The C glue
provides the Resident and the device manager interface; every I/O decision
is made in Rust.

For larger devices the crate's `device` feature has building blocks: an
`IoReq` view of a request, a `RequestQueue` for a unit, and a `Worker` for
the unit's task. These, unit tasks, quick I/O, `AbortIO` and SANA-II
network devices are covered in
[devices/writing-devices.md](devices/writing-devices.md). Using a device
from a program is in [devices/using-devices.md](devices/using-devices.md).

## Unloading cleanly (Expunge)

A library or device loaded from disk must unload cleanly. AmigaOS asks it
to with **Expunge**. If it is still open, Expunge only sets the
`LIBF_DELEXP` flag ("expunge later"). The last Close must then perform the
expunge **by calling Expunge itself**, which removes the node from exec's
list before handing back the segment list (the loaded code). Returning the
segment list from Close alone leaves exec's list pointing into freed code.
The library template's glue does this.

`testprogs/expunge` tests it on the Amiga:

```
expunge DEVICE <name> [unit]
expunge LIBRARY <name>
```

It opens the device or library, calls `RemDevice` or `RemLibrary` (which
defers, as it is open) and closes it. Afterwards the device or library
must be gone from exec's list and load afresh when opened again.

A library or device that starts a server process
(`dos::process::ProcessBuilder::spawn_detached`) must wait until that
process has gone (`FindTask(name)` returns NULL) before it lets Expunge
unload its code.

## Kickstart modules

A Kickstart module is loaded with the system, from the Kickstart
configuration, before or alongside DOS. `testprogs/kick-test` builds
`rusttest.device`, one file with two Residents:

- a **COLDSTART** auto-init device (`RTF_NATIVE|RTF_COLDSTART|RTF_AUTOINIT`,
  priority 0), created by exec during the cold start. Its init reports on
  the serial output and checks the Rust heap;
- an **AFTERDOS** tag (`RTF_NATIVE|RTF_AFTERDOS`, priority -100), run once
  DOS is up. It opens `rusttest.device` by name and reads unit 0 with
  `DoIO`.

To try it, put the file in the Kickstart zip as `Kickstart/rusttest.device`
and add a line inside the configuration in `Kickstart/Kicklayout` (before
the blank line that ends it):

```
MODULE Kickstart/rusttest.device
```

The serial output ends with `rusttest.device: kickstart load test PASSED`.

Its Makefile keeps the Residents as garbage-collection roots, for the same
reason as `--undefined=RomTag` above. A module like this is never
unloaded, and its statics live for the whole session. Kickstart code uses
`SharedExecAllocator`.

## Checklist

- [ ] `#![no_std]`, `#![no_main]`, `extern crate alloc`.
- [ ] `default-features = false` and only the features you need.
- [ ] `panic = "abort"` in both profiles.
- [ ] `#[global_allocator]`: `SharedExecAllocator` for anything called by
      other tasks.
- [ ] `#[panic_handler]` calling `panic::default_panic_handler`.
- [ ] The glue defines and sets `IExec`, and defines `IDOS` if you use `dos`.
- [ ] Libraries and devices: `--undefined=RomTag`, and Close runs a deferred
      Expunge. Test with `testprogs/expunge`.
- [ ] Debug output with `serial_println!` (see [debugging.md](debugging.md)).
