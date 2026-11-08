# Menus

AmigaOS menus live in the screen's title bar: the user holds the right
mouse button over the active window, and the window's menus appear at the
top of the screen. Items can have an *Amiga-key shortcut* (the right
Amiga key plus a letter), a check mark, or a submenu.

The crate offers three ways to make menus:

| | GadTools strip | BOOPSI menus (`menu`) | Pop-up menu (`popup`) |
|---|---|---|---|
| Types | `intuition::MenuStrip`, `MenuEntry` | `menu::Strip`, `Menu`, `Item`, `MenuTree` | `popup::PopupMenu` (built from `menu::Item`) |
| Feature | `intuition` + `gadtools` | `intuition` | `popupmenu` (pulls in `intuition`) |
| Needs | gadtools.library | Intuition V54 (menuclass) | popupmenu.class |
| Shown | in the screen's bar | in the screen's bar, or as a context menu | at the mouse pointer, when you call `open` |
| Plain `intuition::Window` | `set_menu` | `set_menu_strip`, `set_context_menu` | `open(win.as_ptr())` |
| ReAction window | no | `WindowSpec::menu`, `Gadget::context_menu` | `open(win.as_ptr())` |
| Who owns it | you (the window borrows it) | the window | you |
| Radio groups | no | yes | shown, but nothing is excluded |
| Read and set state by id | check marks only (`set_menu_checked`) | check marks and disabled state | no |

**Which to use:**

- **BOOPSI menus** are the default choice. The window owns them, you
  change check marks and greyed-out state by id at any time, radio items
  work, and the same description serves as a menu bar or a context menu.
  They are the only kind a ReAction window takes.
- **GadTools strips** are the older way. Use one only if your program
  must run where Intuition is older than V54.
- **Pop-up menus** are for a menu at the pointer that you open yourself,
  for example on a right-click in a `rmb_trap` window. The call waits
  until the user picks or lets go.

Picks arrive in the window's event loop as a menu event with **your own
`u32` id**. Use non-zero ids (0 means "no item").

All three work in `std` programs and in programs with no C runtime.

## BOOPSI menus: the `menu` module

*BOOPSI* (Basic Object-Oriented Programming System for Intuition) is
AmigaOS's object system; menuclass is the BOOPSI class for menus. You
describe the menus with three plain Rust types, and the crate builds the
objects.

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["intuition"] }
```

### Describing a menu: `Strip`, `Menu`, `Item`

- `Strip::new().menu(m)...`: the menu bar, menus in order. A context menu
  is a strip with one menu.
- `Menu::new(title).item(it)...`: one title and its items.
  `.separator()` adds a line; `.disabled(true)` greys the whole menu out.
- `Item`: one entry.

| Constructor | Makes |
|---|---|
| `Item::action(id, label)` | an item that does something |
| `Item::toggle(id, label)` | an option with a check mark, switched on and off |
| `Item::radio(id, label, group)` | one of a set: picking it clears the other radio items of `group` in the same panel |
| `Item::separator()` | a line between items |
| `Item::submenu(label, vec![...])` | an item with items of its own (any depth) |

Builder methods on `Item`:

| Method | Meaning |
|---|---|
| `.key("O")` | the Amiga-key shortcut (one letter is handled; a longer text is shown only) |
| `.checked(true)` | a toggle or radio item starts checked |
| `.disabled(true)` | greyed out and not selectable |

```rust
use amigaos4::menu::{Item, Menu, Strip};

const OPEN: u32 = 1;
const QUIT: u32 = 2;
const FIT: u32 = 10;
const SMALL: u32 = 20;
const LARGE: u32 = 21;

let strip = Strip::new()
    .menu(
        Menu::new("Project")
            .item(Item::action(OPEN, "Open...").key("O"))
            .separator()
            .item(Item::action(QUIT, "Quit").key("Q")),
    )
    .menu(
        Menu::new("View")
            .item(Item::toggle(FIT, "Fit to window").key("F").checked(true))
            .separator()
            .item(Item::radio(SMALL, "Small", 1).checked(true))
            .item(Item::radio(LARGE, "Large", 1)),
    );
```

A strip must have at least one menu, and every menu at least one item;
otherwise building it fails with `InvalidArgument`.

### On a plain window

```rust
use amigaos4::intuition::{idcmp, Event, PubScreen, Window, WindowOptions};

let screen = PubScreen::default_screen()?;
let mut win = Window::open(&screen, &WindowOptions {
    title: "Menus",
    idcmp: idcmp::CLOSEWINDOW | idcmp::MENUPICK,   // MENUPICK is needed
    ..Default::default()
})?;
win.set_menu_strip(&strip)?;

loop {
    let (msg, _) = win.wait(0);
    match msg.map(|m| m.event) {
        Some(Event::CloseWindow) | Some(Event::Menu { id: QUIT, .. }) => break,
        Some(Event::Menu { id: OPEN, .. }) => println!("open"),
        Some(Event::Menu { id: FIT, checked }) => println!("fit is now {}", checked),
        Some(Event::Menu { id, .. }) => println!("menu item {}", id),
        _ => {}
    }
}
```

- `set_menu_strip(&strip)` builds the objects and attaches them; the
  window owns them and frees them when it closes, or when you attach
  another strip. It replaces a GadTools strip attached before.
- `Event::Menu { id, checked }`: `checked` is the item's check mark after
  the pick (for toggles and radio items). If the user picks several items
  in one go (dragging with the right button and clicking), each comes as
  its own event, in order.
- Amiga-key shortcuts arrive the same way.
- `clear_menu()` takes the strip off.

#### Context menu

`win.set_context_menu(&strip)` adds a context menu: a strip of one menu,
shown when the right button is pressed over the window's *inside*. Over
the borders and title bar the window's normal menus still come up. Its
picks arrive as `Event::Menu` like the strip's. `clear_context_menu()`
removes it.

```rust
let context = Strip::new().menu(
    Menu::new("Picture")
        .item(Item::toggle(FIT, "Fit to window"))
        .item(Item::action(OPEN, "Open...")),
);
win.set_context_menu(&context)?;
```

The same id may appear in the strip and the context menu.

#### State by id

| `Window` method | Does |
|---|---|
| `menu_checked(id) -> Option<bool>` | a check mark (`None` if no menu has the id) |
| `set_menu_checked(id, on) -> bool` | sets or clears it, without a menu event |
| `set_menu_disabled(id, on) -> bool` | greys an item out or makes it usable |

They act on both the strip and the context menu, so a setting changed by
a key press shows in both. They return `false` when no item has the id.
Changes show at once.

### On a ReAction window

```rust
use amigaos4::reaction::{Event, Gadget, Layout, Window, WindowSpec};

let root = Layout::vertical()
    .child(Gadget::space(1).min_size(200, 100).context_menu(context))
    .child(Gadget::button(2, "OK"));
let win = Window::open(&screen, &WindowSpec { title: "Menus", menu: Some(&strip), ..Default::default() }, root)?;
// in the loop:
// Event::Menu { id, checked, gadget } => ...
```

- `WindowSpec::menu: Option<&Strip>`: the window's menu bar. The window
  builds and owns the objects.
- `Gadget::context_menu(strip)` (takes the `Strip` by value): a context
  menu shown by the right button over that gadget.
- `reaction::Event::Menu { id, checked, gadget }`: `gadget` is
  `Some(gadget_id)` for a gadget's context menu, `None` for the menu bar.
- `menu_checked`, `set_menu_checked` and `set_menu_disabled` work as on a
  plain window.

See [ReAction](reaction.md).

### `MenuTree` directly

`MenuTree::new(&strip)` builds the objects without a window. It offers
`contains(id)`, `checked(id)`, `set_checked(id, on)`, `disabled(id)`,
`set_disabled(id, on)`, `selected()` (the ids picked in the last menu
session) and `as_ptr()` (the root object, for raw calls). It is disposed
on drop. Most programs never need it: the window methods above build and
hold the tree for you.

## GadTools strips: `MenuStrip`

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["intuition", "gadtools"] }
```

The menu is a flat list of `MenuEntry` lines. It must start with a
`Title`; items belong to the last title, sub-items to the last item.

| `MenuEntry` | |
|---|---|
| `Title(label)` | a menu in the bar |
| `Item { label, key, id }` | an item; `key: Some('O')` for Amiga-O |
| `Toggle { label, key, id, checked }` | an item with a check mark |
| `Sub { label, key, id }` | a sub-item of the last item |
| `Bar` | a separator line |

```rust
use amigaos4::intuition::{idcmp, Event, MenuEntry, MenuStrip, PubScreen, Window, WindowOptions};

let screen = PubScreen::default_screen()?;
let strip = MenuStrip::new(&screen, &[
    MenuEntry::Title("Project"),
    MenuEntry::Item { label: "Open...", key: Some('O'), id: 10 },
    MenuEntry::Toggle { label: "Autosave", key: Some('A'), id: 20, checked: false },
    MenuEntry::Bar,
    MenuEntry::Item { label: "Quit", key: Some('Q'), id: 99 },
])?;
let mut win = Window::open(&screen, &WindowOptions {
    title: "GadTools menu",
    idcmp: idcmp::CLOSEWINDOW | idcmp::MENUPICK,
    ..Default::default()
})?;
win.set_menu(&strip)?;
```

- `MenuStrip::new(&screen, &entries)` lays the menu out for that screen's
  font and pens. Make it before the window: the window *borrows* the
  strip, so it must outlive the window.
- Picks arrive as `Event::Menu { id, checked }` with the entry's id.
- `win.set_menu_checked(id, on)` changes a toggle's check mark.
- `win.clear_menu()` takes the strip off; it is also taken off when the
  window closes.

## Pop-up menus: `popup::PopupMenu`

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["popupmenu"] }
```

A pop-up menu is built from the same `menu::Item`s (actions, toggles,
separators, submenus). `open(window)` shows it at the mouse pointer over
an open window and **waits** until the user picks an item or lets go. It
returns `Some(id)` for a pick and `None` otherwise.

```rust
use amigaos4::intuition::{code, Event};
use amigaos4::menu::Item;
use amigaos4::popup::PopupMenu;

let pop = PopupMenu::new(&[
    Item::action(201, "Cut"),
    Item::action(202, "Copy"),
    Item::separator(),
    Item::submenu("More", vec![Item::action(203, "Paste")]),
])?;

// In a window opened with rmb_trap: true and idcmp::MOUSEBUTTONS:
if let Some(Event::Button { code: code::MENUDOWN, .. }) = msg.map(|m| m.event) {
    match pop.open(win.as_ptr()) {
        Some(id) => println!("picked {}", id),
        None => {}
    }
}
```

- `PopupMenu::new` needs at least one item.
- `open` takes the raw window pointer: `win.as_ptr()` of an
  `intuition::Window` or a `reaction::Window`. A null pointer (a ReAction
  window while iconified) returns `None` at once.
- Toggle and radio items show a check mark, but a radio item excludes
  nothing here, and the menu's state is not read back.
- The menu and its texts are kept until it is dropped.

## Limitations

- BOOPSI menus need Intuition V54; on an older system building them fails
  with `Unsupported("menuclass refused an object (Intuition V54 needed)")`.
- At most 64 items of one multiple selection are reported.
- A radio group excludes only items in the same panel (the same menu or
  submenu), and only among its first 32 items.
