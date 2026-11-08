# DOS: files, directories, processes and the Shell

The `dos` module is a safe layer over **dos.library**, the part of AmigaOS
that deals with files, directories, disks, the Shell and DOS processes.

Use it when you want to:

- read and write files, list directories, rename, delete, set comments and
  protection bits;
- hold a **lock** on a file or directory (an AmigaOS handle that keeps the
  object in place and lets you ask about it);
- read **variables** (the Shell's own, and global ones in `ENV:`);
- read your program's command line with an AmigaDOS **template**;
- start child **processes**, run Shell commands, load and run programs;
- be told when a file, directory or variable changes;
- list volumes and assigns, and ask how full a disk is.

In a std program you can also use `std::fs`, which works through the C
library. `dos` gives you the AmigaOS view directly: locks, comments,
protection bits, patterns, notification, assigns and so on.

## Cargo features

`dos` is a default feature, so a std program has it already:

```toml
[dependencies]
amigaos4 = { path = "...", features = ["dos"] }
```

A program without the C runtime turns the defaults off and names it:

```toml
[dependencies]
amigaos4 = { path = "...", default-features = false, features = ["dos"] }
```

`dos` pulls in no other feature. Some other features pull in `dos`:
`asl`, `datatypes`, `elf` and `workbench`.

## Build modes

| Mode | Works | Notes |
|---|---|---|
| std program (`fn main` on clib4) | Yes | The program template reads its arguments with `dos::args`; the std panic test uses `fs::File` and `fs::Lock`. |
| no-C-runtime program (`#![no_std]`) | Yes | The crate's library test program runs every part of this module this way. |

The module opens dos.library itself if the program has not.

## The one rule: DOS calls need a Process

AmigaOS has two kinds of running code:

- an **exec task**: the basic unit the kernel schedules;
- a **DOS Process**: a task with extra DOS state (a current directory,
  standard input and output, an error code, local variables).

A program started from the Shell or from Workbench is a Process. A task
made with `amigaos4::task::spawn` is not. Most dos.library calls need the
Process state, so the crate checks first. Called from a plain task, they
return `Err(AmigaError::WrongTask)` instead of crashing.

A few calls are allowed from any task, as the dos.library documentation
says: `DateStamp::now`, the `DateStamp` second conversions, `add_part`,
`file_part`, `path_part`, `fault`, `version` and `process::processes()`.

```rust
use amigaos4::dos;
use amigaos4::error::AmigaError;

if !dos::is_process() {
    // We are a plain exec task: file calls will give WrongTask.
}
match dos::fs::metadata("RAM:") {
    Err(AmigaError::WrongTask) => { /* not a Process */ }
    _ => {}
}
let now = dos::DateStamp::now(); // fine anywhere
```

If you need DOS in background work, start a child **Process** with
`dos::process` (see below) instead of an exec task.

## Errors

Every call returns `amigaos4::error::Result<T>`. A DOS failure is
`AmigaError::DosError(code)`, where `code` is the DOS error number
(IoErr) read straight after the call that failed. Two helpers turn it
into words:

- `dos::fault(code)` gives DOS's own text, for example "object not found"
  for 205;
- `dos::describe(&err)` does the same for a `DosError` and uses the
  crate's own text for any other error.

```rust
use amigaos4::dos;
use amigaos4::error::AmigaError;
use core::fmt::Write;

fn report(path: &str) {
    match dos::fs::File::open(path, dos::fs::OpenMode::Read) {
        Ok(_) => {}
        Err(AmigaError::DosError(205)) => { /* object not found */ }
        Err(e) => {
            if let Ok(mut err) = dos::stderr() {
                let _ = writeln!(err, "{}: {}", path, dos::describe(&e));
            }
        }
    }
}
```

Names are passed as Rust `&str`. A name with a NUL byte inside is refused
with `InvalidArgument` rather than cut short.

## Files: `fs::File`

`File` is an open file. It is closed when dropped. It is **unbuffered**:
every `read` and `write_all` is a dos.library call, so read and write in
large blocks, or keep your own buffer.

| Item | What it does |
|---|---|
| `File::open(path, mode)` | Open a file. |
| `OpenMode::Read` | An existing file, shared. |
| `OpenMode::Create` | A new, empty file; an old one is replaced. Exclusive. |
| `OpenMode::Update` | An existing file, or a new one, for reading and writing. Shared. |
| `File::write_new(path, data)` | Create a file and write all of `data`. |
| `File::read_to_vec(path)` | Read a whole file. |
| `read(&mut buf)` | Read some bytes; `Ok(0)` at the end. |
| `write_all(&buf)` | Write all of it; a short write is an error. |
| `seek(SeekFrom::Start / Current / End)` | Move; returns the new position. |
| `position()`, `len()`, `is_empty()`, `set_len(n)` | Position, size, truncate or extend. |
| `metadata()` | Examine the open file (see `Metadata`). |
| `path()` | The file's full name, for showing to the user. |
| `lock()` | A shared lock on the file. |
| `is_interactive()` | Whether it is a console window. |
| `close()` | Close now and get the result: a late write error shows here. |

`File` also implements the crate's `amigaos4::io::Read` and
`amigaos4::io::Write` traits (simple versions of the std ones).

```rust
use amigaos4::dos::fs::{File, OpenMode, SeekFrom};
use amigaos4::error::Result;

fn file_basics() -> Result<()> {
    File::write_new("RAM:hello.txt", b"Hello, Amiga!\n")?;
    let bytes = File::read_to_vec("RAM:hello.txt")?;

    let mut f = File::open("RAM:hello.txt", OpenMode::Update)?;
    f.seek(SeekFrom::Start(7))?;
    let mut buf = [0u8; 5];
    let n = f.read(&mut buf)?; // "Amiga"
    f.seek(SeekFrom::End(0))?;
    f.write_all(b"More text\n")?;
    f.close()?;
    Ok(())
}
```

`File` and `Lock` are not `Send`: DOS records them against the Process
that made them, so they stay in that Process. You can hand a file or a
lock to a child process through `ProcessBuilder` (below).

## Locks: `fs::Lock`

A lock refers to a file or directory without opening it. It is released
(UnLock) when dropped.

| Item | What it does |
|---|---|
| `Lock::new(path, LockMode::Shared)` | A shared lock; many may exist. |
| `Lock::new(path, LockMode::Exclusive)` | An exclusive lock; fails with "object in use" if anyone else holds one. |
| `try_clone()` | A second lock on the same object (shared locks only). |
| `parent()` | The parent directory; `None` at the root of a volume. |
| `path()` | The full name, for showing. |
| `same(&other)` | `Sameness::Same`, `SameVolume` or `Different`. |
| `metadata()` | Examine the object. |
| `read_dir()` | List the directory it locks. |
| `open()` | Turn the lock into an open `File`. On failure you get the lock back with the error. |

```rust
use amigaos4::dos::fs::{Lock, LockMode, Sameness};
use amigaos4::error::Result;

fn lock_basics() -> Result<()> {
    let dir = Lock::new("SYS:Prefs", LockMode::Shared)?;
    let full = dir.path()?;              // e.g. "Workbench:Prefs"
    assert!(dir.metadata()?.is_dir());

    // Walk up to the root of the volume.
    let mut cur = dir.try_clone()?;
    while let Some(up) = cur.parent()? {
        cur = up;
    }
    let sys = Lock::new("SYS:", LockMode::Shared)?;
    assert_eq!(sys.same(&cur)?, Sameness::Same);
    Ok(())
}
```

An empty name, `Lock::new("", LockMode::Shared)`, locks the current
directory.

## Information about a file: `fs::Metadata`

`fs::metadata(path)`, `File::metadata()` and `Lock::metadata()` return a
`Metadata`: a plain copy of what DOS reported, so nothing in it points
into DOS memory.

| Field | Meaning |
|---|---|
| `name`, `comment` | The name and the file comment (a short note DOS stores with each file). |
| `kind` | `Kind::File`, `Dir`, `SoftLink`, `Pipe`, `Socket` or `Other(n)`. Also `is_file()` and `is_dir()`. |
| `link`, `hard_link` | A soft link's target; whether it is a hard link. |
| `size` | `Some(bytes)` for files. |
| `date` | The last change, a `DateStamp`. |
| `protection` | The protection bits. The low four (read, write, execute, delete) are stored inverted: a set bit means *not* allowed. See `dos/dos.h` in the SDK. |
| `owner`, `object_id`, `ref_count`, `emulated` | Owner IDs, a unique object number, link count, and whether DOS emulated the call for an old filesystem. |

`fs::exists(path)` answers `Ok(true)` or `Ok(false)`, and an error only for
other failures.

## Listing directories: `fs::ReadDir`

`fs::read_dir(path)` (or `Lock::read_dir()`) gives an iterator of
`Result<Metadata>`. Each entry is copied before the next is read.

```rust
use amigaos4::dos::{self, fs, Pattern};
use amigaos4::error::Result;
use core::fmt::Write;

fn list() -> Result<()> {
    let mut out = dos::stdout()?;
    for entry in fs::read_dir("SYS:")? {
        let m = entry?;
        let kind = if m.is_dir() { "(dir)" } else { "" };
        let _ = writeln!(out, "{:30} {:>10} {}", m.name, m.size.unwrap_or(0), kind);
    }

    // Only the names that match a pattern, ignoring case.
    let info = Pattern::new("#?.info", false)?;
    let icons = fs::read_dir_matching("SYS:", &info)?.filter_map(|e| e.ok()).count();
    Ok(())
}
```

### Patterns: `Pattern`

AmigaDOS has its own wildcards. The common ones:

| Pattern | Matches |
|---|---|
| `#?` | anything (like `*` elsewhere) |
| `?` | one character |
| `(a\|b)` | `a` or `b` |
| `~x` | anything but `x` |
| `[a-z]` | one character in the range |

`Pattern::new(text, case)` parses a pattern; `case = false` ignores case.
`matches(name)` tests a name, and `is_wild()` says whether the pattern has
wildcards at all. For `read_dir_matching`, make the pattern with
`case = false`.

```rust
let p = amigaos4::dos::Pattern::new("#?.info", false)?;
assert!(p.matches("Disk.INFO")?);
```

## Creating, deleting and changing names

All in `dos::fs`:

| Function | What it does |
|---|---|
| `create_dir(path)` | Make a directory. |
| `delete(path)` | Delete a file or an *empty* directory. |
| `rename(from, to)` | Rename or move, within one volume. |
| `set_protection(path, bits)` | Set the protection bits. |
| `set_comment(path, text)` | Set the file comment. |
| `set_date(path, &date)` | Set the date. |

```rust
use amigaos4::dos::{self, fs};

fs::create_dir("RAM:work")?;
fs::File::write_new("RAM:work/a.txt", b"x")?;
fs::set_comment("RAM:work/a.txt", "made by Rust")?;
fs::set_date("RAM:work/a.txt", &dos::DateStamp::now()?)?;
fs::rename("RAM:work/a.txt", "RAM:work/b.txt")?;
fs::delete("RAM:work/b.txt")?;
fs::delete("RAM:work")?;
```

## Paths

AmigaOS paths look like `DH0:Work/Notes/today.txt`. The part before the
colon is a **volume** (a disk by name), a **device** (`DH0:`) or an
**assign** (a name that stands for a directory, such as `LIBS:`). `/` at
the start of a part means "the parent directory".

| Function | Example |
|---|---|
| `dos::add_part(dir, file)` | `"RAM:a"` + `"b"` = `"RAM:a/b"`; an absolute `file` replaces `dir` |
| `dos::file_part(path)` | `"DH0:a/b/c"` gives `"c"` |
| `dos::path_part(path)` | `"DH0:a/b/c"` gives `"DH0:a/b"` |

These three work in any task.

## Variables

AmigaOS has **local** variables (belonging to one Shell) and **global**
variables (files in `ENV:`, shared by every program; `ENVARC:` holds the
saved copies). `VarScope` picks which:

| `VarScope` | Get | Set / delete |
|---|---|---|
| `Any` | local first, then global | taken as `Local` |
| `Local` | local only | local |
| `Global` | `ENV:` only | `ENV:` |

```rust
use amigaos4::dos::{self, VarScope};

dos::set_var("MyTool/Colour", b"blue", VarScope::Global)?;
if let Some(v) = dos::get_var("MyTool/Colour", VarScope::Global)? {
    // v == "blue"
}
dos::delete_var("MyTool/Colour", VarScope::Global)?;
let lang = dos::get_var("Language", VarScope::Any)?;
```

`get_var` returns text (invalid UTF-8 replaced); `get_var_bytes` returns
the raw bytes, newlines included. Both return `Ok(None)` when the variable
does not exist.

## Dates: `DateStamp`

A `DateStamp` is how DOS stores a time: days since 1 January 1978, minutes
since midnight, and ticks (1/50 s) in the minute. Its fields are
`ds_Days`, `ds_Minute` and `ds_Tick`.

| Item | What it does |
|---|---|
| `DateStamp::now()` | The system time. Works in any task. |
| `DateStamp::from_seconds(s)` / `to_seconds()` | Seconds since 1978-01-01, both ways (ticks dropped). |
| `to_unix()` | Seconds since 1970-01-01. |
| `to_text(format, relative)` / `from_text(...)` | As text; see `misc` below. |

`DateStamp` is ordered: an earlier date compares less.

## The process's streams: `Stream`

`dos::stdout()`, `dos::stdin()` and `dos::stderr()` give the calling
Process's standard streams. In a Shell they are the console window.
`stderr()` falls back to the output stream when the Process has no
separate error stream. The crate never closes them.

A `Stream` implements `core::fmt::Write`, so `write!` and `writeln!` work,
and also the crate's `io::Read` and `io::Write`. `is_interactive()` tells
a console from a file or pipe. This is the usual way to print in a
no-C-runtime program, where `println!` does not exist.

```rust
use amigaos4::dos;
use core::fmt::Write;

let mut out = dos::stdout()?;
let _ = writeln!(out, "Hello from Rust, PID {}", dos::process::pid());
```

## Child processes: `dos::process`

`dos::process` runs a Rust closure in a new **child Process**. Unlike an
exec task, the child may use dos.library: files, locks, variables and its
own streams.

```rust
use amigaos4::dos::process;

let c = process::spawn("my worker", || 6 * 7)?;
let pid = c.pid();
let answer = c.wait()?; // 42
```

`ProcessBuilder` sets the details:

| Method | Default |
|---|---|
| `ProcessBuilder::new(name)` | the Process's name, as other tools show it |
| `priority(p)` | the parent's priority |
| `stack(bytes)` | `process::DEFAULT_STACK` (65536) |
| `input(file)`, `output(file)` | `NIL:` (nothing in, output thrown away) |
| `current_dir(lock)` | a copy of the parent's |
| `spawn(f)` | starts it and gives a `Child` |

Files and locks given to the child become the child's: DOS closes and
unlocks them when it ends. If starting fails they are still yours, and
are dropped.

```rust
use amigaos4::dos::{self, fs::{File, Lock, LockMode, OpenMode}};
use amigaos4::dos::process::{self, ProcessBuilder};
use core::fmt::Write;

let out = File::open("RAM:child.log", OpenMode::Create)?;
let dir = Lock::new("RAM:", LockMode::Shared)?;
let c = ProcessBuilder::new("my logger")?
    .stack(32768)
    .output(out)
    .current_dir(dir)
    .spawn(|| {
        if let Ok(mut o) = dos::stdout() {
            let _ = writeln!(o, "hello from PID {}", process::pid());
        }
        // A relative name lands in the given directory.
        dos::fs::File::write_new("relative.txt", b"in RAM:").is_ok()
    })?;
let ok = c.wait()?;
```

`Child`:

- `pid()`: the child's process ID;
- `is_running()`: whether it is still going;
- `wait()`: wait until the child has fully ended, then give its result.
  A child that panicked gives `Err(NullPointer)`;
- dropping a `Child` also waits for it.

Other calls:

- `process::pid()` and `process::parent_pid()`: the calling Process's IDs
  (0 in an exec task);
- `process::processes()`: every DOS process, newest first, as
  `ProcessInfo { pid, parent_pid, name, priority }`. Exec tasks are not
  listed. It is a copy: a process may have ended since.

### How it is kept safe

- The child waits for a start signal (CTRL-F) that the parent sends only
  after DOS has fully registered it. Without this, a child that ran and
  ended very quickly could crash DOS.
- The child is created as a dependant of the parent, so the parent's code
  stays loaded while the child runs, and `wait` returns only once the
  child has completely gone.
- `spawn` needs dos.library 54.107 or newer (for WaitForChildExit). On an
  older one it returns `Err(Unsupported)`.

### Memory the closure captures

The closure must be `Send + 'static`, and so must its result. The source
warns: what the closure captures lives where it was allocated. With the
crate's `ExecAllocator` (private memory) the child should not use it;
programs whose processes share heap memory use `SharedExecAllocator`.

### Detached processes

`ProcessBuilder::spawn_detached(f)` starts a Process that belongs to no
one and is never waited for. It returns the PID. It is meant for a server
process that outlives its starter, for example one started by the first
user of a library and stopped by the last. The code `f` runs in must stay
loaded until the process has gone: signal it to stop, then wait until it
can no longer be found by name before unloading. Give it a name of its
own.

### Loading and running programs: `SegList`

`SegList::load(path)` loads a program into memory (ELF or the older hunk
format). `run(args, stack)` runs it inside the calling Process with
`args` as its command line, and returns its exit code (0 is OK, 5 a
warning, 10 an error, 20 a failure). It shares your input, output and
current directory, and can be run again. It is unloaded on drop.

```rust
use amigaos4::dos::process::SegList;

let seg = SegList::load("C:Version")?;
let rc = seg.run("exec.library", 0)?; // 0 = DOS's default stack
```

## Command-line arguments: `dos::args`

AmigaOS programs describe their arguments with a **template**, and the
system call ReadArgs parses the command line against it. Each option is a
name followed by modifiers:

| Modifier | Meaning | Read it with |
|---|---|---|
| (none) | a string | `text(name)` |
| `/A` | required | (any) |
| `/K` | must be given with its keyword: `BUF=12` or `BUF 12` | (any) |
| `/S` | a switch, present or not | `switch(name)` |
| `/T` | a toggle: `ON`/`OFF`/`YES`/`NO` | `toggle(name)` |
| `/N` | a number | `number(name)` |
| `/M` | several strings | `multi(name)` |
| `/F` | the rest of the line | `text(name)` |

`BUF=BUFFERS` gives one option two names. Names are matched without case.

```rust
use amigaos4::dos::args::Args;

const TEMPLATE: &str = "FROM/A/M,TO/A,ALL/S,BUF=BUFFERS/K/N,QUIET/T";

let a = Args::parse(TEMPLATE, "a b c dest ALL BUF=12 QUIET=on")?;
let from = a.multi("FROM")?;      // ["a", "b", "c"]
let to = a.text("TO")?;           // Some("dest")
let all = a.switch("ALL")?;       // true
let buf = a.number("BUFFERS")?;   // Some(12), by either name
let quiet = a.toggle("QUIET")?;   // Some(true)
```

`Args::from_command_line(template)` reads your own program's arguments.
A typical std `main`:

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
    let name = args.text("NAME").ok().flatten().unwrap_or("Amiga");
    println!("Hello, {}", name);
}
```

Rules worth knowing:

- Reading an option at the wrong type, or a name not in the template, is
  an error (`InvalidArgument`).
- An absent option reads as `false`, `None` or an empty list.
- Bad command lines give DOS's own errors: a missing `/A` argument, a bad
  number, too many arguments, a keyword with no value.
- `parse_template(template)` splits a template into `Opt` values (names,
  `kind`, `required`, `keyword`, `rest`); `Args::options()` gives the same.
- The strings borrow the `Args`; everything is freed when it is dropped.

## Change notification: `dos::notify`

`Watcher::new(path)` asks DOS to signal your Process when a file or
directory changes. The path need not exist yet. A file's change is
reported when the file is closed.

`VarWatcher::new(name)` does the same for a global variable: its signal is
set when the variable is created, changed or deleted. Use it, not
`Watcher`, for anything in `ENV:` or `ENVARC:`.

Both have:

- `signal_mask()`: the bit to include in your own Wait();
- `changed()`: whether a change was signalled since the last check
  (clears it);
- `wait()`: block until the next change.

Both stop watching when dropped. `VarWatcher` must be dropped by the
Process that made it (it is not `Send`).

```rust
use amigaos4::dos::notify::{VarWatcher, Watcher};

let w = Watcher::new("RAM:inbox")?;
let v = VarWatcher::new("Language")?;
let mask = w.signal_mask() | v.signal_mask();
// ... Wait(mask) together with your other signals, then:
if w.changed() { /* rescan the drawer */ }
if v.changed() { /* reload the setting */ }
```

## Volumes, disks and assigns: `dos::volume`

- `names(Kinds { volumes, devices, assigns })`: the names DOS knows, each
  with its colon (`"Workbench:"`, `"DH0:"`, `"LIBS:"`). A snapshot.
- `disk_info(name)`: a `DiskInfo` with `state` (`DiskState::Validated`,
  `WriteProtected`, `Validating` or `Other`), `blocks`, `blocks_used`,
  `bytes_per_block`, `dos_type` (the filesystem's ID) and `in_use`, plus
  `total_bytes()` and `free_bytes()`.

```rust
use amigaos4::dos::volume::{self, Kinds};

let vols = volume::names(Kinds { volumes: true, devices: false, assigns: false })?;
let d = volume::disk_info("SYS:")?;
let free_mb = d.free_bytes() >> 20;
```

Making and removing assigns (names may be given with or without the
colon):

| Function | Kind of assign |
|---|---|
| `assign_lock(name, lock)` | `name:` is that directory from now on. DOS keeps the lock; on failure you get it back. |
| `assign_path(name, path)` | `name:` is expanded to `path` at each use; the path need not exist yet. |
| `assign_late(name, path)` | resolved at first use, then acts like a lock assign. |
| `assign_add(name, lock, position)` | adds a directory to a multi-directory assign (`None` appends). |
| `remove_assign_entry(name, &lock)` | takes one directory out of a multi-directory assign. |
| `remove_assign(name, AssignKind::Lock / Path / Late)` | removes the assign and all its entries. |

```rust
use amigaos4::dos::volume::{self, AssignKind};

volume::assign_path("PROJ", "RAM:project")?;
// "PROJ:file" now means "RAM:project/file"
volume::remove_assign("PROJ", AssignKind::Path)?;
```

## Odds and ends: `dos::misc`

- `system(command, input, output)` runs a Shell command line and waits for
  it, returning its exit code. It uses your own streams, or the files you
  give (which stay open and stay yours).
- `CurrentDir::enter(&lock)` changes the Process's current directory while
  the guard lives, and puts the old one back on drop.
- `NoRequesters::new()` stops "Please insert volume" requesters while it
  lives: calls on a volume that is not mounted fail at once instead of
  waiting for the user.
- `DateStamp::to_text(format, relative)` gives a `DateText { day, date,
  time }`. With `relative`, the day can be "Today", "Yesterday" and so on.
  `DateStamp::from_text(date, time, format)` goes the other way; a missing
  part means today or midnight. `DateFormat` is `Dos` (dd-mmm-yy), `Int`
  (yy-mmm-dd), `Usa` (mm-dd-yy), `Cdn` (dd-mm-yy), `Iso` (yyyy-mm-dd) or
  `Default` (the user's locale).

```rust
use amigaos4::dos::fs::{self, File, Lock, LockMode, OpenMode};
use amigaos4::dos::misc::{system, CurrentDir, DateFormat, NoRequesters};
use amigaos4::dos::DateStamp;

let out = File::open("RAM:list.txt", OpenMode::Create)?;
let rc = system("List SYS:", None, Some(&out))?;
drop(out);

let dir = Lock::new("RAM:", LockMode::Shared)?;
{
    let _here = CurrentDir::enter(&dir)?;
    File::write_new("note.txt", b"in RAM:")?;
} // the old current directory is back

let found = {
    let _quiet = NoRequesters::new()?;
    fs::exists("SomeDisk:file")
};

let now = DateStamp::now()?.to_text(DateFormat::Iso, false)?;
let xmas = DateStamp::from_text(Some("2025-12-24"), Some("18:00:00"), DateFormat::Iso)?;
```

## Other small items

- `dos::version()`: dos.library's version and revision.
- `dos::io_err()`: the Process's current DOS error code (0 in a task).

## What happens on a panic

Open files, locks, notifications and loaded programs register with the
task that made them. If the program panics through the crate's panic
handler, the crate releases what is still held (see the `track` module in
the crate's README).

## Deliberately left out

- Buffered I/O (FOpen and friends): use `File` and buffer in Rust.
- MatchFirst / MatchNext: `read_dir_matching` and `Pattern` cover them.
- The asynchronous form of System, which closes the streams it is given.
  Only the waiting form, `misc::system`, is wrapped.
- Shell (CLI) internals and DOS packets.

## See also

- [utility.md](utility.md): dates as calendar fields, tag lists, named
  objects.
- [workbench.md](workbench.md): programs started from Workbench, icons'
  tool types.
- [datatypes.md](datatypes.md) and [elf.md](elf.md), which build on `dos`.
