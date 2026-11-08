# AmigaOS concepts for Rust developers

AmigaOS works differently from Linux or Windows in a few important ways.
This page explains the ideas you meet when you use the crate, in plain
terms, and names the crate type that handles each one. The module guides
(linked from [README.md](README.md)) go into each area in depth.

## The short version

- The OS is a set of **libraries**. You open a library, get an
  **interface** from it, and call functions through that interface.
- Programs run as **tasks** or **processes**. They talk to each other with
  **signals** and **messages** sent to **message ports**.
- Memory is one shared address space. You choose a **memory type** when you
  allocate, and some code (drivers, libraries) must use shared memory.
- Many functions take **tag lists**: arrays of (tag, value) pairs.
- The OS does not clean up after a program. Whatever you open, you must
  close. The crate does that for you with Rust's `Drop`.

## Libraries and interfaces

Almost every OS service lives in a **library**: `dos.library` for files,
`intuition.library` for windows, `graphics.library` for drawing, and so on.
A library is opened by name and minimum version (`OpenLibrary`), and closed
again when you are done (`CloseLibrary`).

AmigaOS 4 adds **interfaces**. A library offers one or more named
interfaces (usually `"main"`), each a table of function pointers. You get
one with `GetInterface` and give it back with `DropInterface`. Calls go
through the interface: in C, `IDOS->Open(...)`. The global variables
`IExec`, `IDOS`, `IIntuition` and so on hold these interface pointers.

**exec** is the kernel library. It is always open; `IExec` is set before
any of your code runs.

How the crate handles this:

| What | Crate type |
|---|---|
| Open a library, close it on drop | `library::Library` (`Library::open(name, version)`) |
| Get an interface, drop it before the library closes | `library::Interface<I>` (`Library::interface::<I>(name, version)`) |
| A resource (opened, never closed) | `library::open_resource` |
| Raw interface tables and functions | the `sys` module |

You rarely need `Library` yourself. Each safe module (`dos`, `intuition`,
`graphics`, ...) finds its own interface: it uses the program's global
(`IDOS`, `IIntuition`, ...) when that is set, and otherwise opens the
library itself the first time it is needed and keeps it. This logic is
internal to the crate (the private `iface` module). In practice: turn on
the feature, call the safe API, and the library is there.

```rust
// Cargo.toml: amigaos4 = { path = "../..", features = ["utility"] }
use amigaos4::library::Library;

fn show_version() -> amigaos4::Result<()> {
    // OpenLibrary("utility.library", 50); CloseLibrary when `lib` drops.
    let lib = Library::open("utility.library", 50)?;
    println!("utility.library {}.{}", lib.version(), lib.revision());
    Ok(())
}
```

## Tasks and processes

A **task** is exec's unit of execution: a thread with its own stack, a
priority and a set of signal bits. There is no memory protection between
tasks; they share one address space.

A **process** is a task with extra fields that dos.library needs: a current
directory, input and output streams, and so on. Only a process may call
dos.library (files, locks, variables), or anything that might load a
library or device from disk. Programs started from the Shell or Workbench
are processes.

| What | Crate type |
|---|---|
| A closure in a new exec task | `task::spawn`, `task::Builder` → `task::JoinHandle` |
| A closure in a new DOS process | `dos::process::spawn`, `dos::process::ProcessBuilder` → `Child` |
| The calling task's priority | `task::priority`, `task::set_priority` |

Rules the crate follows for you:

- Both kinds of spawn use a **start-up handshake**: the child waits until
  the parent holds its handle before it runs. Without this, a child that
  runs and ends very quickly can crash DOS.
- In an exec task, the crate's `dos` calls return `AmigaError::WrongTask`
  instead of crashing. Use a DOS process if the child needs files.
- Dropping a `JoinHandle` or `Child` waits for the child.
- If a child panics, `join` or `wait` returns an error (see
  [debugging.md](debugging.md)).
- Spawn children relative to `task::priority()`. A busy child above its
  parent's priority can starve the parent.

See [exec/tasks-and-sync.md](exec/tasks-and-sync.md) for details.

## Signals

Each task has 32 **signal bits**. Another task (or an interrupt) can set
them, and a task can **Wait** for any of a set of bits, sleeping without
using the CPU until one arrives. Signals are the base of all waiting on
AmigaOS: message ports, timers and windows all wake you by a signal.

Some bits are reserved; for example Ctrl-C in a Shell sets
`SIGBREAKF_CTRL_C`. A task allocates the bits it wants to use, and frees
them before it ends.

| What | Crate type |
|---|---|
| A signal bit of the calling task, freed on drop | `signal::Signal` (`alloc`, `mask`, `wait`, `take`) |
| Wait for any of several masks | `signal::wait(mask)` |
| Signal another task (unsafe: it must still exist) | `signal::SignalTarget` (from `Signal::target`) |
| Ctrl-C as a stop request | `signal::Cancel` (`Cancel::check()?`, `Cancel::MASK`) |

A `Signal` is not `Send`: a bit belongs to the task that allocated it.

```rust
use amigaos4::signal::{self, Cancel, Signal};

fn wait_for_work_or_break() -> amigaos4::Result<()> {
    let work = Signal::alloc()?;
    // Sleep until another task sets `work` or the user presses Ctrl-C.
    let got = signal::wait(work.mask() | Cancel::MASK);
    if got & Cancel::MASK != 0 {
        println!("stopped by Ctrl-C");
    }
    Ok(())
}
```

## Message ports and messages

A **message port** is a queue owned by one task, with a signal bit. Other
tasks put **messages** on it (`PutMsg`); when a message arrives, the
owner's signal is set. The owner takes messages off (`GetMsg`) and usually
**replies** them (`ReplyMsg`), which sends the message back to the
sender's reply port. Messages are not copied: the receiver reads the
sender's memory, so messages must be in shared memory.

Ports can be private, or public under a name so other programs can find
them. Windows, devices, Workbench and many libraries talk to you this way.

| What | Crate type |
|---|---|
| Untyped port (device replies, public ports) | `port::AmigaMsgPort` (`new`, `public`, `signal_mask`) |
| Typed messages between tasks of one program | `msg::Port<T>`, `msg::Sender<T>`, `msg::Message<T>`, `msg::Received<T>` |

The typed version allocates each message in shared memory, replies
automatically when a `Received<T>` is dropped, and hands back a message
sent to a port that has gone.

```rust
use amigaos4::msg::{Message, Port};
use amigaos4::task;

fn sum_from_child() -> amigaos4::Result<u32> {
    let port: Port<u32> = Port::new()?;
    let tx = port.sender();
    // A child task sends the numbers 1..=10.
    let child = task::spawn(amigaos4::amstr!("counter"), move || {
        for i in 1..=10u32 {
            if let Ok(m) = Message::new(i) {
                let _ = tx.send(m);
            }
        }
    })?;
    let mut sum = 0;
    for _ in 0..10 {
        let m = port.recv()?; // waits for the port's signal
        sum += *m;            // Received<u32> derefs to the value
    }                         // each message is replied (freed) on drop
    child.join()?;
    Ok(sum)
}
```

See [exec/messages-and-ports.md](exec/messages-and-ports.md).

## Memory types

All tasks share one address space, but each allocation has a **type**:

| Type | Meaning | Use it for |
|---|---|---|
| `MEMF_PRIVATE` | Visible only to the task that allocated it | A program's own heap, used by one task |
| `MEMF_SHARED` | Visible to all tasks | Anything another task touches: messages, a driver's or library's data, DMA buffers |

Why drivers and libraries need shared memory: a library's functions, and a
device's `BeginIO`, `Open` and `Close`, run **in the caller's task**, not
in the library's own. If the library's heap were private to one task,
another caller would be touching memory that is not visible to it. A
device built with private memory has crashed DOS this way.

| What | Crate type |
|---|---|
| Global allocator, private memory | `allocator::ExecAllocator` |
| Global allocator, shared memory | `allocator::SharedExecAllocator` |
| One buffer with a chosen type, freed on drop | `mem::AmigaVec` (`AmigaVec::alloc(size, mem_type)`) |
| Pools | `mem::AmigaPool`, `pool::MemPool`, `pool::ItemPool` |
| DMA buffers | `mem::DmaBuffer`, `mem::DmaMapping` |

Programs that use `std` get their heap from clib4 and need no allocator.
Code without a C runtime picks one of the two allocators; see
[drivers-and-libraries.md](drivers-and-libraries.md) and
[exec/memory.md](exec/memory.md).

## Tag lists

Many AmigaOS 4 functions take a **tag list** instead of a long argument
list: an array of `TagItem { ti_Tag, ti_Data }` pairs ending with
`TAG_DONE`. Each tag names an option; options you leave out get defaults.
`ti_Data` is 32 bits, so a pointer or a number fits, but a 64-bit value
must be passed by reference.

The safe modules build tag lists for you. When you call `sys` functions
directly:

| What | Crate type |
|---|---|
| Build a `Vec<TagItem>` ending in `TAG_DONE` | `tag::TagListBuilder` (`new`, `tag`, `tag_if`, `build`) |
| Read and filter tag lists received from the OS | `utility::tags` (`TagList`, `TagListMut`) |

```rust
use amigaos4::sys::TAG_USER;
use amigaos4::tag::TagListBuilder;

let tags = TagListBuilder::new()
    .tag(TAG_USER + 1, 5)
    .tag_if(false, TAG_USER + 2, 1) // left out when the condition is false
    .build();                       // ends with TAG_DONE
```

See [utility.md](utility.md).

## BOOPSI

**BOOPSI** is AmigaOS's object system: classes and objects created with
`NewObject`, configured with tag lists and driven by method calls. Gadgets,
ReAction windows, menus (menuclass), pop-up menus and datatypes are all
BOOPSI objects. The crate wraps the common ones so you never call BOOPSI
directly: `reaction::Window` builds the objects from a description,
`menu::Strip` builds menus, `popup::PopupMenu` pop-up menus, and
`datatypes::DtObject` datatype objects. Each is disposed of on drop.

See [gui/reaction.md](gui/reaction.md), [gui/menus.md](gui/menus.md) and
[datatypes.md](datatypes.md).

## Forbid and locking

Traditional Amiga code often calls `Forbid()` to stop task switching while
it changes shared data. This does not scale to several CPU cores and blocks
the whole system. **The crate never calls Forbid.** It uses exec's mutexes
and semaphores instead:

| What | Crate type |
|---|---|
| Mutual exclusion with data inside | `sync::Mutex<T>` (`lock`, `try_lock`, `lock_or_signal`) |
| Reader-writer lock | `sync::Semaphore<T>` (`read`, `write`, bids) |

Both refuse a second lock by the task that already holds them with
`AmigaError::WouldDeadlock` instead of hanging. Neither may be used from an
interrupt.

The only crate code that runs under Forbid is a low-memory handler
(`interrupt::MemHandler`), because exec calls those under Forbid itself.

## Cache and DMA

A device that reads or writes memory directly (**DMA**) needs physical
addresses, and the CPU caches must agree with memory. Some AmigaOS
machines are not cache-coherent, so getting this wrong corrupts data only
on some hardware.

The crate follows the exec rules: `StartDMA`, then `GetDMAList` for the
physical blocks, then `EndDMA`. `mem::DmaBuffer::map` does the first two
and returns a `DmaMapping`; dropping the mapping does `EndDMA`. Exec does
the cache work. The `arch` module also has the raw PowerPC cache and
barrier instructions (`cache_flush`, `cache_invalidate`, `mmio_r32`, ...)
for drivers that need them.

See [exec/memory.md](exec/memory.md) and
[devices/writing-devices.md](devices/writing-devices.md).

## How the safe types fit together

The pattern throughout the crate is **RAII**: a value owns an OS resource,
and dropping the value releases it.

| Resource | Opened by | Released on drop by |
|---|---|---|
| Library | `Library::open` | `CloseLibrary` |
| Interface | `Library::interface` | `DropInterface` (before the library closes) |
| Signal bit | `Signal::alloc` | `FreeSignal` |
| Message port | `AmigaMsgPort::new`, `msg::Port::new` | `FreeSysObject` (a `msg::Port` first replies what it still holds) |
| File | `dos::fs::File::open` | `Close` |
| Lock | `dos::fs::Lock::new` | `UnLock` |
| Window | `intuition::Window::open` | `CloseWindow` |
| Child task | `task::spawn` | join (waits for it) |
| Memory | `mem::AmigaVec::alloc` | `FreeVec` |

Lifetimes enforce order where the OS needs it. An `Interface` borrows its
`Library`, so it cannot outlive it. A `Window` borrows its `PubScreen`. A
`DmaMapping` borrows its buffer, so the CPU cannot touch the buffer while
the device owns it.

Rust's `Drop` does not run after a panic when a program is built with
`panic = "abort"`. For that case the crate keeps a registry of open
resources per task and releases them on a panic. See
[debugging.md](debugging.md).
