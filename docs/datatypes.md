# datatypes: what a file is, and reading pictures and text

## What it is for

*Datatypes* are AmigaOS's system-wide file format handlers. Each installed datatype (a descriptor in `DEVS:DataTypes` and a class in `CLASSES:DataTypes`) knows one format, such as PNG, JPEG, GIF, BMP, ILBM or plain text. A program that loads files through datatypes.library reads every format the user has installed, without containing any format code itself.

The `datatypes` module uses datatypes.library without a window. It can:

- tell you what a file (or a block of bytes) is: `examine`, `examine_memory`;
- list every installed datatype: `list_all`;
- load a file or bytes as an object: `DtObject`;
- read a loaded picture's pixels as RGB, RGBA or ARGB bytes: `DtObject::read_pixels`;
- get a text file's contents: `DtObject::text`;
- make a picture from your own pixels and save it as IFF: `DtObject::new_picture`, `DtObject::save_iff`.

*IFF* (Interchange File Format) is the traditional Amiga file format family. A picture saved as IFF is an *ILBM* file.

The crate's picture viewer, `apps/imgview`, loads every picture through this module.

## Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", default-features = false, features = ["datatypes", "intuition", "graphics"] }
```

| Feature | Pulls in | Needed for |
|---|---|---|
| `datatypes` | `dos` | The module: `examine`, `examine_memory`, `list_all`, `dt_string`, `DtObject`, attributes, `text`, `save_iff`. |
| `intuition` | nothing | `DtObject::read_pixels`. |
| `graphics` | nothing | `DtObject::new_picture`. |

## Build modes

- **No-C-runtime program**: tested by `testprogs/libtest` (listing, examining files and memory, text objects, reading pixels from a BMP file and from memory, making a picture, saving it as IFF and reading it back the same) and used by `apps/imgview`.
- **std program**: compiles; not exercised by the crate's test programs.

## Calls need a Process

Every call in this module needs a DOS *Process* (a task that DOS knows about, such as a program started from a Shell or from Workbench), because datatypes read files. From a plain exec task the calls return `AmigaError::WrongTask` (`dt_string` is the exception: it reads no file). See `dos.md`.

## What is this file?

```rust
use amigaos4::datatypes::{self, group};
use amigaos4::error::Result;

fn what_is_it() -> Result<()> {
    let info = datatypes::examine("S:Startup-Sequence")?;
    println!("{} ({}), group {}", info.name, info.base_name, info.group_str()); // e.g. "ascii ... text"
    if info.group == group::PICTURE {
        // a picture
    }
    for d in datatypes::list_all()? {
        println!("{:<12} {} {}", d.name, d.group_str(), d.pattern);
    }
    Ok(())
}
```

- `examine(path)` returns a `DataTypeInfo` for a file. A missing file gives `AmigaError::DosError(205)` (object not found).
- `examine_memory(&bytes)` does the same for bytes in memory.
- `list_all()` returns every installed datatype, in the system's order.

`DataTypeInfo` is a copy, so it can be kept:

| Field | Meaning |
|---|---|
| `name` | The datatype's name, such as `"ILBM"` or `"ascii"`. |
| `base_name` | The class's base name (the `<base>.datatype` file). |
| `pattern` | The file name pattern it matches. |
| `group` | The group as four bytes; compare with the `group` constants. `group_str()` gives it as text. |
| `id` | The datatype's id (an IFF FORM type), four bytes. |
| `priority` | Its priority. |

The `group` constants: `SYSTEM`, `TEXT`, `DOCUMENT`, `SOUND`, `INSTRUMENT`, `MUSIC`, `PICTURE`, `ANIMATION`, `MOVIE`.

## `DtObject`: loading a file

`DtObject::from_file(path, &options)` loads a file. `DtObject::from_memory(&bytes, &options)` loads bytes; the bytes stay borrowed for as long as the object lives. The object is disposed of when dropped.

`DtOptions` has one field, `group`. Set it to refuse anything not in that group: asking for a picture from a text file gives `AmigaError::DosError(212)` (object of the wrong type).

Pictures are loaded in true colour and are not remapped to a screen's palette, so you get the picture's own colours.

### Reading a picture's pixels

This is how `apps/imgview` loads a picture:

```rust
use amigaos4::datatypes::{group, DtObject, DtOptions, PixelFormat};
use amigaos4::error::{AmigaError, Result};

fn load_argb(path: &str) -> Result<(usize, usize, Vec<u8>)> {
    let o = DtObject::from_file(path, &DtOptions { group: Some(group::PICTURE) })?;
    let h = o.bitmap_header().ok_or(AmigaError::InvalidArgument("no size"))?;
    let (w, ht) = (h.width as usize, h.height as usize);
    let mut argb = vec![0u8; w * ht * 4];
    o.read_pixels(PixelFormat::Argb, &mut argb, w * 4)?; // needs feature "intuition"
    Ok((w, ht, argb))
}
```

- `bitmap_header()` returns the picture's `BitMapHeader` (a copy): `width`, `height`, `depth` and the other IFF header fields. `None` for an object that is not a picture.
- `read_pixels(format, &mut buffer, bytes_per_row)` reads the whole picture. `PixelFormat` is `Rgb` (3 bytes per pixel), `Rgba` or `Argb` (4 bytes); `bytes_per_pixel()` tells you which. A buffer or row length too small for the picture is refused with an error.

### Other things you can read

| Method | Gives |
|---|---|
| `data_type()` | The object's datatype as a `DataTypeInfo`. |
| `text()` | A text object's contents, copied (`None` for other kinds). |
| `methods()` | The method numbers the object supports. `supports(method)` checks one; the `method` module names a few (`WRITE`, `PDTM_READPIXELARRAY`, ...). |
| `get_u32(tag)` | Any 32-bit attribute, by tag. The `attr` module names common ones (`NOMINAL_HORIZ`, `NOMINAL_VERT`, `PDTA_NUM_COLORS`, ...). `None` if the object does not have it. |
| `as_ptr()` | The raw object pointer, for calls the crate does not wrap. Valid while the `DtObject` lives. |

```rust
use amigaos4::datatypes::{method, DtObject, DtOptions};
use amigaos4::error::Result;

fn read_text() -> Result<()> {
    let o = DtObject::from_file("S:Startup-Sequence", &DtOptions::default())?;
    if let Some(bytes) = o.text() {
        // the file's text
    }
    let can_save = o.supports(method::WRITE);
    Ok(())
}
```

## Making and saving a picture

`DtObject::new_picture(width, height, format, &pixels, bytes_per_row)` makes a 24-bit picture object from your pixels. `save_iff(path)` writes any object to a new file in IFF form (ILBM for a picture). A file left behind by a failed write is deleted.

```rust
use amigaos4::datatypes::{DtObject, PixelFormat};
use amigaos4::error::Result;

fn save_gradient() -> Result<()> {
    const W: usize = 32;
    const H: usize = 16;
    let mut rgb = vec![0u8; W * H * 3];
    for y in 0..H {
        for x in 0..W {
            rgb[(y * W + x) * 3..][..3].copy_from_slice(&[(x * 8) as u8, (y * 16) as u8, 128]);
        }
    }
    let pic = DtObject::new_picture(W as u16, H as u16, PixelFormat::Rgb, &rgb, W * 3)?; // needs feature "graphics"
    pic.save_iff("RAM:gradient.iff")?;
    Ok(())
}
```

## Error messages

Datatypes errors are numbers from `DTERROR_UNKNOWN_DATATYPE` (2000) up, returned as `AmigaError::DosError(code)`. `dt_string(code)` gives datatypes.library's text for one; the text may contain `%s` for the file name. Lower numbers are ordinary DOS errors, which `dos::fault` describes. From `apps/imgview`:

```rust
use amigaos4::{datatypes, dos};
use amigaos4::error::AmigaError;

fn reason(e: &AmigaError, name: &str) -> String {
    match e {
        AmigaError::DosError(c) if *c >= datatypes::DTERROR_UNKNOWN_DATATYPE => datatypes::dt_string(*c as u32)
            .map(|s| s.replace("%s", name))
            .unwrap_or_else(|| format!("datatypes error {}", c)),
        AmigaError::DosError(c) => dos::fault(*c),
        other => format!("{}", other),
    }
}
```

## Limitations

From the module's own notes:

- Not wrapped: showing an object in a window (AddDTObject and the rest), printing, writing in a datatype's own format (only IFF is written), tools, ARexx, the clipboard as a source, and setting attributes (SetDTAttrs).
- `save_iff` is tested for pictures (ILBM). Text (FTXT) and sounds (8SVX) follow the classes' convention but are not tested.
- `new_picture` always makes a 24-bit picture. It hands an ARGB bitmap to the picture object rather than writing pixels into it, because the picture datatype ignores pixel writes into a new, empty object.

## See also

- `dos.md`: the Process rule, files, `dos::fault`.
- `workbench.md`: files dropped on your window, to load with `DtObject::from_file`.
- `locale-prefs-application.md`: OpenDoc requests from other programs.
