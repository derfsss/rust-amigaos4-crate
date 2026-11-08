# Gadget windows: `reaction`

**ReAction** is AmigaOS's toolkit of BOOPSI classes for windows with
gadgets. A *gadget* is a control: a button, a text field, a check box, a
slider, a list. *BOOPSI* is Intuition's object system; each kind of
gadget is a class (`button.gadget`, `string.gadget`, ...), and
`window.class` and `layout.gadget` arrange them for you, so you never
compute a pixel position.

The `reaction` module lets you describe a window as a tree of plain Rust
values (layouts containing gadgets), then opens it. The crate makes the
objects, keeps every string and list they point at alive, reports what
the user does as events, and lets you read and set gadget values by a
numeric id. Dropping the `Window` disposes of everything.

Use it for anything form-like: settings windows, dialogues, tools with
buttons and lists. For a bare window you draw into yourself, see
[intuition](intuition.md).

## Cargo feature

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["reaction"] }
# to draw into a space gadget, add graphics:
# amigaos4 = { git = "...", features = ["reaction", "graphics"] }
```

`reaction` pulls in `intuition`, `listbrowser`, `workbench` and
`speedbar`. It does **not** pull in `graphics`: `Window::rastport()`
needs `graphics` added (and `icon` for `AppIcon`, the Workbench icon
a program can show; ReAction windows do not need it).

The module works in a `std` program (start from `templates/program`) and
in a program with no C runtime. The real-world reference is
`apps/imgview` in the repository, a picture viewer built without a C
runtime on a ReAction window with menus, a context menu, a toolbar,
border scrollers, iconify and file drops.

The classes are opened the first time a window needs them and kept for
the rest of the program.

## Describing a window

### Layouts

A `Layout` is a row or a column of children. Children are gadgets or
other layouts (`Node::Gadget` / `Node::Layout`, made for you by
`.child(...)`).

| Method | Meaning |
|---|---|
| `Layout::vertical()` | children one above the other |
| `Layout::horizontal()` | children side by side |
| `.child(gadget_or_layout)` | add a child |
| `.label("Title")` | a framed group with this title |
| `.even()` | all children the same size |
| `.weight(w)` | its share of the parent layout's room (default 100) |

### Gadgets

Every gadget has a `u16` id of your choosing. Events report it, and the
value methods take it. Give each gadget a different id.

| Constructor | Class | What it is | Value methods |
|---|---|---|---|
| `Gadget::button(id, "Text")` | button | a push button | `checked` (only with `.push()`) |
| `Gadget::string(id)` | string | a one-line text field | `text`, `set_text` |
| `Gadget::checkbox(id, "Text")` | checkbox | a check box with text beside it | `checked`, `set_checked` |
| `Gadget::integer(id)` | integer | a number field with arrows, 0 to 100 | `number`, `set_number` |
| `Gadget::slider(id)` | slider | a horizontal slider, 0 to 100 | `level`, `set_level` |
| `Gadget::chooser(id, &["A", "B"])` | chooser | a pop-up choice | `active`, `set_active` |
| `Gadget::list(id, &["a", "b"])` | listbrowser | a list of rows, one column | `selected`, `set_selected`, `set_items` |
| `Gadget::gauge(id)` | fuelgauge | a progress bar, 0 to 100, shown as a percentage | `level`, `set_level` |
| `Gadget::space(id)` | space | an empty area you draw in | `render_box` |
| `Gadget::toolbar(id, buttons)` | speedbar | a row of buttons | `tool_selected`, `set_tool_selected` |

Builder methods (each applies only to the kinds named; on others it does
nothing):

| Method | For | Meaning |
|---|---|---|
| `.text("...")` | string | starting text |
| `.max_chars(n)` | string | most characters it takes (default 256) |
| `.checked(on)` | checkbox, button | ticked, or a push button down |
| `.push()` | button | stays down when clicked, until clicked again |
| `.range(lo, hi)` | integer, slider, gauge | the range |
| `.value(v)` | integer, slider, gauge | starting value |
| `.vertical()` | slider | a vertical slider |
| `.select(k)` | chooser, list | starting item, or selected row |
| `.disabled(on)` | all | greyed out and not usable |
| `.hint("...")` | all | a help bubble when the mouse rests on it |
| `.context_menu(strip)` | all | a context menu on the right button (see [Menus](menus.md)) |
| `.label("...")` | all | a text beside it in its layout |
| `.weight(w)` | all | its share of the layout's room (default 100) |
| `.min_size(w, h)` | all | the least room it takes, in pixels |

### Labels, weights and minimum sizes

- `.label("Name")` puts a text to the left of the gadget, and the layout
  lines up the labels of a column.
- Room left over in a layout is shared out by weight. `.weight(0)` keeps
  a child at its own size, which is what a toolbar in a vertical layout
  wants.
- `.min_size(w, h)` stops a gadget getting smaller than that. A list or a
  space gadget needs one, or it may be laid out tiny.
- The window's own least size comes from the layout; the user cannot make
  it smaller.

## Opening the window

```rust
use amigaos4::intuition::PubScreen;
use amigaos4::reaction::{Gadget, Layout, Window, WindowSpec};

let screen = PubScreen::default_screen()?;
let root = Layout::vertical()
    .child(Gadget::string(1).label("Name"))
    .child(Gadget::button(2, "OK"));
let win = Window::open(&screen, &WindowSpec { title: "Hello", ..Default::default() }, root)?;
```

`Window::open(&screen, &spec, root)` takes the root layout by value. The
window borrows the screen. `WindowSpec` implements `Default`:

| Field | Default | Meaning |
|---|---|---|
| `title` | `""` | the title bar text |
| `position` | `None` | `Some((left, top))`; `None` centres the window on the screen |
| `centre_on_mouse` | `false` | with no `position`: centre on the mouse instead |
| `inner_size` | `None` | `Some((w, h))`; `None` uses the layout's own size |
| `activate` | `true` | active when it opens |
| `size_gadget` | `true` | resizable |
| `close_gadget` | `true` | a close gadget |
| `menu` | `None` | `Some(&strip)`: the window's menus (see [Menus](menus.md)) |
| `screen_title` | `None` | the screen's title while the window is active |
| `simple_refresh` | `false` | the system keeps no copy of hidden parts; redraw your own drawing after `NewSize` and `Uniconified` |
| `iconify` | `false` | an iconify gadget (see below) |
| `drops` | `false` | files dropped from Workbench arrive as `Event::Dropped` |
| `icon_title` | `None` | the iconified icon's label (default: the title) |
| `scrollers` | `false` | scrollers in the right and bottom borders |

Every window also gets a drag bar, a depth gadget and new-look menus.

## Events

`win.next()` returns the next `reaction::Event`, or `None` when there are
no more for now. It never blocks. Wait on `win.signal_mask()`, then
drain:

```rust
loop {
    amigaos4::signal::wait(win.signal_mask());
    while let Some(e) = win.next() {
        // handle e
    }
}
```

| `Event` | When |
|---|---|
| `Close` | the close gadget |
| `Gadget { id, code }` | a gadget was released: a button clicked, a string ended with Return, a check box, slider, chooser or list changed, a toolbar button clicked. `code` is the slider's level, the chooser's item, the list's row, or the toolbar button's id |
| `Key(u8)` | a character key with no gadget active |
| `RawKey { code, qualifier }` | a raw key (cursor keys, function keys) with the qualifiers held (shift `0x0003`, alt `0x0030`) |
| `Menu { id, checked, gadget }` | a menu item picked; `gadget` is `Some(id)` for a gadget's context menu |
| `NewSize` | the window has a new size |
| `Active`, `Inactive` | activation changed |
| `Iconified`, `Uniconified` | see below |
| `Dropped` | files were dropped: `win.dropped()` gives their paths |
| `Scroll { vertical, top }` | a border scroller was moved |
| `Other(u32)` | anything else window.class reported |

To combine with other signals (CTRL-C, another port), OR their masks
with `win.signal_mask()` in one `amigaos4::signal::wait` call, then check
each source.

## Reading and setting values by id

| Read | Set | For |
|---|---|---|
| `text(id) -> Result<String>` | `set_text(id, &str)` | string |
| `checked(id) -> Result<bool>` | `set_checked(id, bool)` | checkbox, push button |
| `number(id) -> Result<i32>` | `set_number(id, i32)` | integer (held to its range) |
| `level(id) -> Result<i32>` | `set_level(id, i32)` | slider, gauge |
| `active(id) -> Result<u32>` | `set_active(id, u32)` | chooser |
| `selected(id) -> Result<Option<u32>>` | `set_selected(id, Option<u32>)` | list (`set_selected` also scrolls the row into view) |
| | `set_items(id, &[&str])` | list: replace the rows (none selected after); takes `&mut self` |
| `disabled(id) -> Result<bool>` | `set_disabled(id, bool)` | any gadget |
| `kind(id) -> Option<GadgetKind>` | | which kind a gadget is |

An unknown id, or a method used on the wrong kind of gadget (`number` on
a string), returns an error.

## A worked example: a small form

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["reaction"] }
```

```rust
use amigaos4::error::Result;
use amigaos4::intuition::{easy_request, PubScreen};
use amigaos4::reaction::{Event, Gadget, Layout, Window, WindowSpec};
use amigaos4::sys::SIGBREAKF_CTRL_C;

#[used]
static STACK: [u8; 14] = *b"$STACK:262144\0";

// Gadget ids.
const NAME: u16 = 1;
const AGE: u16 = 2;
const COLOUR: u16 = 3;
const NEWS: u16 = 4;
const SAVE: u16 = 10;
const CANCEL: u16 = 11;

fn form() -> Layout {
    Layout::vertical()
        .child(
            Layout::vertical()
                .label("About you")
                .child(Gadget::string(NAME).max_chars(40).label("Name").hint("Your full name"))
                .child(Gadget::integer(AGE).range(0, 130).value(30).label("Age"))
                .child(Gadget::chooser(COLOUR, &["Red", "Green", "Blue"]).label("Colour")),
        )
        .child(Gadget::checkbox(NEWS, "Send me the newsletter"))
        .child(
            Layout::horizontal()
                .even()
                .child(Gadget::button(SAVE, "Save"))
                .child(Gadget::button(CANCEL, "Cancel")),
        )
}

fn run() -> Result<()> {
    let screen = PubScreen::default_screen()?;
    let win = Window::open(&screen, &WindowSpec { title: "Sign up", ..Default::default() }, form())?;
    // Save stays greyed out until a name is typed.
    win.set_disabled(SAVE, true)?;

    loop {
        let got = amigaos4::signal::wait(win.signal_mask() | SIGBREAKF_CTRL_C);
        if got & SIGBREAKF_CTRL_C != 0 {
            return Ok(());
        }
        while let Some(e) = win.next() {
            match e {
                Event::Close | Event::Gadget { id: CANCEL, .. } => return Ok(()),
                Event::Gadget { id: NAME, .. } => {
                    // Return was pressed in the name field (or it was left).
                    let empty = win.text(NAME)?.trim().is_empty();
                    win.set_disabled(SAVE, empty)?;
                }
                Event::Gadget { id: SAVE, .. } => {
                    let colours = ["red", "green", "blue"];
                    let summary = format!(
                        "{}, {} years old, likes {}{}",
                        win.text(NAME)?,
                        win.number(AGE)?,
                        colours[win.active(COLOUR)? as usize],
                        if win.checked(NEWS)? { ", wants the newsletter" } else { "" },
                    );
                    easy_request(Some(&win), "Sign up", &summary, &["OK"])?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("sign up: {}", e);
        std::process::exit(10);
    }
}
```

Points to note:

- The window is laid out by the classes, so the program never gives a
  pixel position. The labels in the "About you" group line up.
- A string gadget reports `Event::Gadget` when the user presses Return
  or leaves the field, not on every key.
- A ReAction window is a `ParentWindow`, so `easy_request(Some(&win),
  ...)` opens the requester on its screen.

## Toolbars

`Gadget::toolbar(id, buttons)` is a row of buttons (speedbar.gadget). A
click arrives as `Event::Gadget { id: <the toolbar's id>, code: <the
button's id> }`.

| `ToolButton` | Meaning |
|---|---|
| `ToolButton::new(id, "Label")` | a button with a `u16` id |
| `.image(path)` | an image file (for example a 24x24 PNG) shown instead of the label; give several and the first that exists is used, else the label is shown as text |
| `.hint("...")` | a help bubble when the mouse rests on it |
| `.help("...")` | a text shown in the window title while the mouse is over it |
| `.toggle(on)` | a toggle button: stays pressed in while on |
| `ToolButton::separator()` | a line between groups |

For toggle buttons, `win.tool_selected(bar_id, button_id)` reads the
state and `win.set_tool_selected(bar_id, button_id, on)` sets it. A click
flips a toggle button by itself.

imgview builds its toolbar like this, looking for each image in three
places:

```rust
use amigaos4::reaction::{Gadget, Layout, ToolButton};

fn tool(id: u16, text: &str, image: &str, hint: &str) -> ToolButton {
    ToolButton::new(id, text)
        .hint(hint)
        .image(&format!("PROGDIR:Images/{}", image))
        .image(&format!("TBImages:{}", image))
}

let bar = vec![
    tool(1, "<", "nav_west", "Previous picture"),
    tool(2, ">", "nav_east", "Next picture"),
    ToolButton::separator(),
    tool(3, "Fit", "fit", "Fit to window").toggle(true),
];
let root = Layout::vertical()
    .child(Gadget::space(100).min_size(160, 100))
    .child(Gadget::toolbar(200, bar).weight(0));   // weight 0: its own height
```

`PROGDIR:` is the drawer the program was started from.

## Hints

`.hint("...")` on a gadget or a tool button gives it a help bubble. When
any gadget in the window has a hint, the window turns gadget help on by
itself.

## Drawing in a space gadget: `render_box` and `rastport`

`Gadget::space(id)` is an empty area for your own drawing (a picture, a
graph). Give it a `.min_size`.

- `win.render_box(id)` returns `Some((x0, y0, x1, y1))`, the area in
  window coordinates (inclusive), never past the window's inside. It is
  `None` while iconified, or for a gadget that is not a space gadget.
- `win.rastport()` (feature `graphics`) lends you the window's RastPort,
  limited to the window's outer size; it is `Err` while iconified. See
  [Graphics and layers](graphics-and-layers.md).

window.class lays the window out *while it handles input*, that is inside
`next()`. The render box read straight after opening the window, or
straight after `change_box`, is the old (or minimum) box. So:

1. Drain the events (`while let Some(e) = win.next()`).
2. Then read `render_box` and draw.
3. Draw again on `NewSize` and `Uniconified`, and whenever the render
   box differs from the one you last drew.

When the window has shrunk, call `win.redraw_gadgets()` before drawing:
gadgets the new layout moved may otherwise be left undrawn.

```rust
use amigaos4::graphics::Rect;

const PIC: u16 = 100;
let mut drawn = (0, 0, -1, -1);
let mut redraw = true;
loop {
    while let Some(e) = win.next() {
        match e {
            Event::Close => return Ok(()),
            Event::NewSize | Event::Uniconified => redraw = true,
            _ => {}
        }
    }
    if let Some(area) = win.render_box(PIC) {
        if redraw || area != drawn {
            if area != drawn {
                win.redraw_gadgets();
            }
            let (x0, y0, x1, y1) = area;
            if let Ok(mut rp) = win.rastport() {
                let _ = rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF50_5050);
            }
            drawn = area;
            redraw = false;
        }
    }
    amigaos4::signal::wait(win.signal_mask());
}
```

A `.context_menu(strip)` on a space gadget appears when the user presses
the right button over the area.

## Border scrollers

With `WindowSpec { scrollers: true, .. }` the window gets a scroller in
its right border and one in its bottom border.

- `win.set_scroll(vertical, total, visible, top)`: `total` units, of
  which `visible` are shown, the first shown at `top` (pixels, rows,
  whatever suits). Only changed values are sent, so calling it often is
  cheap.
- When the user moves one, `Event::Scroll { vertical, top }` arrives with
  the new top.

imgview sets them from the picture and window sizes after each draw:

```rust
win.set_scroll(false, picture_width, area_width, scroll_x);
win.set_scroll(true, picture_height, area_height, scroll_y);
```

## Other window methods

| Method | Does |
|---|---|
| `signal_mask()` | the signals to wait on (the event port, and the AppPort with `iconify` or `drops`) |
| `as_ptr()` | the raw Intuition window; null while iconified |
| `bounds()` | `Some((left, top, width, height))`; `None` while iconified |
| `min_size()` | the least outer size the layout allows |
| `change_box(l, t, w, h)` | move and resize (carried out later; never below `min_size`) |
| `to_front()` | activate and bring to the front |
| `set_title(&str)` | a new title (also the icon's while iconified) |
| `set_busy(bool)` | the busy pointer, or the normal one again |
| `redraw_gadgets()` | draw the frame and gadgets again |
| `menu_checked`, `set_menu_checked`, `set_menu_disabled` | menu state by id (see [Menus](menus.md)) |

### Iconify

With `WindowSpec { iconify: true, .. }` the window has an iconify gadget.
Clicking it closes the window and puts an icon on Workbench
(`Event::Iconified`); double-clicking the icon opens the window again
(`Event::Uniconified`: redraw your own drawing). `win.iconify()` and
`win.uniconify()` do the same from code, and `win.is_iconified()` tells
you which state it is in. While iconified there is no Intuition window:
`as_ptr()` is null, `bounds()` and `render_box()` are `None`, and
`rastport()` is an error.

### File drops

With `WindowSpec { drops: true, .. }` the window is an *AppWindow*: icons
dropped on it from Workbench arrive as `Event::Dropped`, and
`win.dropped()` returns the paths of the oldest drop not yet taken.

## Limitations

- The gadget kinds are those listed above; other ReAction classes are
  not wrapped.
- A list has one column of text.
- The layout is done by window.class while it handles input, so sizes
  read before the events are drained are stale (see the space gadget
  section).
