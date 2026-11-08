# Troubleshooting

Known pitfalls, with their causes and fixes. Each entry comes from the
crate's sources, its README or its test programs. For first-build errors,
see also [getting-started.md](getting-started.md#common-first-errors).

## Building and linking

### The build cannot find the cross-compiler image

`FAIL: docker cannot see walkero/amigagccondocker:os4-gcc15-3.6.1`

The image must be in the Docker engine your Linux shell uses. Under WSL
with its own Docker engine, images pulled in Docker Desktop are not
visible. Run `docker pull walkero/amigagccondocker:os4-gcc15-3.6.1` in the
WSL shell.

### The build breaks after moving a project

The templates use relative paths: the crate is `path = "../.."`, the
linker is `../../tools/ppc-amigaos-gcc` in `.cargo/config.toml`, and the
Makefiles use `../../glue/...`. Keep the project two folders below the
repository root (for example `apps/my-program`), or change those paths.
`tools/build-package.sh` mounts the repository into Docker for `make`, so
no-C-runtime packages must stay inside the repository.

### Link error: undefined `IExec` or `IDOS` (no C runtime)

The crate reads the interface globals but does not define `IExec` or
`IDOS`. The package's C glue must define them, and set `IExec` before any
Rust code runs. `IDOS` may be left NULL; the `dos` module then opens
dos.library itself. See
[drivers-and-libraries.md](drivers-and-libraries.md#interface-globals-in-the-glue).

### Link error: undefined `memcpy`, `memset`, `strlen` (no C runtime)

The target's `compiler_builtins` exports no memory functions, and there is
no C library. Link `glue/freestanding_mem.c` (the templates' Makefiles do).
Its functions are weak, so another definition elsewhere wins.

### Do not call `DebugPrintF` as a Rust variadic function

exec's `DebugPrintF` uses the AmigaOS "linear varargs" convention: the
arguments go on the stack, not in registers. A plain Rust C-variadic call
puts them in registers, and `DebugPrintF` reads garbage (it crashed a
`std` program). Use `serial_println!`, which calls it the right way and
needs no C glue.

## Programs that will not start

### The file will not load: "not executable", IoErr 305, no output

The release profile has `strip = true` or `strip = "symbols"`. Those pass
`--strip-all` to the linker, and AmigaOS refuses the file. Use:

```toml
[profile.release]
strip = "debuginfo"
```

### A std program crashes as it starts or exits

clib4.library on the Amiga is too old. With 2.1 and 2.4 (the one in the
cross-compiler image's SDK) a program built here crashes in clib4's
start-up or exit code (`clib4_start` → `CloseLibraryInterface` →
`__pthread_exit`, a DSI). Install clib4.library 2.5 or newer in `LIBS:`.

### The program crashes deep inside a library call

Check the stack. Set it with a `$STACK:` cookie (see
[programs.md](programs.md#stack-and-version-cookies)). Some libraries run
large code on the caller's stack: the Vulkan loader and driver do, and
`apps/vkinfo` asks for 1 MB.

## Memory

### A device, library or handler crashes callers, or DOS

Its heap is `MEMF_PRIVATE` ("only visible to the allocator task"). A
library's functions and a device's `BeginIO`, `Open` and `Close` run in
the caller's task and may allocate there. A device built with
`MEMF_PRIVATE` crashed DOS when a caller's process exited. Use
`allocator::SharedExecAllocator` in anything other tasks call into.

### A child task corrupts data its closure captured

The crate's task and process control blocks are always shared memory, but
what a closure captures lives wherever it was allocated. With
`ExecAllocator` (private memory) the child should not use it. Programs
whose tasks share heap data use `SharedExecAllocator`.

### Named memory has no `try_lock`

exec's `AttemptNamedMemory`, documented as never blocking, spins in the
kernel while another task holds the block. `named::lock` waits properly
instead, and there is no non-blocking form.

## Tasks, processes and signals

### Starting a child crashes DOS (if you call CreateNewProc yourself)

dos.library links a new process into its list only after it has been
started. A child that runs and ends in that gap crashes DOS; exec can
likewise run a new higher-priority task before `CreateTask` returns. The
crate's `task::spawn` and `dos::process::spawn` use a start-up handshake
(the child waits for CTRL-F, which the parent sends once the create call
has returned), so no child runs before the parent holds it. If you create
tasks or processes with the raw `sys` calls, do the same.

### `WrongTask` from a `dos` call

An exec task may not call dos.library, nor anything that may load a
library or device from disk. The crate's `dos` calls return
`AmigaError::WrongTask` in an exec task. Use `dos::process::spawn` for a
child that needs files, or hand results back to the parent. In an exec
task, write diagnostics with `serial_println!`, not `std` I/O.

### A child's output disappears

A child process's default input and output are `NIL:`. Give it a stream
with `ProcessBuilder::input` and `ProcessBuilder::output`.

### The parent stops running while a child works

A busy child above its parent's priority starves the parent. A Shell job
may run below priority 0, so an absolute priority 0 can be above it. Set
children's priorities relative to `task::priority()`.

### `join` gives `WrongTask`, or a `JoinHandle` is "leaked"

The done signal belongs to the task that spawned the child, so only that
task can join it. `JoinHandle` is not `Send`. Dropping it on another task
leaks the control block (and prints
`amigaos4::task: JoinHandle dropped on the wrong task; leaked`).

### `join` or `wait` returns an error

The child panicked. Its closure gave no result. The panic report is on
the debug output; see [debugging.md](debugging.md).

### A `Signal` or socket cannot be moved to another task

Signal bits belong to the task that allocated them, so `signal::Signal`
is not `Send`. Signal another task through `SignalTarget` (unsafe: that
task must still exist and own the bit). Likewise `net` sockets belong to
the task that opened the library and are not `Send`. Signals must not be
allocated or freed from exception code, and `Wait` must never be called
from an interrupt.

### `WouldDeadlock` from a mutex or semaphore

The calling task already holds the lock. exec would grant a non-recursive
mutex again to its holder, so `sync::Mutex` tracks the owner and refuses.
The same applies to `sync::Semaphore`'s exclusive owner. `sync` locks are
for tasks only, never interrupts.

### A server process outlives its library or device

A library or device that starts a process with
`ProcessBuilder::spawn_detached` must wait until `FindTask(name)` returns
NULL before it lets its code be unloaded.

### A loop that restarts a timer uses 100% CPU

An aborted request's reply still sets its port's signal, and `WaitIO`
takes the message but leaves the signal. A loop that aborts its timer
whenever something else wakes it, then starts a new one, wakes again at
once, every time. `timer::AmigaTimer::abort` clears the signal for you;
do the same if you abort requests with raw calls.

## Libraries and devices

### A library or device crashes the machine after it is unloaded

Close returned the segment list itself after a deferred Expunge, leaving
exec's list pointing into freed code. When the last Close finds
`LIBF_DELEXP` set, it must call Expunge, which removes the node first.
Test with `testprogs/expunge` (`expunge DEVICE <name> [unit]`,
`expunge LIBRARY <name>`).

### Byte-reversed MMIO reads are wrong

The byte-reversed helpers in `arch` (`lwbrx`/`stwbrx`) work where PCI
memory is mapped directly, as on the Pegasos II, but not on the AmigaOne
(Articia S bridge).

### A PCI memory region is not reachable

`pci::MmioRegion` is `unsafe` because the CPU may not reach a memory BAR
behind every bridge. `PciDevice::lock` never waits: it returns `AmigaError::InUse`
while another driver holds the device.

## Debugging

### The panic report has no function names

Symbol names need the `debug` feature, and are looked up only in a DOS
process: `ObtainDebugSymbol` never returned when called from a panicking
plain exec task, so the report prints raw addresses there. Stripped
binaries also have no names (see [debugging.md](debugging.md#tips)).

### No `T:<name>.crash` file

The crash file needs the `dos` feature and a DOS process.

### `debug::Debug::backtrace` shows exec frames first

It traces from where the task last waited or was switched out, so exec's
and DOS's frames come before the program's own.

### The program does not return to the Shell after a panic

The main task panicked while child tasks the crate started were still
running. They run in the program's code, so the main task waits for good
and releases nothing. Join children before work that might panic, or let
them end first.

### "leaked resource(s) released at the end of the program"

Something registered with the crate's tracker was still held when
`track::main`'s closure returned, usually because of `mem::forget` or a
static. Each one is listed with the source line that made it.

## Graphics

### A drawing call returns `InvalidArgument`

A RastPort without layers does not clip, so the crate checks coordinates,
pens and buffer sizes first and refuses anything that would reach outside
the bitmap or window. Text must fit entirely: `RastPort::text` checks the
whole line's box.

## Networking

### `SocketLib::set_opt` refuses `SO_RCVTIMEO`, `SO_SNDTIMEO` or `SO_LINGER`

Those options take a `timeval` or a `struct linger`, and a 4-byte value is
wrong for them. Use the typed setters on `net::Socket`:
`set_read_timeout`, `set_write_timeout` and `set_linger`, each taking an
`Option<Duration>`. See
[networking.md](networking.md).

## Vulkan

### Vulkan finds no GPU device

- RADV gives a device only to a process that belongs to a program, which
  it tells by the process's C library context. A program built without a
  C runtime gets an instance and a surface but no device. Build Vulkan
  programs as `std` programs.
- The loader needs a driver: `VK_DRIVER_FILES` set (for example
  `SetEnv SAVE VK_DRIVER_FILES SYS:Mesa/radv_vk.json`). It is read only
  when `vulkan.library` first loads, so a change needs a reboot.

See [vulkan.md](vulkan.md).

## Shared objects

### Loading a `.so` crashes

`elf::SharedObject::open` is `unsafe`: the object must match the
program's C runtime. Shared objects are built for a particular C library,
and a program without one cannot satisfy them.
