# Console: text windows and key translation

The `console` module wraps **console.device**, the AmigaOS device that draws
text in a window and turns key presses into characters. (A *device* on
AmigaOS is a driver you talk to by sending it I/O requests; the crate does
that part for you.)

## What it is for

- **Turning a raw key into text.** Intuition reports keys as *raw key codes*
  (a number per physical key) plus *qualifiers* (Shift, Alt, Ctrl and so on).
  `raw_key_convert` turns one into the bytes the user's keymap gives.
- **A simple text window.** `Console` opens a window that console.device
  creates and manages. You write text into it, read what the user types, and
  ask for its size and cursor position.
- **Text in your own window.** `WindowConsole` attaches a console to a window
  your program already opened.

Use it when you want a terminal-style window, or when you handle raw keys
from an Intuition window and need the characters. For a program that only
prints to the Shell it was started from, you do not need this module: see
`Stream` in [dos.md](dos.md).

## Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", features = ["console"] }
# with a console on your own window:
amigaos4 = { path = "...", features = ["console", "intuition"] }
```

| Feature | Gives | Pulls in |
|---|---|---|
| `console` | `raw_key_convert`, `Console`, `Unit`, `WindowSpec` | `device` (the crate's I/O request helpers) and `timer` (timer.device, used for the read time limit) |
| `intuition` | `WindowConsole` | nothing else |

## Build modes

| Mode | Status |
|---|---|
| no-C-runtime program (`#![no_std]`) | Tested by the crate's test programs: key conversion, a console window, writing, CSI sequences, size and cursor, a read with a time limit, typed input, and a console on the program's own window. |
| std program (`fn main` on clib4) | Compiles; not exercised by the crate's test programs. |

You need no interface global of your own. If the program has no `IConsole`,
the crate opens console.device "as a library" once and keeps it.

## `raw_key_convert`

```rust
pub fn raw_key_convert(code: u16, qualifier: u16) -> Result<Vec<u8>>
```

It returns the bytes that raw key `code` produces with the qualifier bits
`qualifier` (the `IEQUALIFIER_*` bits), using the default keymap.

```rust
use amigaos4::console::raw_key_convert;

let a = raw_key_convert(0x20, 0)?;           // b"a"
let shifted = raw_key_convert(0x20, 0x0001)?; // b"A" (left Shift)
let up = raw_key_convert(0x4C, 0)?;          // b"\x9bA" (cursor up)
```

Points to know:

- A key release (the code with bit 7 set, such as `0x20 | 0x80`) gives an
  empty vector.
- Special keys come back as **CSI sequences**. CSI is the byte `0x9B`, the
  one-byte form of `ESC [`. Cursor up is `0x9B 'A'`.
- Dead keys (accents typed before a letter) are not combined.

## `Console`: a console in its own window

`Console::open` asks console.device to create a window (on the default
public screen) and attach a console to it. Dropping the `Console` closes the
device and the window.

```rust
use amigaos4::console::{Console, Unit, WindowSpec};

let spec = WindowSpec {
    title: "My console",
    left: 60,
    top: 60,
    width: 480,
    height: 200,
    activate: true,
};
let mut con = Console::open(Unit::CharMap, &spec)?;

con.write(b"Hello from Rust\n")?;
con.write(b"\x9b1mBold\x9b0m and normal\n")?; // CSI sequences work

let (rows, cols) = con.size()?;   // text area in characters
let (x, y) = con.cursor()?;       // cursor column and row, from 0
```

### The unit kinds

| `Unit` | Meaning |
|---|---|
| `Standard` | No character map: text that another window covers is not redrawn. |
| `CharMap` | Keeps a character map, so the window redraws and resizes its text. |
| `SnipMap` | A character map with cut and paste. |

`CharMap` is a good default.

### Methods

| Method | What it does |
|---|---|
| `write(&mut self, bytes: &[u8]) -> Result<usize>` | Writes text and control sequences (`0x9B` or `ESC [`). Returns the bytes written. |
| `read(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<usize>` | Waits up to `timeout_ms` for typed input. Returns the bytes read, or `0` if nothing came; an unanswered read is aborted. |
| `clear_input(&mut self) -> Result<()>` | Throws away input not yet read. |
| `size(&mut self) -> Result<(i32, i32)>` | The text area as `(rows, columns)`. |
| `cursor(&mut self) -> Result<(i32, i32)>` | The cursor as `(column, row)`, counted from 0. |
| `window_ptr(&self) -> *mut sys::Window` | The raw window, for calls the crate does not wrap. Valid while the console lives. |

### Reading a line

The window must be active (selected) for keys to reach it, so set
`activate: true` when you want typed input. Return arrives as a carriage
return byte (`b'\r'`).

```rust
con.write(b"Type something and press Return: ")?;
let mut line = Vec::new();
let mut buf = [0u8; 32];
while !line.contains(&b'\r') {
    let n = con.read(&mut buf, 1000)?;
    if n == 0 {
        continue; // nothing typed in the last second
    }
    line.extend_from_slice(&buf[..n]);
}
```

The time limit lets your loop do other work, such as checking for Ctrl-C,
between reads.

### Report requests

console.device answers some control sequences by putting a reply in the
input stream, which you then `read`. For example `CSI 0 SPACE q` (the bytes
`b"\x9b0 q"`) asks for the window status and is answered with
`CSI 1;1;<rows>;<columns> SPACE r`. Prefer `size` and `cursor`: the crate's
tests found that the cursor position report (`CSI 6n`) answers with numbers
that are not the cursor.

## `WindowConsole`: a console on your own window

With the `intuition` feature, `WindowConsole::open` attaches a console to an
`intuition::Window` you opened. Text goes into the window's inner area, and
keys typed while the window is active can be read. Dropping the console
leaves your window open. The console borrows the window, so the window
cannot be closed while the console exists.

Everything `Console` offers is available through it (it dereferences to a
`Console`).

```rust
use amigaos4::console::{Unit, WindowConsole};
use amigaos4::intuition::{PubScreen, Window, WindowOptions};

let screen = PubScreen::default_screen()?;
let win = Window::open(&screen, &WindowOptions {
    title: "Console in my window",
    inner_width: 400,
    inner_height: 150,
    ..Default::default()
})?;

let mut con = WindowConsole::open(Unit::CharMap, &win)?;
con.write(b"in my own window")?;
let (rows, cols) = con.size()?;
drop(con); // the window stays open
```

## Limitations

- Not wrapped: keymaps, palettes, notify hooks and the snip (cut and paste)
  functions.
- `raw_key_convert` always uses the default keymap and does not combine dead
  keys.
- The window `Console` creates always has a drag bar, depth gadget and size
  gadget, on the default public screen.
- Each `read` opens timer.device for its time limit.

## See also

- [dos.md](dos.md): `Stream`, the program's own input and output.
- [input.md](input.md): joysticks, mice and keyboards through AmigaInput.
