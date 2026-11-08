# Debugging

AmigaOS has no debugger attached by default, and drivers and libraries have
no console to print to. The usual tool is the **debug output**: a text
channel the kernel writes to the serial port (or keeps in memory). This
page covers writing to it, reading it, what happens when Rust code panics,
and the `debug` module for symbols and backtraces.

## Writing debug output

`serial_println!` and `serial_print!` work like `println!` and `print!`,
but write to the debug output through exec's `DebugPrintF`. They work
in `std` programs, drivers, libraries, devices and Kickstart modules. No feature and no C glue are needed.

```rust
amigaos4::serial_println!("Hello from Rust on AmigaOS 4!");
amigaos4::serial_println!("Vec has {} elements", v.len());
amigaos4::serial_print!("no newline: ");
amigaos4::serial_println!("x = {:08x}", 0xdead_beefu32);
```

For your own `core::fmt::Write` code, `amigaos4::fmt::serial()` returns a
`fmt::Serial` writer:

```rust
use core::fmt::Write;

let _ = writeln!(amigaos4::fmt::serial(), "state = {:?}", state);
```

Output goes out in pieces of up to 127 bytes, so lines from different
tasks may interleave.

Use `serial_println!` rather than the older `debug_print!` and
`debug_println!` macros, which take NUL-terminated byte strings and need
the C glue.

In a `std` program, `println!` writes to the Shell window instead, and is
usually more convenient. `serial_println!` is still useful there for
messages you want to see even when the program's window has gone.

## Reading the debug output

Where the output goes depends on how the machine is set up:

- **Serial port.** Connect a null-modem cable to another computer and
  read it with a terminal program. Real hardware usually sends debug
  output here.
- **QEMU.** Start QEMU with its serial port sent somewhere you can read:
  `-serial stdio`, `-serial file:serial.log`, or a TCP socket.
- **Memory buffer.** When the output is kept in memory, the
  `DumpDebugBuffer` command in a Shell prints what is in the buffer.

The crate's own test programs (`testprogs/test-driver`, `exectest`,
`libtest`) write their results to the debug output as well.

`debug::Debug::watch_output` (below) lets a program be woken whenever
something is written to the debug output.

## What happens on a panic

AmigaOS does not clean up after a task: when it ends, files stay open,
locks stay held, windows stay on the screen. In Rust each wrapper releases
its resource when dropped, but with `panic = "abort"` (always the case
without a C runtime) no destructor runs on a panic. The crate's `track`
module closes that gap.

### The resource registry

The crate's wrappers for long-lived OS resources register each one with
the task that made it, and with the source line that asked for it. This
covers files, locks, signal bits, message ports, public screen locks,
windows, ReAction windows, AppWindows, AppIcons, Tools menu items,
libraries, interfaces, devices, interrupt servers, reset callbacks,
low-memory handlers, DOS and variable notifications, seglists, shared
objects, bitmaps, fonts, datatypes objects, locales, catalogs and
bsdsocket bases. Dropping the wrapper removes the registration. The
registry is a fixed table, so registering never allocates.

`track::count(task)` tells you how many resources a task has registered;
`track::untracked()` how many could not be registered because the table
was full.

### The panic path

`panic::default_panic_handler` (which you set as `#[panic_handler]` in
`no_std` code) calls `track::on_panic`, which:

1. prints a report to the debug output and, in a DOS process (with the
   `dos` feature), to the file `T:<command>.crash`;
2. releases everything the panicking task still has registered, newest
   first, using the raw OS release calls (not `Drop`, since the panicking
   code may have left its own state half-changed);
3. returns to the task's **catch point**.

A report looks like this:

```
PANIC at src/main.rs:63: panictest: deliberate panic at depth 3
task 5f2a1c40 "panictest"; backtrace:
  7f8a1234 panictest+1234 level3
  7f8a1300 panictest+1300 level2
  ...
track: releasing dos lock made at src/main.rs:81
track: releasing dos file made at src/main.rs:80
5 resource(s) released
returning to the catch point (RETURN_FAIL)
```

(The addresses and offsets here are only illustrations.) Symbol names
appear in the backtrace only with the `debug` feature and only in a DOS
process; in a plain exec task the backtrace is raw addresses.

The crash file is named after the command (a Shell command's file part,
with spaces turned into `_`), or after the task.

### Catch points

A catch point is where execution continues after a panic:

| Catch point | After a panic |
|---|---|
| `track::main(f)` around a program's body | returns `RETURN_FAIL` (20) to the Shell |
| `task::spawn` / `task::Builder::spawn` | the child ends; `JoinHandle::join` returns an error |
| `dos::process::spawn` / `ProcessBuilder::spawn` | the child ends; `Child::wait` returns an error |
| `track::catch(f)` | returns `Err(track::Panicked)` |
| none (a device's unit task, an interrupt) | the task sleeps for good (`Wait(0)`) instead of spinning |

Two special cases:

- If the main task panics while child tasks the crate started are still
  running, nothing is released (the children may be using it) and the
  main task sleeps instead of returning to the Shell, because the
  children are running in the program's code.
- When a child ends, whatever it still has registered passes to its
  parent, except its own signal bits and ports, which end with it.

### Leak reports

When `track::main`'s closure returns, anything the main task still has
registered was leaked (for example with `mem::forget` or by storing it in
a static). Each one is reported with its source line, and released:

```
track: releasing dos lock made at src/main.rs:128
track: 1 leaked resource(s) released at the end of the program
```

A wrapper that hands its OS handle on deliberately should give it up with
its `into_raw` method (where it has one, such as `graphics::BitMap`), not
with `mem::forget`.

### A no-C-runtime program

```rust
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

#[no_mangle]
pub extern "C" fn rust_main(_args: *const u8, _arglen: i32) -> i32 {
    // The body runs under a catch point: a panic gives RETURN_FAIL.
    amigaos4::track::main(|| {
        // ...
        0
    })
}
```

`testprogs/panictest` shows this in full: it holds a file, a lock, a
window, a signal and a port, panics three calls deep, and checks
afterwards that everything was released.

### A std program

In a `std` program, panics go through Rust's standard library, not the
crate's handler. Install the crate's hook at the start of `main`:

```rust
fn main() {
    amigaos4::track::install_std_hook();
    // ...
}
```

The hook writes the same report (debug output and `T:<command>.crash`).
Then:

- with unwinding (the default profile), destructors release what the
  program held, and the hook only reports;
- with `panic = "abort"`, the hook releases what the task has registered
  before std aborts.

The previous hook (std's message on stderr) runs afterwards.
`testprogs/panicstd` is a small example. `track::main` is not used in
`std` programs.

## The `debug` module

With the `debug` feature, `debug::Debug` wraps exec's debug interface. It
opens the interface itself, so the program needs no `IDebug` global.

```toml
[dependencies]
amigaos4 = { path = "../..", features = ["debug"] }
```

| Method | What it does |
|---|---|
| `Debug::open()` | opens exec's `"debug"` interface |
| `symbol(addr)` | what exec knows about a code address: a `Symbol` with `module`, `segment`, `segment_offset`, `source_file`, `source_line`, `function` (copied out) |
| `backtrace(max)` | the calling task's return addresses, innermost first |
| `watch_output(&signal)` | sets `signal` whenever something is written to the debug output; ends when the returned value drops |

```rust
use amigaos4::debug::Debug;

fn print_backtrace() -> amigaos4::Result<()> {
    let d = Debug::open()?;
    for addr in d.backtrace(32) {
        match d.symbol(addr) {
            Some(s) => amigaos4::serial_println!(
                "{:08x} {}+{:x} {}",
                addr,
                s.module,
                s.segment_offset,
                s.function.as_deref().unwrap_or("")
            ),
            None => amigaos4::serial_println!("{:08x}", addr),
        }
    }
    Ok(())
}
```

Things to know:

- `backtrace` traces from where the task last waited or was switched out,
  not from the call itself, so the first frames are exec's and DOS's and
  the program's own frames come after them.
- Looking up symbols (`ObtainDebugSymbol`) is only reliable in a DOS
  process. In a plain exec task it has been seen never to return, which is
  why the panic report prints raw addresses there.
- `watch_output` needs a `signal::Signal` of the calling task. Wait for it
  with `signal::wait(sig.mask() | ...)`.

```rust
use amigaos4::debug::Debug;
use amigaos4::signal::{self, Cancel, Signal};

fn wait_for_debug_output() -> amigaos4::Result<()> {
    let d = Debug::open()?;
    let sig = Signal::alloc()?;
    let _watch = d.watch_output(&sig)?; // ended when `_watch` drops
    // Sleep until something writes to the debug output, or Ctrl-C.
    let _got = signal::wait(sig.mask() | Cancel::MASK);
    Ok(())
}
```

## Tips

- A `std` program's release profile strips debug information
  (`strip = "debuginfo"`) but keeps symbols, so function names can still
  appear in backtraces. The no-C-runtime Makefiles run `ppc-amigaos-strip`
  on the result, so expect module offsets rather than names there.
  `testprogs/libtest`'s Makefile keeps an unstripped copy
  (`libtest.debug`) beside the stripped file for looking offsets up.
- Print the return code and any `AmigaError` you get; most crate calls
  return a specific error instead of crashing.
- Check [troubleshooting.md](troubleshooting.md) for crashes with known
  causes.
