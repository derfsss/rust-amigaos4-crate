# Writing programs

A program here means an ordinary Rust executable that you start from the
Shell or from Workbench. It has `fn main`, uses Rust's `std`, and runs on
**clib4**, the C library the AmigaLabs target is built on. `println!`,
`std::env::args`, `std::fs` and threads work as on any other platform. The
`amigaos4` crate adds the AmigaOS side: windows, requesters, AmigaDOS
argument parsing, Workbench and so on.

If you have not built anything yet, start with
[getting-started.md](getting-started.md).

## Project layout

Start from `templates/program`. A program package has three files:

```
my-program/
  Cargo.toml
  .cargo/config.toml
  src/main.rs
```

### Cargo.toml

```toml
[package]
name = "my-program"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "my-program"
path = "src/main.rs"

[profile.release]
opt-level = "s"
lto = true
# Not `true` or "symbols": the linker's --strip-all makes a file AmigaOS
# will not load (IoErr 305).
strip = "debuginfo"

[dependencies]
# std (the default) on clib4; add the interfaces the program uses.
amigaos4 = { path = "../..", features = ["intuition"] }
```

Points to note:

- **Keep the default features.** The default set is `std`, `dos` and
  `utility`. Adding features with `features = [...]` keeps the defaults.
  Never use `default-features = false` in a program: that is for code
  without a C runtime.
- **Add one feature per area you use**, for example `intuition` for
  windows, `graphics` for drawing, `asl` for file requesters. Some
  features pull in others (`asl` pulls in `dos` and `intuition`). The full
  list is in [features.md](features.md).
- **`strip = "debuginfo"`**, never `strip = true` or `strip = "symbols"`.
  Those pass `--strip-all` to the linker, and AmigaOS refuses to load the
  result (IoErr 305, with no other output).

### .cargo/config.toml

```toml
[build]
target = "powerpc-unknown-amigaos"

[target.powerpc-unknown-amigaos]
linker = "../../tools/ppc-amigaos-gcc"
```

rustc links the program itself with the target's linker, `ppc-amigaos-gcc`
with clib4. `tools/ppc-amigaos-gcc` runs your installed `ppc-amigaos-gcc`
if there is one, or else the one in the cross-compiler Docker image, with
the paths rustc passes mounted at the same places. No Makefile and no C
glue are needed. The `linker` path is relative to the project; change it
if the project moves.

### Building

```bash
tools/build-package.sh apps/my-program
# -> apps/my-program/target/powerpc-unknown-amigaos/release/my-program
```

## Interface globals

C programs on AmigaOS declare globals such as `IIntuition` and set them
after opening the library. In a Rust program you declare none of them.
The crate gives every interface global it reads a weak NULL definition.
clib4 sets its own (`IExec`, `IDOS`, `IUtility`, `IElf`) and these replace
the weak ones; for the rest, a NULL global makes the safe module open the
library itself the first time it is needed. So you turn on a feature and
call the API.

## Stack and version cookies

AmigaOS reads two special strings from an executable:

- `$STACK:` sets the stack size the Shell gives the program when it starts.
  The default stack is small for Rust code; 256 KB is a sensible start.
  Code that runs deep libraries (the Vulkan example uses 1 MB) needs more.
- `$VER:` is the version string the `Version` command and Workbench show.
  The usual format is `$VER: name version (day.month.year)`.

Put them in statics marked `#[used]` so the linker keeps them:

```rust
/// The stack AmigaOS gives the program when it starts.
#[used]
static STACK: [u8; 14] = *b"$STACK:262144\0";

/// The version string the Version command reads.
#[used]
static VERSION: [u8; 33] = *b"$VER: my-program 0.1 (5.11.2026)\0";
```

The array length must match the string exactly, including the final `\0`.
The compiler tells you if it does not.

## Arguments

`std::env::args()` works and gives the program name followed by the
arguments.

For Shell-style arguments, use `dos::args::Args`, which wraps AmigaDOS
**ReadArgs**. You describe the arguments with a **template** such as
`"FROM/A,TO/K,ALL/S,BUF/N"`, where each modifier gives an option's type:

| Modifier | Meaning | Read with |
|---|---|---|
| (none) | text, optional | `args.text("NAME")` → `Result<Option<&str>>` |
| `/A` | required | (combined with others) |
| `/K` | must be given with its keyword (`TO file`) | `text` |
| `/S` | switch, present or not | `args.switch("ALL")` → `Result<bool>` |
| `/T` | toggle (on/off) | `args.toggle("QUIET")` → `Result<Option<bool>>` |
| `/N` | number | `args.number("BUF")` → `Result<Option<i32>>` |
| `/M` | several values | `args.multi("FROM")` → `Result<Vec<&str>>` |

`Args::from_command_line(template)` parses the program's own command line;
`Args::parse(template, line)` parses any string. Reading an option at the
wrong type, or one not in the template, returns an error. When parsing
fails you get `AmigaError::DosError(code)`; `dos::fault(code)` turns the
code into the usual AmigaDOS message.

This is the template program's argument handling:

```rust
use amigaos4::dos::{self, args::Args};
use amigaos4::error::AmigaError;

const TEMPLATE: &str = "NAME,REQUESTER/S";

fn main() {
    let args = match Args::from_command_line(TEMPLATE) {
        Ok(a) => a,
        Err(AmigaError::DosError(c)) => {
            eprintln!("my-program: {} (template {})", dos::fault(c), TEMPLATE);
            std::process::exit(10);
        }
        Err(e) => {
            eprintln!("my-program: {}", e);
            std::process::exit(10);
        }
    };
    let name = args.text("NAME").ok().flatten().unwrap_or("Amiga").to_string();
    println!("Hello, {}.", name);
    if args.switch("REQUESTER").unwrap_or(false) {
        println!("a requester was asked for");
    }
}
```

The `Args` value frees its ReadArgs state when dropped, so copy out
(`to_string()`) anything you keep longer.

## Return codes

AmigaOS return codes are 0 (OK), 5 (WARN), 10 (ERROR) and 20 (FAIL). Use
`std::process::exit(code)` to return one. Shell scripts test them with
`If WARN`, `If ERROR` and so on.

## Errors

Most crate calls return `amigaos4::Result<T>`, which is
`Result<T, amigaos4::AmigaError>`. `AmigaError` implements `Display` and,
with the `std` feature, `std::error::Error`, so `?` works into
`Box<dyn std::error::Error>` as well. Common variants:

| Variant | Meaning |
|---|---|
| `DosError(code)` | an AmigaDOS error (IoErr); `dos::fault(code)` describes it |
| `NullPointer` | an OS call returned NULL (a library or object could not be made) |
| `WrongTask` | the call needs a DOS process, or the task that made the object |
| `Cancelled` | the user pressed Ctrl-C while the call waited |
| `WouldDeadlock` | a lock the calling task already holds |
| `InvalidArgument(text)` | the arguments cannot be passed to the OS as they are |

## A complete example: a window

This program opens a window on the default public screen (usually
Workbench), writes a line of text in it, and waits until the window is
closed or Q is pressed.

`Cargo.toml` dependencies:

```toml
[dependencies]
amigaos4 = { path = "../..", features = ["intuition", "graphics"] }
```

`src/main.rs`:

```rust
use amigaos4::graphics::Font;
use amigaos4::intuition::{idcmp, Event, PubScreen, Window, WindowOptions};

#[used]
static STACK: [u8; 14] = *b"$STACK:262144\0";
#[used]
static VERSION: [u8; 35] = *b"$VER: hello-window 0.1 (8.11.2026)\0";

fn main() {
    // Panic reports go to the serial output and T:hello-window.crash.
    amigaos4::track::install_std_hook();
    if let Err(e) = run() {
        eprintln!("hello-window: {}", e);
        std::process::exit(10);
    }
}

fn run() -> amigaos4::Result<()> {
    // Lock the default public screen; unlocked when `screen` drops.
    let screen = PubScreen::default_screen()?;

    // Open a window on it. IDCMP flags choose which events we receive.
    let mut win = Window::open(
        &screen,
        &WindowOptions {
            title: "Hello from Rust",
            left: 40,
            top: 40,
            inner_width: 320,
            inner_height: 60,
            idcmp: idcmp::CLOSEWINDOW | idcmp::VANILLAKEY,
            ..Default::default()
        },
    )?;

    // Draw a line of text inside the window's borders.
    let font = Font::open("topaz.font", 8)?;
    {
        let (x0, y0, _, _) = win.inner();
        let mut rp = win.rastport()?; // borrowed until the end of this block
        rp.set_font(&font);
        rp.set_a_pen(1);
        rp.text(x0 + 10, y0 + 10, b"Press Q or close the window.")?;
    }

    // Wait for events. Each IntuiMessage is copied and replied for us.
    loop {
        let (msg, _) = win.wait(0);
        match msg.map(|m| m.event) {
            Some(Event::CloseWindow) | Some(Event::Key(b'q')) => break,
            _ => {}
        }
    }
    Ok(())
    // Dropped in reverse order: the font, the window, then the screen lock.
}
```

What each part does:

- `PubScreen::default_screen()` locks the screen so it cannot close under
  the window.
- `Window::open` takes a `WindowOptions`; `..Default::default()` fills in
  the rest (close, drag and depth gadgets, activate).
- `win.rastport()` needs the `graphics` feature. Every drawing call checks
  its coordinates and returns an error rather than drawing outside the
  window.
- `win.wait(others)` sleeps until a window event or one of the signals in
  `others` arrives. Pass `Cancel::MASK` from `amigaos4::signal` to stop on
  Ctrl-C too.

## A second example: files

Both `std::fs` and the crate's `dos::fs` work in a program. `dos::fs`
gives you AmigaDOS-specific things (locks, protection bits, comments,
notification). This needs only the default features:

```rust
use amigaos4::dos::fs::{File, OpenMode, SeekFrom};

fn append_note() -> amigaos4::Result<()> {
    // Write a new file (an old one is replaced).
    File::write_new("RAM:notes.txt", b"first line\n")?;

    // Open it for reading and writing, and add to the end.
    let mut f = File::open("RAM:notes.txt", OpenMode::Update)?;
    f.seek(SeekFrom::End(0))?;
    f.write_all(b"second line\n")?;
    f.close()?; // or let it drop; close() reports Close's result

    let bytes = File::read_to_vec("RAM:notes.txt")?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}
```

`dos::fs::File` is unbuffered DOS I/O. Wrap it in your own buffering for
many small reads or writes. More is in [dos.md](dos.md).

## Panics in a program

With the crate's hook installed, a panic writes a report (message, task,
backtrace) to the serial debug output and to `T:<command>.crash`:

```rust
fn main() {
    amigaos4::track::install_std_hook();
    // ...
}
```

Add the `debug` feature to get symbol names in the backtrace. With the
default unwinding, destructors release what the program held; with
`panic = "abort"` in your profile, the hook releases the resources the
crate tracks before std aborts.

`track::main(f)` is the catch point for programs built **without** a C
runtime (as a driver is built), where the crate's own panic handler runs.
In a `std` program, panics go through std, so use `install_std_hook`
instead. Details are in [debugging.md](debugging.md).

## Programs started from Workbench

A program started by double-clicking its icon receives a Workbench
start-up message instead of a command line. `workbench::WbStartup::take()`
takes that message, gives you the tool and project icons as paths, and
replies it when dropped. The crate's example of this, `apps/imgview`, is
built without a C runtime; in a `std` program clib4's own start-up code
runs first. See [workbench.md](workbench.md).

## Where to look next

- [gui/intuition.md](gui/intuition.md), [gui/reaction.md](gui/reaction.md),
  [gui/menus.md](gui/menus.md), [gui/asl.md](gui/asl.md): windows and
  gadgets.
- [dos.md](dos.md): files, directories, processes, arguments.
- `apps/vkinfo` and `apps/vkplasma`: real programs built this way (Vulkan;
  see [vulkan.md](vulkan.md)).
- `templates/program`: the starting point.
