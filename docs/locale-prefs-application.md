# locale, prefs and application: language, settings and talking to other programs

This page covers three modules that most desktop programs want sooner or later:

| Module | Library | What it gives you |
|---|---|---|
| `locale` | locale.library | The user's language and country settings: day and month names, character tests, comparing and sorting text, dates as text and back, formatted numbers, and translated strings from catalogs. |
| `prefs` | application.library (its PrefsObjects part) | Settings kept as a dictionary of strings, numbers, true/false values, lists and nested dictionaries, saved to and loaded from an XML file. |
| `application` | application.library | Registering your program as an *application*, so the system and other programs can ask it to quit, come to the front, open a document, and so on. |

A *library* on AmigaOS is a shared component of the system, opened by name at run time (for example `locale.library`). Each safe module in this crate opens the library it needs the first time you use it. You do not open it yourself.

All three modules come together in one place in the crate's picture viewer, `apps/imgview`: it registers as an application, keeps its window position in a prefs file, and reacts to Quit and OpenDoc requests.

---

## locale

### What it is for

A *locale* is the set of language and country preferences the user picked in the Locale preferences editor: the language, the character set, the time zone offset, and how dates and numbers are written. Use the `locale` module when your program shows dates, sorts names, or wants to speak the user's language.

A *catalog* is a file of translated strings for one program and one language (for example `LOCALE:Catalogs/deutsch/myprogram.catalog`). Your program keeps its own built-in strings (usually English); when the user's language has a catalog, the translations are used instead.

### Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", default-features = false, features = ["locale", "dos"] }
```

- `locale` turns the module on. It pulls in no other feature.
- Add `dos` (on by default) as well when you open catalogs. Without `dos`, `Catalog::open` cannot read DOS's error code, so it cannot tell "no catalog file" from a real failure. The examples below also use `dos::DateStamp`.

### Build modes

- **No-C-runtime program**: tested by the crate's test program (`testprogs/libtest`).
- **std program**: compiles; not exercised by the crate's test programs.

### Text is bytes, not always UTF-8

Strings from locale.library are in the locale's *code set* (character set), which you can read with `Locale::code_set()`. The number is the IANA character set number: 4 is ISO-8859-1 (Latin-1), 106 is UTF-8. Because the text may not be UTF-8, most methods return `&[u8]` or `Vec<u8>`. Each one has a `_lossy` partner that returns a Rust `String`, replacing any byte that is not valid UTF-8.

### `Locale`: the user's settings

`Locale::current()` opens the user's current locale. `Locale::open(path)` opens one from a preferences file instead. The locale is closed when the value is dropped.

```rust
use amigaos4::error::Result;
use amigaos4::locale::{id, Locale};

fn show() -> Result<()> {
    let loc = Locale::current()?;
    println!("language {}, code set {}", loc.language(), loc.code_set());
    if let Some(day) = loc.text_lossy(id::DAY_1) {
        println!("the first day is {}", day); // "Sunday" in English
    }
    Ok(())
}
```

What you can read:

| Method | Gives |
|---|---|
| `language()` | The language name, such as `"english"`. |
| `code_set()` | The character set number (see above). |
| `country_code()` | The country code as four bytes, such as `b"USA\0"`. |
| `gmt_offset()` | Minutes from GMT. |
| `text(n)` / `text_lossy(n)` | A named string: day names (`id::DAY_1` is Sunday, up to `DAY_1 + 6`), short day names (`id::ABDAY_1`), months (`id::MON_1` to `MON_1 + 11`), short months (`id::ABMON_1`), `id::YESSTR`, `id::NOSTR`, `id::AM_STR`, `id::PM_STR`. `None` for an unknown number. |

### Characters, comparing and sorting

```rust
use core::cmp::Ordering;
use amigaos4::error::Result;
use amigaos4::locale::{Class, Locale, Sort};

fn compare() -> Result<()> {
    let loc = Locale::current()?;
    let same = loc.cmp(b"abc", b"ABC", Sort::Ascii)? == Ordering::Equal; // true: case ignored
    let upper = loc.to_upper(b'a' as u32);                                // 'A'
    let digit = loc.is(Class::Digit, b'7' as u32);                        // true

    // Sort names the way the user's language does.
    let mut names: Vec<&[u8]> = vec![b"pear", b"Apple", b"banana"];
    names.sort_by(|a, b| loc.cmp(a, b, Sort::Collate2).unwrap_or(Ordering::Equal));
    Ok(())
}
```

- `is(class, c)` tests a character: `Class::AlNum`, `Alpha`, `Cntrl`, `Digit`, `Graph`, `Lower`, `Print`, `Punct`, `Space`, `Upper`, `XDigit`. Characters are codes in the locale's code set.
- `to_lower(c)` and `to_upper(c)` change case, including accented Latin-1 letters.
- `cmp(a, b, sort)` compares two whole strings; `cmp_n(a, b, n, sort)` compares at most `n` characters (`-1` for all).
- `sort_key(s, sort)` gives a key whose plain byte order is the `sort` order. This is useful when you sort the same list many times.

The `Sort` choices:

| `Sort` | Order |
|---|---|
| `Ascii` | Byte order, case ignored. |
| `Collate1` | The language's order, accents ignored. |
| `Collate2` | The language's order, with accents and case used to break ties. |

### Dates and formatted text

An AmigaOS date is a `DateStamp`: days since 1 January 1978, minutes since midnight, and ticks (1/50 second) within the minute. It lives in the `dos` module (`amigaos4::dos::DateStamp`).

```rust
use amigaos4::dos::DateStamp;
use amigaos4::error::Result;
use amigaos4::locale::{FmtArg, Locale};

fn dates() -> Result<()> {
    let loc = Locale::current()?;
    let day0 = DateStamp { ds_Days: 0, ds_Minute: 0, ds_Tick: 0 };
    let text = loc.format_date_lossy("%Y-%m-%d %H:%M", &day0)?;     // "1978-01-01 00:00"
    let back = loc.parse_date("%Y-%m-%d", b"1979-01-02")?;          // Some(day 366)

    let s = loc.format_lossy("%ld files, %s", &[FmtArg::Int(3), FmtArg::Str(b"all copied")])?;
    let moved = loc.format_lossy("%2$s has %1$ld", &[FmtArg::Int(3), FmtArg::Str(b"RAM:")])?;
    let grouped = loc.format_lossy("%lU bytes", &[FmtArg::Int(1_234_567)])?; // digits grouped the user's way
    Ok(())
}
```

- `format_date(template, &date)` writes a date with `%` codes such as `%Y %m %d %H %M %S %A`. `format_date_lossy` returns a `String`.
- `parse_date(template, input)` reads a date back. It returns `Ok(None)` when the input does not fit the template (a wrong separator, 30 February). Only these codes are allowed in the template: `%a %A %b %B %d %e %h %H %I %m %M %p %S %y %Y`. Any other code is refused with an error before locale.library is called.
- `format(template, args)` is locale.library's FormatString: a printf-like formatter that uses the locale's conventions for `%D` and `%U` (numbers with grouped digits). Each argument is a `FmtArg::Int(i32)` or a `FmtArg::Str(&[u8])`. The commands have the form `%[n$][-][0][width][.limit][l]type` with type `s` (a string) or `c d D u U x X` (a number); `%%` is a percent sign. `n$` picks argument `n`, counting from 1.
- Every command is checked against `args` first, because a wrong one would make the library read past the arguments. Too few arguments, an argument of the wrong kind, or the `h`, `ll`, `b` and `p` forms give an `Err`. Note that `x` gives *uppercase* hex digits and `X` lowercase (the AmigaOS convention, the reverse of C's printf).

### `Catalog`: translated strings

```rust
use amigaos4::error::Result;
use amigaos4::locale::{Catalog, CatalogOptions};

const MSG_HELLO: i32 = 1;

fn greet() -> Result<()> {
    let cat = Catalog::open(
        "myprogram.catalog",
        &CatalogOptions { builtin_language: Some("english"), version: 0, language: None },
    )?;
    let hello = cat.get(MSG_HELLO, c"Hello");
    println!("{}", String::from_utf8_lossy(hello));
    Ok(())
}
```

- `Catalog::open(name, &options)` opens a catalog. A catalog that does not exist is **not** an error: you get a `Catalog` that answers with your built-in strings. `is_loaded()` tells you whether a file was found. Only a real failure gives an `Err`.
- `CatalogOptions` (it implements `Default`): `builtin_language` is the language of your built-in strings (the default is "english"); `version` asks for a catalog version (0 means any); `language` uses a language other than the user's.
- `get(id, builtin)` returns the translation of string `id`, or `builtin` (a C string literal such as `c"Hello"`) when there is none. The result is in the locale's code set.
- Open catalogs from a DOS *Process* (a task that DOS knows about, such as a program started from the Shell or Workbench), because it reads files.

### Limitations

- Text comes back in the locale's code set, not necessarily UTF-8.
- `format` takes 32-bit arguments only.
- `parse_date` accepts only the codes listed above.
- The crate's tests cover a missing catalog (the built-in strings are used). Reading a real translation from a loaded catalog has not been tested.

---

## prefs

### What it is for

The `prefs` module stores settings as *prefs objects*: a dictionary (keys to values) that can hold strings, numbers, true/false values, arrays and further dictionaries. It can write the whole dictionary to a file as UTF-8 XML and read it back.

Use it to remember your program's settings between runs. On AmigaOS, current settings usually go in `ENV:` (kept in memory until the next reboot) and saved settings in `ENVARC:` (kept on disk and copied to `ENV:` at boot). A program writes both when the user saves.

### Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", default-features = false, features = ["prefs_objects"] }
```

- `prefs_objects` turns on the `prefs` module. It pulls in no other feature.
- It needs application.library version 53 or later, prefsobjects interface version 2.

### Build modes

- **No-C-runtime program**: tested by `testprogs/libtest` and used by `apps/imgview`.
- **std program**: compiles; not exercised by the crate's test programs.

### Ownership

Prefs objects are reference counted by the library. The crate hides this:

- `PrefsDict` and `PrefsArray` release their object when dropped.
- A `Value` put into a dictionary or array is *moved* in. The container owns it from then on.
- Reads copy out (`get_str`, `get_int`), or borrow the container (`PrefsDict::array` returns an `ArrayRef` tied to the dictionary).

### Writing settings

```rust
use amigaos4::error::Result;
use amigaos4::prefs::{PrefsArray, PrefsDict, Value};

fn save() -> Result<()> {
    let mut d = PrefsDict::new()?;
    d.set_str("name", "myprogram")?;
    d.set_int("width", 640)?;
    d.set_bool("smooth", true)?;

    let mut recent = PrefsArray::new()?;
    recent.push(Value::Str(String::from("Work:pictures/a.png")))?;
    recent.push(Value::Str(String::from("Work:pictures/b.png")))?;
    d.set("recent", Value::Array(recent))?;

    let mut window = PrefsDict::new()?;
    window.set_int("left", 40)?;
    d.set("window", Value::Dict(window))?;

    d.write("ENV:myprogram.xml")?;
    d.write("ENVARC:myprogram.xml")?;
    Ok(())
}
```

### Reading settings

```rust
use amigaos4::error::Result;
use amigaos4::prefs::{self, PrefsDict};

fn load() -> Result<()> {
    let mut d = PrefsDict::new()?;
    if d.read("ENV:myprogram.xml").is_ok() {
        let width = d.get_int("width").unwrap_or(640);
        let smooth = d.get_bool("smooth", false);
        if let Some(list) = d.array("recent") {
            let files: Vec<String> = prefs::strings(&list);
        }
    }
    Ok(())
}
```

### The types

| Item | What it does |
|---|---|
| `PrefsDict::new()` | An empty dictionary. Keys keep their order. |
| `set(key, Value)` | Puts a value under `key`, replacing (and releasing) an old one. Shortcuts: `set_str`, `set_int`, `set_bool`. |
| `get_str(key)`, `get_int(key)` | A copy of the value, or `None` if the key is missing **or holds another type**. |
| `get_bool(key, default)` | The true/false value, or `default`. |
| `array(key)` | The array under `key` as an `ArrayRef`, borrowed from the dictionary. |
| `has_dict(key)` | Whether the value under `key` is a dictionary. |
| `remove(key)` | Removes the key; an error if it is not there. |
| `len()`, `is_empty()` | The number of keys. |
| `write(path)` | Writes the dictionary as XML to `path`. |
| `read(path)` | Clears the dictionary, then fills it from the file at `path`. |
| `PrefsArray::new()`, `push(Value)`, `len()`, `is_empty()` | A list to build and put into a dictionary. |
| `ArrayRef`: `len()`, `is_empty()`, `get_str(i)`, `get_int(i)` | Reading a borrowed array. |
| `prefs::strings(&array_ref)` | Every string item of an array as a `Vec<String>`. |
| `Value` | `Str(String)`, `Int(i32)`, `Bool(bool)`, `Array(PrefsArray)`, `Dict(PrefsDict)`. |

### Limitations

- **The file name is an ordinary path.** A bare name such as `"myprogram.xml"` lands in the current directory, *not* in `ENV:`. Always give the full path.
- Nested dictionaries can be written, and `has_dict` tells you one is there, but there is no method to read the values inside a nested dictionary. Keep settings you need to read back at the top level, or in arrays.
- Not wrapped: dates, binary data, floating-point numbers, and reading or writing through callbacks.

---

## application

### What it is for

Registering with application.library makes your program known to the system as an *application*. Once registered, it gets a message port (a mailbox for messages from other tasks) on which the system, AmiDock (the AmigaOS dock bar) and other programs can send requests: quit, hide, come to the front, open a document, reload preferences, and more. You can also find other applications and send requests to them.

Use it in any program with a window that the user may want to quit from the dock, or that should open files sent to it.

### Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", default-features = false, features = ["application"] }
```

- `application` turns the module on. It pulls in no other feature.
- It needs application.library version 53 or later, application interface version 2.

### Build modes

- **No-C-runtime program**: tested by `testprogs/libtest` and used by `apps/imgview`.
- **std program**: compiles; not exercised by the crate's test programs.

### Registering and receiving events

`Application::register(name, &options)` registers the calling task. Dropping the `Application` unregisters it (after replying anything still queued).

`next()` takes one message off the port, copies it into an `Event` and replies to it before returning. Your program never holds a library message. `wait(other_signals)` blocks until a message arrives or one of `other_signals` is raised.

```rust
use amigaos4::application::{Application, Event, RegisterOptions};
use amigaos4::error::Result;

fn run() -> Result<()> {
    let app = Application::register(
        "MyProgram",
        &RegisterOptions { url_identifier: Some("example.org"), description: Some("An example"), ..Default::default() },
    )?;
    let mut quit = false;
    while !quit {
        let (got, _others) = app.wait(0);
        if let Some((_from, ev)) = got {
            match ev {
                Event::Quit | Event::ForceQuit => quit = true,
                Event::OpenDoc(path) => println!("open {}", path),
                _ => {}
            }
        }
    }
    Ok(())
}
```

A program with a window waits on both at once. Each AmigaOS task has 32 *signal* bits; a port raises one when a message arrives. Combine the masks and wait on them together:

```rust
let mask = app.signal_mask() | win.signal_mask();
amigaos4::signal::wait(mask);
while let Some((from, ev)) = app.next() {
    // handle ev
}
// ... then the window's messages
```

`RegisterOptions` (it implements `Default`):

| Field | Meaning |
|---|---|
| `url_identifier` | A reverse-domain style suffix that makes the identifier unique, such as `"example.org"`. |
| `unique` | Refuse a second copy. The second `register` fails with `AmigaError::Exists`, and the first copy gets `Event::Unique`. |
| `no_icon` | No icon in the dock. |
| `hidden` | Register as hidden. |
| `description` | A line describing the program (application.library 53.2 or later). |

`Event` values include `Quit`, `ForceQuit`, `Hide`, `Unhide`, `ToFront`, `Unique`, `OpenPrefs`, `ReloadPrefs`, `NewBlankDoc`, `OpenDoc(String)`, `PrintDoc(String)`, `Custom(String)`, the system notices (`AppRegister`, `AppUnregister`, `BlankerAllow`, `BlankerDisallow`, `ConfirmClose`, `GameModeEntered`, `GameModeLeft` and others) and `Other(u32)` for a type the crate does not name. `next` returns the sender's `AppId` with each event.

### Finding and asking other applications

```rust
use amigaos4::application::{self, AppId, Application, RegisterOptions, Request};
use amigaos4::error::Result;

fn remote() -> Result<()> {
    let me = Application::register("MyRemote", &RegisterOptions { no_icon: true, ..Default::default() })?;
    if let Some(viewer) = application::find("imgview")? {
        me.send(viewer, Request::OpenDoc("Work:pictures/a.png"))?;
        me.send(viewer, Request::ToFront)?;
    }
    for (id, name) in application::list()? {
        println!("{:>5} {}", id.0, name);
    }
    me.send(AppId::ALL, Request::Custom("hello"))?; // to every application
    Ok(())
}
```

| Item | What it does |
|---|---|
| `Application::id()` | Your `AppId`. IDs are never reused while the system runs. |
| `Application::identifier()` | The identifier the library gave you (name, a counter, the URL identifier). |
| `Application::send(to, Request)` | Sends a request. `Request`: `Quit`, `ForceQuit`, `Hide`, `Unhide`, `ToFront`, `OpenPrefs`, `ReloadPrefs`, `NewBlankDoc`, `OpenDoc(&str)`, `PrintDoc(&str)`, `Custom(&str)`. The library copies any text, so it need not outlive the call. |
| `AppId::ALL` | Every application (a broadcast). |
| `application::find(name)` | The `AppId` of a registered name, or `None`. |
| `application::find_identifier(id)` | The same, by identifier. |
| `application::list()` | Every registered application as `(AppId, name)` pairs, copied. |

### Limitations

Not wrapped: notification pop-ups, icon locking, the global application settings, and the Workbench start-up tags.

---

## See also

- `dos.md`: `DateStamp`, paths, files and the Process rule.
- `utility.md`: dates as calendar fields, case-insensitive text helpers.
- `workbench.md`: drops on windows, AppIcons, programs started from Workbench, tool types.
- `datatypes.md`: loading pictures and other files.
