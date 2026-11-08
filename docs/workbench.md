# workbench: drops, AppIcons, Tools menu items and Workbench start-up

## What it is for

*Workbench* is the AmigaOS desktop: the screen with disk and drawer icons. The `workbench` module lets a program work with it:

| Item | What it does |
|---|---|
| `AppPort` | A message port (mailbox) that receives Workbench's *AppMessages*, copied into `AppEvent` values. |
| `Window::set_app_window` | Makes an Intuition window an *AppWindow*: the user can drop file icons on it. |
| `AppIcon` | An icon of your own on the Workbench backdrop. The user can double-click it or drop icons on it. |
| `AppMenuItem` | An item of your own in Workbench's Tools menu. |
| `WbStartup` | For a program started by double-clicking its icon: the start-up message with the program's own path and any project files. |
| `tool_types`, `set_tool_types` | Read and write an icon's *tool types*: the `NAME=value` settings stored in a `.info` icon file. |

Use it when your program should accept files dragged from Workbench, offer an icon or Tools menu entry while it runs, or be started from an icon and read its settings from the icon.

The crate's picture viewer, `apps/imgview`, uses `WbStartup` and tool types: started from its icon, it reads `FIT`, `SMOOTH` and `PUBSCREEN=name` from the icon and opens the project file it was started with.

## Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", default-features = false, features = ["workbench", "icon", "intuition"] }
```

| Feature | Pulls in | Needed for |
|---|---|---|
| `workbench` | `dos` | The module: `AppPort`, `AppEvent`, `AppMenuItem`, `WbStartup`. |
| `icon` | nothing | `AppIcon`, `tool_types`, `set_tool_types`, `WbStartup::tool_types` (icon.library). |
| `intuition` | nothing | `Window::set_app_window` and `Window::clear_app_window` (in the `intuition` module). |

The `reaction` feature also turns on `workbench`. A ReAction window (the `reaction` module) handles drops by itself and reports them as its own `Event::Dropped`; you do not need an `AppPort` for it.

## Build modes

- **No-C-runtime program**: tested by `testprogs/libtest` (AppWindow, AppIcon and AppMenuItem added and removed, events from drops and from opening the AppIcon, tool types written and read back, `WbStartup::take` from a Shell). `WbStartup` from Workbench and tool types are used by `apps/imgview`.
- **std program**: compiles; not exercised by the crate's test programs. See the note on `WbStartup` below.

## Rule: the port outlives the objects

`AppIcon`, `AppMenuItem` and an AppWindow all *borrow* the `AppPort` they send to, so the compiler makes sure the port lives longer than they do. When the port is dropped it replies to any messages still queued.

## `AppPort` and `AppEvent`

`AppPort::new()` makes the port. `next()` takes one message, copies it into an `AppEvent` (the files become paths) and replies to it before returning, or returns `None` if nothing is queued. `signal_mask()` gives the signal bit to wait on. A *signal* is one of a task's 32 wake-up bits; a port raises its bit when a message arrives.

`AppEvent` fields:

| Field | Meaning |
|---|---|
| `kind` | `AppKind::Window`, `Icon`, `MenuItem`, `WindowZone`, or `Other(u16)`. |
| `id` | The id you gave when adding the object, so one port can serve several objects. |
| `class` | For an AppIcon: what was done with it (0 means opened). |
| `x`, `y` | The pointer position (for an AppWindow, inside the window). |
| `files` | The files dropped or selected, as paths. A disk or volume is given as `"Name:"`. |

## AppWindow: files dropped on a window

```rust
use amigaos4::error::Result;
use amigaos4::intuition::{idcmp, Event, PubScreen, Window, WindowOptions};
use amigaos4::workbench::{AppKind, AppPort};

fn drop_target() -> Result<()> {
    let port = AppPort::new()?;
    let screen = PubScreen::default_screen()?;
    let mut win = Window::open(&screen, &WindowOptions {
        title: "Drop files here",
        inner_width: 300,
        inner_height: 150,
        idcmp: idcmp::CLOSEWINDOW,
        ..Default::default()
    })?;
    win.set_app_window(&port, 1)?;

    let mut done = false;
    while !done {
        // Wait for the window's own messages or the AppPort's.
        let (msg, _) = win.wait(port.signal_mask());
        if let Some(m) = msg {
            done = m.event == Event::CloseWindow;
        }
        while let Some(ev) = port.next() {
            if ev.kind == AppKind::Window {
                for f in &ev.files {
                    println!("dropped {} at {},{}", f, ev.x, ev.y);
                }
            }
        }
    }
    Ok(())
}
```

`set_app_window(&port, id)` replaces any AppWindow the window already had. The window removes it before it closes, or you can call `clear_app_window()` yourself.

## AppIcon and AppMenuItem

```rust
use amigaos4::error::Result;
use amigaos4::workbench::{AppIcon, AppKind, AppMenuItem, AppPort};

fn icon_and_menu() -> Result<()> {
    let port = AppPort::new()?;
    let _icon = AppIcon::add(&port, 2, "My icon")?;          // needs feature "icon"
    let _item = AppMenuItem::add(&port, 3, "Do my thing")?;
    loop {
        amigaos4::signal::wait(port.signal_mask());
        while let Some(ev) = port.next() {
            match ev.kind {
                AppKind::Icon if ev.files.is_empty() => println!("icon opened"),
                AppKind::Icon => println!("dropped on the icon: {:?}", ev.files),
                AppKind::MenuItem => println!("Tools menu item chosen: {:?}", ev.files),
                _ => {}
            }
        }
    }
}
```

- `AppIcon::add(&port, id, label)` puts an icon on the Workbench backdrop with the system's default tool icon and your label. It is removed when dropped.
- `AppMenuItem::add(&port, id, label)` adds `label` to Workbench's Tools menu. The event's `files` are the icons selected when the item was chosen. It is removed when dropped.

## `WbStartup`: programs started from Workbench

When the user double-clicks a program's icon, Workbench starts it and sends it a *start-up message* (in C, `struct WBStartup`). It names the program's own icon and any *project icons* (documents) the user selected with it, or whose default tool is this program. The program must reply to the message when it ends; that tells Workbench the program has finished.

```rust
use amigaos4::workbench::WbStartup;

fn start() {
    // Call this first thing: from Workbench it waits for the message.
    let wb = WbStartup::take();
    if let Some(w) = &wb {
        let files = w.files();               // project files, without the tool itself
        for t in w.tool_types() {            // needs feature "icon"
            let (key, value) = t.split_once('=').unwrap_or((&t, ""));
            // e.g. "PUBSCREEN=Workbench" or "FIT"
        }
    }
    // ... run the program ...
    drop(wb); // replied here; keep it alive until the very end
}
```

| Method | Gives |
|---|---|
| `WbStartup::take()` | The message, or `None` when started from a Shell (and when called a second time, or from a task that is not a process). |
| `args()` | Every path: the tool itself first, then each project icon. An icon with no file name (a drawer or disk) gives that drawer's or disk's path. |
| `files()` | `args()` without the tool. |
| `tool_window()` | The icon's `TOOLWINDOW` tool type, if set. |
| `tool_types()` | The tool icon's tool types (with `icon`); empty when it has no icon. |
| `code_handed_to_dos()` | `true` when DOS frees the program's code after it ends, `false` when Workbench does after the reply. |

The message is replied when the `WbStartup` is dropped. Keep it until your program is about to return.

**Why no Forbid.** The classic Workbench unloads a program's code as soon as it gets the reply, while the program is still returning through that code. C start-up code avoids this by replying under `Forbid` (turning off multitasking). This crate does not use `Forbid`. The AmigaOS 4.1 Final Edition Workbench needs none: it starts the program so that DOS frees its code once the process has ended. For a starter that does ask to unload the code, `take` hands the code over to DOS instead when it can. `code_handed_to_dos()` tells you which happened.

**std programs.** `WbStartup` is written for programs that do their own start-up (no-C-runtime programs such as `apps/imgview`). In a std program the C library's start-up code runs before `main`; whether it has already taken the message is not covered by the crate's tests, so treat `WbStartup` in a std program as untested.

## Tool types

An icon (`name.info`) can hold a list of text settings called tool types. Each is `NAME=value` or just `NAME`. An entry in brackets, such as `(DONOTWAIT)`, is switched off by convention.

```rust
use amigaos4::error::Result;
use amigaos4::workbench::{set_tool_types, tool_types};

fn icon_settings() -> Result<()> {
    let current = tool_types("Work:MyProgram");   // reads Work:MyProgram.info; empty if none
    set_tool_types("Work:MyProgram", &["FIT", "PUBSCREEN=Workbench", "(DONOTWAIT)"])?;
    Ok(())
}
```

- `tool_types(path)` reads the tool types of `path`'s icon. The path is the file without `.info`.
- `set_tool_types(path, &[...])` writes `path.info` with these tool types. It uses the file's own icon if it has one, otherwise the default icon for that kind of file. It replaces the whole list.

Both need the `icon` feature.

## Limitations

- `AppIcon` always uses the default tool icon image; there is no way to give your own image.
- Choosing an `AppMenuItem` has not been tested (adding and removing it has).
- `WbStartup` in a std program is untested (see above).

## See also

- `dos.md`: paths, locks and the Process rule; `dos::args` for programs started from a Shell.
- `locale-prefs-application.md`: application.library, which also delivers OpenDoc requests and Quit from the dock.
- `datatypes.md`: loading the files dropped on your window.
