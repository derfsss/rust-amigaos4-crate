# Standard requesters: `asl`

`asl.library` provides the standard AmigaOS dialogues for choosing a
file, a font or a screen mode. Every Amiga program uses the same ones,
so users know them. The `asl` module wraps them:

| Type | Lets the user choose |
|---|---|
| `FileRequester` | a file (or several, or a drawer) to open or save |
| `FontRequester` | a font and size |
| `ScreenModeRequester` | a display mode, and optionally its size and depth |

## Cargo feature

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["asl"] }
```

`asl` pulls in `dos` and `intuition`. It works in `std` programs and in
programs with no C runtime. The crate opens asl.library itself.

## How they behave

- Make a requester once with `new()` and keep it. Each `request(...)`
  call shows it with that call's options. Because the same requester is
  reused, it remembers the drawer, the file, and its own place and size
  between calls, as Amiga programs are expected to.
- `request` **blocks** until the user answers.
- `parent: Option<&dyn ParentWindow>`: pass `Some(&win)` (an
  `intuition::Window` or a `reaction::Window`) to open the requester on
  that window's screen and put the window to sleep meanwhile (a busy
  pointer, no input). Pass `None` to open it on the default public screen,
  or on `Common::pub_screen`.
- The result is `Result<Option<...>>`:
  - `Ok(Some(choice))`: the user chose.
  - `Ok(None)`: the user pressed **Cancel** (or Esc). asl.library also
    reports a requester it could not open this way, so the two cannot be
    told apart.
  - `Err(..)`: something failed before the requester was shown, such as a
    string the crate could not pass.
- The answers are copied into Rust values; nothing points into
  asl.library's memory afterwards.
- The requester is freed when dropped.

## Options every requester takes: `Common`

| Field | Meaning |
|---|---|
| `title: Option<&str>` | the requester window's title |
| `pub_screen: Option<&str>` | the public screen to open on when there is no parent window |
| `bounds: Option<(i16, i16, i16, i16)>` | left, top, width, height (default: asl's own, or where the user left it) |

All option structs implement `Default`.

## Choosing a file: `FileRequester`

```rust
use amigaos4::asl::{Common, FileOptions, FileRequester};

let mut fr = FileRequester::new()?;
let answer = fr.request(Some(&win), &FileOptions {
    common: Common { title: Some("Open a picture"), ..Default::default() },
    pattern: Some("#?.(png|jpg|iff)"),
    ok_text: Some("Open"),
    reject_icons: true,
    ..Default::default()
})?;
match answer {
    Some(choice) if !choice.file.is_empty() => println!("open {}", choice.paths[0]),
    Some(_) => println!("no file chosen"),
    None => println!("cancelled"),
}
```

`FileOptions`:

| Field | Meaning |
|---|---|
| `common` | see above |
| `drawer: Option<&str>` | the drawer to start in (default: the one chosen last) |
| `file: Option<&str>` | the file name to start with |
| `pattern: Option<&str>` | an AmigaDOS pattern of files to show, such as `#?.txt`, with a pattern field the user can change |
| `ok_text: Option<&str>` | the OK button's text |
| `save: bool` | a save requester: a name that does not exist yet may be typed |
| `multi_select: bool` | several files may be chosen |
| `drawers_only: bool` | only drawers are shown: choose a drawer |
| `reject_icons: bool` | `.info` files (icons) are not shown |

The answer is a `FileChoice`:

| Field | Contents |
|---|---|
| `drawer: String` | the drawer, as typed or chosen (may be relative or empty) |
| `file: String` | the file name (empty when only a drawer was chosen) |
| `paths: Vec<String>` | full paths: the drawer joined with each file chosen. One entry normally, several with `multi_select`, the drawer alone when no file was chosen |

A save requester:

```rust
let answer = fr.request(Some(&win), &FileOptions {
    common: Common { title: Some("Save as"), ..Default::default() },
    file: Some("Untitled.txt"),
    save: true,
    ..Default::default()
})?;
if let Some(choice) = answer {
    if let Some(path) = choice.paths.first() {
        if let Err(e) = std::fs::write(path, b"hello") {   // in a std program
            eprintln!("cannot save {}: {}", path, e);
        }
    }
}
```

## Choosing a font: `FontRequester`

```rust
use amigaos4::asl::{Common, FontOptions, FontRequester};

let mut fo = FontRequester::new()?;
if let Some(font) = fo.request(Some(&win), &FontOptions {
    common: Common { title: Some("Text font"), ..Default::default() },
    name: Some("topaz.font"),
    size: Some(8),
    ..Default::default()
})? {
    println!("{} size {}", font.name, font.size);
}
```

`FontOptions`: `common`, `name` (the font to start with, such as
`"DejaVu Sans.font"`), `size`, `min_height` and `max_height` (the sizes
offered; asl's default is 5 to 24), `fixed_width_only`.

The answer, `FontChoice`, has `name: String`, `size: u16`, `style: u8`
and `flags: u8`. Open the font with
`graphics::Font::open(&font.name, font.size)`.

## Choosing a screen mode: `ScreenModeRequester`

```rust
use amigaos4::asl::{Common, ScreenModeOptions, ScreenModeRequester};

let mut sm = ScreenModeRequester::new()?;
if let Some(mode) = sm.request(None, &ScreenModeOptions {
    common: Common { title: Some("Display mode"), ..Default::default() },
    do_width: true,
    do_height: true,
    ..Default::default()
})? {
    println!("mode {:#x}, {}x{}", mode.display_id, mode.width, mode.height);
}
```

`ScreenModeOptions`: `common`, `display_id` (the mode to start with),
`do_width`, `do_height`, `do_depth` (show those fields too).

The answer, `ScreenMode`, has `display_id: u32`, `width: u32`,
`height: u32`, `depth: u16`, `overscan: u16` and `auto_scroll: bool`.

## Keys

In the file requester, Return in the name field answers OK and Esc
cancels. The font requester may ignore Return while it is still scanning
the fonts; click OK then.

## Limitations

Not wrapped: aborting or activating a requester from another task,
`AslControl`, filter hooks, and the obsolete pre-V36 calls.
