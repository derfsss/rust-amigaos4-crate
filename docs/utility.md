# Utility: dates, hashes, text, tag lists and named objects

The `utility` module is a safe layer over **utility.library**, a small
library in the AmigaOS Kickstart (the part of the system in ROM or loaded
at boot) that many other libraries build on.

Use it when you want to:

- turn Amiga time (seconds since 1 January 1978) into calendar fields and
  back, with checks for illegal dates;
- get numbers that are unique across the whole system;
- compute a SHA-1 digest, or a simple repeatable random sequence;
- compare text without case the way the user's language does, or with
  fixed Unicode rules;
- inspect or change **tag lists** safely;
- share an object between programs by name (**named objects**);
- sort an exec list (`NameList::sort_by`).

## Cargo features

`utility` is a default feature, so a std program has it already:

```toml
[dependencies]
amigaos4 = { path = "...", features = ["utility"] }
```

A program without the C runtime names it:

```toml
[dependencies]
amigaos4 = { path = "...", default-features = false, features = ["utility"] }
```

`utility` pulls in no other feature. `bsdsocket` and `sana2` pull it in.

## Build modes

| Mode | Works | Notes |
|---|---|---|
| std program (`fn main` on clib4) | Compiles; uses the C library's own `IUtility`. | Not exercised by the crate's test programs. |
| no-C-runtime program (`#![no_std]`) | Yes | The crate's library test program runs every part of this module this way. |

Unlike most of `dos`, these calls work in any exec task, not only in a DOS
Process.

## Getting the interface: `Utility`

Everything hangs off a `Utility` value:

```rust
use amigaos4::utility::Utility;

let u = Utility::get()?;
let (version, revision) = u.version();
```

`Utility::get()` uses the program's `IUtility` interface if it has one,
otherwise the crate opens utility.library once. It only fails if the
library cannot be opened, which should not happen (it is in the
Kickstart). `Utility` is small and `Copy`, and may be sent to other tasks.

Some calls need utility.library version 54 (the UTF-8 and UCS-4 ones) or
54.2 (`NameList::sort_by`). On an older library they return
`Err(AmigaError::Unsupported(..))`.

## Dates

AmigaOS counts time in seconds since 1978-01-01 00:00:00 (a Sunday). A
`ClockData` (from `amigaos4::sys`) holds the calendar fields: `sec`, `min`,
`hour`, `mday`, `month`, `year`, and `wday` (0 = Sunday).

| Call | What it does |
|---|---|
| `u.amiga_to_date(seconds)` | Seconds to a `ClockData`. No time-zone change. |
| `u.date_to_amiga(&date)` | Checks the date, then gives its seconds, or `None` for an illegal date (31 February, hour 24, before 1978, after 2114 ...). |
| `u.date_to_amiga_unchecked(&date)` | No check; for a date known to be legal. An illegal one gives a meaningless number, never undefined behaviour. |
| `utility::EPOCH` | 1978-01-01 00:00:00 as a `ClockData`. |

```rust
use amigaos4::sys::ClockData;
use amigaos4::utility::{Utility, EPOCH};

let u = Utility::get()?;
let d = u.amiga_to_date(86400 + 3661);           // 1978-01-02 01:01:01
assert_eq!((d.year, d.month, d.mday), (1978, 1, 2));

let feb31 = ClockData { sec: 0, min: 0, hour: 0, mday: 31, month: 2, year: 1990, wday: 0 };
assert_eq!(u.date_to_amiga(&feb31), None);
assert_eq!(u.date_to_amiga(&EPOCH), Some(0));
```

`date_to_amiga` does not check `wday`: this is a documented bug in the
library's CheckDate. For the current time and DOS's own date format, see
`DateStamp` in [dos.md](dos.md).

## Unique numbers, SHA-1 and random numbers

- `u.unique_id()` gives a number no other call returns, system-wide, from
  any task (until 2^32 calls).
- `Sha1` computes a 160-bit SHA-1 digest. It is **not for cryptography**.
  Feed it with `update`, end with `finish`, or use `Sha1::digest` for one
  piece of data. A `Sha1` can be cloned to carry on from the same state.
- `Random` is the library's simple generator: values in 1 to 2^31 - 1 from
  a state you own. The same seed always gives the same sequence. Also
  **not for cryptography**. A seed of two zeros is refused.

```rust
use amigaos4::utility::{Random, Sha1, Utility};

let u = Utility::get()?;
let id = u.unique_id();

let digest: [u8; 20] = Sha1::digest(u, b"abc");
let mut h = Sha1::new(u);
h.update(b"ab");
h.update(b"c");
assert_eq!(h.finish(), digest);

let mut r = Random::new(u, 12345, 678)?;
let roll = r.next_u31() % 6 + 1;
```

## Text: `utility::text`

These are methods on `Utility`. There are two families, with different
rules:

| Family | Methods | Rules |
|---|---|---|
| 8-bit text | `stricmp`, `strnicmp`, `to_lower`, `to_upper` | The system's character set (Latin-1 on an English system). When locale.library is running it replaces these with the user's language rules, so results can differ between machines. Use them where the user's sort order is wanted (names shown in a list); use Rust's ASCII calls where a fixed answer is needed. |
| Unicode | `utf8_stricmp`, `utf8_strnicmp`, `ucs4_to_lower`, `ucs4_to_upper`, `ucs4_valid`, `utf8_decode_lossy` | Unicode simple case folding (one character to one), the same on every machine. Needs version 54. |

Simple folding differs from Rust's full mapping: here `ß` stays `ß`, while
Rust's `to_uppercase` gives `SS`.

`utf8_strnicmp` counts characters, not bytes. `utf8_decode_lossy` decodes
UTF-8 and replaces bad sequences with U+FFFD.

```rust
use amigaos4::utility::Utility;
use core::cmp::Ordering;

let u = Utility::get()?;
assert_eq!(u.stricmp(b"apple", b"Banana")?, Ordering::Less);
assert_eq!(u.utf8_stricmp("Ärger", "äRGER")?, Ordering::Equal);
assert_eq!(u.utf8_strnicmp("ÄÖXa", "äöXb", 3)?, Ordering::Equal);
assert_eq!(u.ucs4_to_upper('é')?, 'É');
assert_eq!(u.ucs4_to_upper('ß')?, 'ß');
let s = u.utf8_decode_lossy(b"caf\xc3\xa9 \xff")?;   // "café \u{FFFD}"
```

The library takes NUL-terminated strings, so each argument is copied with
a NUL added. Text that contains a NUL itself is refused with
`InvalidArgument` rather than cut short.

## Tag lists: `utility::tags`

A **tag list** is how AmigaOS passes optional arguments: an array of
`TagItem { ti_Tag, ti_Data }` pairs ending with `TAG_DONE`. A few special
tags steer the walk: `TAG_IGNORE` (skip this item), `TAG_SKIP` (skip the
next n items) and `TAG_MORE` (continue in another list at this address).
Your own tags start at `TAG_USER`. These types and constants are in
`amigaos4::sys`.

The OS follows a list blindly. A Rust slice that has no `TAG_DONE`, skips
past its end or contains a `TAG_MORE` pointer could send it anywhere, so
this module checks the slice first:

- `TagList::new(u, &items)` checks a borrowed list and refuses those
  cases. `TAG_MORE` chains are not supported in the safe layer.
- `TagListMut::new(u, &mut items)` does the same for a list the library may
  change in place.

| `TagList` method | What it does |
|---|---|
| `find(tag)` | The first item with that tag. |
| `get(tag, default)` | That item's data, or `default`. |
| `iter()` | The items, with `TAG_IGNORE` and skipped ones left out. |
| `pack_bool(initial, &map)` | Build a set of flag bits from boolean tags. |
| `as_slice()` | The checked items, up to and including `TAG_DONE`. |

| `TagListMut` method | What it does |
|---|---|
| `filter(&tags, Filter::Keep / Filter::Drop)` | Keep only, or drop, the tags in a `TAG_DONE`-ended array. Returns how many are left. |
| `map(&map, keep_unknown)` | Change tags to new tags. A map to `TAG_MORE` or `TAG_SKIP` is refused. |
| `apply(&changes)` | Items also in `changes` take its data. |
| `filter_changes(&mut original, apply)` | Ignore items equal to the original; optionally copy the others into it. |
| `as_list()` | A `TagList` view. |

The calls that change a list never shorten it: they turn items into
`TAG_IGNORE` in place.

`u.tag_in_array(tag, &tags)` tells whether a tag is in a `TAG_DONE`-ended
array of tag values.

`TagClone::new(list)` makes the library's own copy of a list (freed on
drop). It borrows the original, so the original cannot change while the
clone lives. `as_mut()` lets you change the clone; `refresh()` puts the
original's data back.

```rust
use amigaos4::sys::{TagItem, TAG_DONE, TAG_USER};
use amigaos4::utility::tags::{Filter, TagClone, TagList, TagListMut};
use amigaos4::utility::Utility;

const MY_WIDTH: u32 = TAG_USER + 1;
const MY_HEIGHT: u32 = TAG_USER + 2;
const MY_DEPTH: u32 = TAG_USER + 3;

let u = Utility::get()?;
let items = [
    TagItem { ti_Tag: MY_WIDTH, ti_Data: 640 },
    TagItem { ti_Tag: MY_HEIGHT, ti_Data: 480 },
    TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
];
let list = TagList::new(u, &items)?;
let w = list.get(MY_WIDTH, 320);   // 640
let d = list.get(MY_DEPTH, 8);     // 8: not in the list

let mut m = [
    TagItem { ti_Tag: MY_WIDTH, ti_Data: 640 },
    TagItem { ti_Tag: MY_HEIGHT, ti_Data: 480 },
    TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
];
let left = TagListMut::new(u, &mut m)?.filter(&[MY_WIDTH, TAG_DONE], Filter::Keep)?;
// left == 1; the height item is now TAG_IGNORE

let mut c = TagClone::new(list)?;
c.as_mut()?.filter(&[MY_WIDTH, TAG_DONE], Filter::Drop)?;
c.refresh(); // back to the original's items
```

To *build* a tag list to pass to an OS call, see the crate's `tag` module
(`TagListBuilder`).

## Named objects: `utility::names`

A **named object** is a block of memory with a name, placed in a
**namespace** where other programs can find it by that name. The system
has one root namespace, and an object can carry a namespace of its own for
nested objects.

| Item | What it is |
|---|---|
| `Named::new(u, name, Options { .. })` | An object not yet in any namespace; freed on drop. |
| `Options` | `user_space` (bytes of memory for your data), `priority`, and for an object with its own namespace: `namespace`, `no_dups` (refuse a second object of the same name), `case` (compare names with case). Implements `Default`. |
| `named.user_space()` | Your data area, as `&mut [u8]`. |
| `named.publish(space)` | Put it into a namespace. Gives a `Published`, or the `Named` back if a `no_dups` namespace already has the name. |
| `Space::root()`, `named.space()` | The root namespace, or the one inside an object. |
| `names::find(u, space, name)` | The first object with that name, as a `Found`. |
| `names::find_all(u, space, name)` | All of them (`None`: every object), in namespace order. |
| `found.object()`, `published.object()` | The address of the user space. |

Names are compared without case by default (by the current locale's
rules), with case when the namespace was made with `case`.

Each `Found` holds one use of the object and releases it on drop, so the
object stays alive while you hold it. Dropping a `Published` takes the
object out of its namespace and frees it; if another task still holds a
`Found`, the drop waits until it lets go.

```rust
use amigaos4::utility::names::{self, Named, Options, Space};
use amigaos4::utility::Utility;

let u = Utility::get()?;
let mut n = Named::new(u, "MyApp.shared", Options { user_space: 64, ..Default::default() })?;
n.user_space()[0] = 1;
let published = match n.publish(Space::root()) {
    Ok(p) => p,
    Err(_n) => return Ok(()), // the namespace refused it
};
if let Some(found) = names::find(u, Space::root(), "myapp.shared")? {
    let ptr = found.object(); // the same user space
}
drop(published); // out of the namespace, then freed
```

Things to watch:

- Dropping a `Published` while **the same task** holds a `Found` of it
  waits for ever, like locking a mutex twice.
- The user space is plain shared memory. Reading or writing it through
  `object()` is an agreement between you and the other programs; the crate
  does not lock it.

## Sorting a list: `NameList::sort_by`

`amigaos4::list::NameList` is an exec list of named nodes (see the crate's
`list` module). With the `utility` feature it gains `sort_by`, which uses
the library's SortList. It needs utility.library 54.2, else
`Err(Unsupported)`.

```rust
use amigaos4::list::NameList;

let mut l = NameList::new(0)?;
for n in ["pear", "apple", "fig"] {
    l.push_back(n, 0)?;
}
l.sort_by(|a, b| a.name().cmp(b.name()))?;
let names: Vec<&str> = l.iter().map(|n| n.name()).collect(); // apple, fig, pear
```

The compare function gets two nodes and may use `name()` and `pri()`. Do not rely on equal nodes keeping their original order: the
crate does not promise it.

## Deliberately left out

- Skip lists and splay trees: use `BTreeMap` or the crate's
  `avl::AvlMap`.
- String copies, `memset`-like and printf-like calls: Rust has its own.
- PackStructureTags / UnpackStructureTags (raw offset tables).
- RemNamedObject outside `names` (the module handles removal itself).

## See also

- [dos.md](dos.md): `DateStamp`, the DOS date format, as text.
- [locale-prefs-application.md](locale-prefs-application.md): the user's
  language, dates and number formats.
- [networking.md](networking.md): `bsdsocket` uses `utility`.
