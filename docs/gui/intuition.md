# Windows and events: `intuition`

The `intuition` module opens windows, delivers what the user does in them
as Rust events, and shows simple requesters. It wraps the parts of
`intuition.library` a small program needs, with safe types that clean up
after themselves.

Use it when you want a plain window: something to draw into, keys and
mouse clicks, maybe a menu. If you want buttons, text fields, lists and
an automatic layout, use [ReAction](reaction.md) instead; it builds on the
same screen and requester types described here.

## Cargo feature

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["intuition", "graphics"] }
```

| Feature | What it adds |
|---|---|
| `intuition` | `PubScreen`, `Window`, events, `easy_request`, `Requester`; also the BOOPSI menu module `menu` (see [Menus](menus.md)) |
| `graphics` | `Window::rastport()`, to draw into the window (see [Graphics and layers](graphics-and-layers.md)) |
| `gadtools` | `MenuStrip` and `MenuEntry`, GadTools menus (see [Menus](menus.md)) |
| `layers` | `Window::clip()`, drawing limited to a region; pulls in `graphics` |
| `workbench` | `Window::set_app_window()`, files dropped on the window from Workbench |

The module works in both kinds of program the crate supports: an ordinary
`std` program on clib4 (start from `templates/program`; this is the
easiest route and what the examples here assume), and a program with no C
runtime (`#![no_std]`, see "Drivers, libraries and resources" in the
repository README). It only needs `alloc`.

You never declare `IIntuition` yourself: the crate opens
`intuition.library` the first time a call needs it.

## A few AmigaOS words

- **Screen**: the whole display a window sits on. The Workbench screen is
  the usual one.
- **Public screen**: a screen that other programs may open windows on.
  You *lock* it while you use it so it cannot close under you.
- **Window**: a rectangle on a screen with a border, a title bar and
  optional system gadgets (close, depth, size, drag bar).
- **IDCMP** (Intuition Direct Communication Message Port): the message
  port Intuition sends a window's events to. You choose which kinds
  (classes) of event you want with a bit mask.
- **Requester**: a small dialogue window with a message and buttons.

## Public screens: `PubScreen`

A window opens on a locked public screen. The lock is released when the
`PubScreen` is dropped, and every window opened on it borrows it, so the
compiler makes sure the windows close first.

```rust
use amigaos4::intuition::PubScreen;

let screen = PubScreen::default_screen()?;   // usually Workbench
// or: let screen = PubScreen::lock("MYSCREEN")?;
let (width, height) = screen.size();
println!("{} is {}x{}", screen.title(), width, height);
screen.beep();                                // DisplayBeep
```

## Opening a window: `Window` and `WindowOptions`

`Window::open(&screen, &options)` opens a window; dropping the `Window`
closes it. `WindowOptions` implements `Default`, so give only what you
change:

| Field | Default | Meaning |
|---|---|---|
| `title` | `""` | the title bar text |
| `left`, `top` | `0`, `0` | position on the screen |
| `inner_width`, `inner_height` | `200`, `100` | size of the inside, without borders |
| `idcmp` | `idcmp::CLOSEWINDOW` | the event classes to receive |
| `close_gadget`, `drag_bar`, `depth_gadget` | `true` | system gadgets |
| `size_gadget` | `false` | a sizing gadget (the user can resize) |
| `activate` | `true` | make it the active window when it opens |
| `rmb_trap` | `false` | get the right mouse button as a click instead of showing menus |
| `simple_refresh` | `false` | see [Simple-refresh windows](#simple-refresh-windows) |

```rust
use amigaos4::intuition::{idcmp, PubScreen, Window, WindowOptions};

let screen = PubScreen::default_screen()?;
let win = Window::open(&screen, &WindowOptions {
    title: "My window",
    left: 40,
    top: 40,
    inner_width: 320,
    inner_height: 160,
    idcmp: idcmp::CLOSEWINDOW | idcmp::VANILLAKEY | idcmp::NEWSIZE,
    size_gadget: true,
    ..Default::default()
})?;
```

The window can be resized by the user between 40x20 pixels and the size
of the screen.

### Geometry

All three read the window as it is now:

| Method | Returns |
|---|---|
| `bounds()` | `(left, top, width, height)`: position on the screen and outer size |
| `borders()` | `(left, top, right, bottom)` border widths |
| `inner()` | `(x0, y0, x1, y1)`: the inside in window coordinates, *inclusive* |

Window coordinates start at the window's top-left corner, border
included. Draw inside `inner()` so you do not paint over the borders.

### Changing the window

| Method | Does |
|---|---|
| `set_title(&str)` | a new title (the crate keeps the string alive) |
| `activate()` | makes it the active window |
| `to_front()` | brings it in front of other windows |
| `change_box(left, top, width, height)` | moves and resizes it (outer size) |
| `set_busy(bool)` | shows the busy pointer (after a short delay), or the normal one again |
| `set_idcmp(classes)` | changes the event classes received from now on |
| `as_ptr()` | the raw `struct Window` pointer, for calls the crate does not wrap |

`activate`, `to_front` and `change_box` are carried out later by
Intuition, not before the call returns. After `change_box`, wait for
`Event::ChangeWindow` (or `Event::NewSize` if the size changed) before
reading `bounds()` again.

`set_busy(true)` is useful around anything slow, so the user sees the
program is working:

```rust
win.set_busy(true);
let result = load_big_file();
win.set_busy(false);
```

## Events

### IDCMP classes

Ask for the classes you need in `WindowOptions::idcmp` (or later with
`set_idcmp`). The constants are in `intuition::idcmp`:

| Constant | Gives |
|---|---|
| `CLOSEWINDOW` | `Event::CloseWindow` (the close gadget) |
| `VANILLAKEY` | `Event::Key(u8)`: a character key, already translated by the keymap |
| `RAWKEY` | `Event::RawKey { code, qualifier }`: any key, including cursor and function keys |
| `MOUSEBUTTONS` | `Event::Button { code, x, y }` |
| `MOUSEMOVE` | `Event::MouseMove { x, y }` |
| `NEWSIZE` | `Event::NewSize`: the size changed |
| `CHANGEWINDOW` | `Event::ChangeWindow`: moved or resized |
| `REFRESHWINDOW` | `Event::RefreshWindow`: part of a simple-refresh window must be redrawn |
| `ACTIVEWINDOW`, `INACTIVEWINDOW` | `Event::Active`, `Event::Inactive` |
| `INTUITICKS` | `Event::Tick`: about ten a second while the window is active |
| `MENUPICK` | `Event::Menu { id, checked }`: a menu item was chosen |

Anything else arrives as `Event::Other { class, code }`.

For `Event::Button`, `code` is one of the `intuition::code` constants:
`SELECTDOWN` / `SELECTUP` (left button) and `MENUDOWN` / `MENUUP` (right
button, only with `rmb_trap`). `x` and `y` are window coordinates. The
same module has qualifier bits for `RawKey`: `QUAL_LSHIFT`,
`QUAL_RSHIFT`, `QUAL_CONTROL`, `QUAL_LALT`. In a raw key code, bit 7
(`0x80`) set means the key was released.

### Reading events: `next` and `wait`

Intuition sends each event as a message that must be replied. The crate
does that for you: `next()` takes one message, copies what matters into a
`Message`, replies it, and returns the copy. Your program never holds an
Intuition message.

```rust
pub struct Message {
    pub event: Event,
    pub qualifier: u16,   // keys and buttons held at the time
    pub seconds: u32,     // time stamp
    pub micros: u32,
}
```

- `win.next()` returns `Some(Message)` if an event is waiting, `None` if
  not. It never blocks.
- `win.wait(others)` blocks until an event arrives and returns
  `(Some(message), 0)`. `others` is a mask of extra signals to wake up
  for; when one of those arrives first, you get the next event if there
  is one (often `None`) and the signals that came. Pass `0` to wait for
  window events only.
- `win.signal_mask()` is the window's own signal bit, for building your
  own `Wait` over several sources with `amigaos4::signal::wait(mask)`.

A loop that also ends on CTRL-C:

```rust
use amigaos4::intuition::Event;
use amigaos4::sys::SIGBREAKF_CTRL_C;

loop {
    let (msg, signals) = win.wait(SIGBREAKF_CTRL_C);
    if signals & SIGBREAKF_CTRL_C != 0 {
        break;
    }
    match msg.map(|m| m.event) {
        Some(Event::CloseWindow) => break,
        Some(Event::Key(b'q')) => break,
        Some(Event::Key(k)) => println!("key {}", k as char),
        _ => {}
    }
}
```

When you `Wait` yourself on several signals, drain every window with
`while let Some(m) = win.next() { ... }` after each wake-up: one signal
can stand for several queued events.

## Drawing into the window: `rastport()`

With the `graphics` feature, `win.rastport()` lends you the window's
**RastPort** (the drawing context: pens, font, position, and the bitmap
it draws on). It returns a `graphics::RastPort` borrowed from the window,
so you cannot close the window while drawing.

```rust
use amigaos4::graphics::Rect;

let (x0, y0, x1, y1) = win.inner();
let mut rp = win.rastport()?;
rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF20_4080)?;   // ARGB
rp.set_a_pen(1);
rp.text(x0 + 8, y0 + 8, b"Hello")?;
```

Every coordinate is checked against the window's outer size: a call that
would reach outside it returns `Err(AmigaError::InvalidArgument(..))`
instead of drawing. Parts hidden behind other windows are clipped by the
system. If you change the RastPort's font with `set_font`, the window's
own font is put back when `rp` is dropped. See
[Graphics and layers](graphics-and-layers.md) for all drawing calls.

`rastport()` takes `&mut self`, so drop `rp` before calling other methods
on the window that need it mutably.

## Simple-refresh windows

By default a window is *smart refresh*: when another window covers it,
the system keeps a copy of the hidden part and puts it back. You draw
once and redraw only after a resize.

A *simple-refresh* window (`simple_refresh: true`) keeps nothing. When a
hidden part becomes visible again, Intuition sends
`Event::RefreshWindow` (ask for `idcmp::REFRESHWINDOW`) and you redraw.
`Window::refresh()` wraps Intuition's BeginRefresh and EndRefresh: it
returns a guard, and while the guard lives, drawing through its
`rastport()` reaches only the damaged parts. EndRefresh runs when the
guard is dropped.

```rust
match m.event {
    Event::RefreshWindow => {
        let (x0, y0, x1, y1) = win.inner();
        let mut guard = win.refresh();
        if let Ok(mut rp) = guard.rastport() {
            let _ = rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF00_C000);
        }
    } // EndRefresh here
    _ => {}
}
```

Simple refresh saves memory, but the program must be able to redraw at
any time. On a screen that composites windows, covering and uncovering
may not damage anything, so `RefreshWindow` may never arrive; your
program must still handle it.

## Drawing through a clip region: `clip()`

With the `layers` feature, `win.clip(&region)` installs a
`graphics::Region` as the window's clip region. Drawing through the
returned guard's `rastport()` reaches only the region (in window
coordinates). The old clip region is put back when the guard is dropped.
Do not keep the guard across a refresh. See
[Graphics and layers](graphics-and-layers.md#a-windows-clip-region-windowclip).

## Requesters

### `easy_request`: ask and wait

```rust
use amigaos4::intuition::easy_request;

let answer = easy_request(Some(&win), "Save changes?", "The picture has changed.", &["Save", "Discard", "Cancel"])?;
match answer {
    0 => save(),
    1 => {}
    _ => return Ok(()),   // Cancel
}
```

- The result is the index of the button chosen, counted from the left
  (0, 1, 2, ...).
- `window`: `Some(&win)` opens the requester on that window's screen and
  ties it to the window; `None` uses the default public screen. Any type
  implementing `intuition::ParentWindow` works, including a ReAction
  window.
- Texts are shown exactly as given: a `%` is shown as `%`. Use `\n` for
  a new line. A button text must not contain `|`, and at least one button
  is needed.
- The call blocks until the user answers.

### `Requester`: ask without blocking

`Requester::open(title, text, buttons)` shows the same kind of requester
but returns at once. Call `poll()` from time to time: it returns
`Some(index)` once a button was chosen, `None` until then. Dropping the
`Requester` closes it, answered or not.

```rust
use amigaos4::intuition::Requester;

let req = Requester::open("Working", "Copying files...\nClick Stop to end.", &["Stop"])?;
for file in files {
    if req.poll().is_some() {
        break;
    }
    copy(file);
}
// req is dropped here and the requester goes away
```

## A complete program

A `std` program (start from `templates/program`) with a resizable window.
It counts left-button clicks, redraws after a resize, and asks before
quitting.

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["intuition", "graphics"] }
```

```rust
use amigaos4::error::Result;
use amigaos4::graphics::Rect;
use amigaos4::intuition::{code, easy_request, idcmp, Event, PubScreen, Window, WindowOptions};
use amigaos4::sys::SIGBREAKF_CTRL_C;

/// The stack the Shell gives the program.
#[used]
static STACK: [u8; 14] = *b"$STACK:262144\0";

fn draw(win: &mut Window<'_>, clicks: u32) -> Result<()> {
    let (x0, y0, x1, y1) = win.inner();
    if x1 < x0 || y1 < y0 {
        return Ok(()); // no inside to draw in
    }
    let mut rp = win.rastport()?;
    rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF30_3050)?;
    rp.set_a_pen(1);
    let line = format!("Clicks: {}  (q quits)", clicks);
    // Refused (an error) if the text does not fit: ignore it then.
    let _ = rp.text(x0 + 8, y0 + 8, line.as_bytes());
    Ok(())
}

fn run() -> Result<()> {
    let screen = PubScreen::default_screen()?;
    let mut win = Window::open(&screen, &WindowOptions {
        title: "Click counter",
        left: 60,
        top: 60,
        inner_width: 300,
        inner_height: 120,
        idcmp: idcmp::CLOSEWINDOW | idcmp::VANILLAKEY | idcmp::MOUSEBUTTONS | idcmp::NEWSIZE,
        size_gadget: true,
        ..Default::default()
    })?;
    let mut clicks = 0;
    draw(&mut win, clicks)?;

    loop {
        let (msg, signals) = win.wait(SIGBREAKF_CTRL_C);
        if signals & SIGBREAKF_CTRL_C != 0 {
            break;
        }
        let Some(m) = msg else { continue };
        match m.event {
            Event::CloseWindow | Event::Key(b'q') => {
                if easy_request(Some(&win), "Click counter", "Really quit?", &["Quit", "Cancel"])? == 0 {
                    break;
                }
            }
            Event::Button { code: code::SELECTDOWN, .. } => {
                clicks += 1;
                draw(&mut win, clicks)?;
            }
            Event::NewSize => draw(&mut win, clicks)?,
            _ => {}
        }
    }
    Ok(())
    // win is dropped (closed) before screen (unlocked)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("click counter: {}", e);
        std::process::exit(10);
    }
}
```

## Limitations

- Not wrapped yet: gadgets and BOOPSI objects on a plain window (use
  [ReAction](reaction.md) for gadgets), screens of the program's own
  (only public screens), and Workbench start-up handling in this module
  (see the `workbench` module).
- Text drawn with `RastPort::text` is bytes in the font's character set
  (Latin-1), not UTF-8.
- `rastport()` limits drawing to the window's outer size, not its
  inside: keep to `inner()` yourself to leave the borders alone.
