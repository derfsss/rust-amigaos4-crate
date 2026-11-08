# Messages, ports and other exec building blocks

This page covers how tasks talk to each other, and the small exec helpers
most programs meet sooner or later:

| Module | What it gives you | Cargo feature |
|---|---|---|
| `port` | `AmigaMsgPort`: an untyped exec message port | none |
| `msg` | Typed messages between the tasks of a program | none (`recv_timeout` needs `timer`) |
| `hook` | `Hook<F>`: an OS callback that calls a Rust closure | none |
| `list` | `NameList`: an exec list of named nodes | none (`sort_by` needs `utility`) |
| `avl` | `AvlMap<V>`: an ordered map on exec's AVL trees | none |
| `tag` | `TagListBuilder`: tag lists for OS calls | none |
| `cstr` | `amstr!` and `require_nul`: NUL-terminated strings | none |
| `io` | `Read` and `Write` traits | none |
| `error` | `AmigaError` and `Result` | none |

All of these work in both build modes (a std program, and a `#![no_std]`
build without a C runtime).

## AmigaOS terms used here

- **Message port**: a queue of messages owned by one task, with a signal
  bit. When a message arrives, exec sets the bit, so the owner can sleep
  until there is something to read.
- **Message**: a block that starts with exec's `struct Message` header.
  The sender puts it on a port (`PutMsg`); the receiver takes it
  (`GetMsg`) and, if the sender asked, sends it back (`ReplyMsg`) to the
  message's **reply port**.
- **Public port**: a port with a name in exec's public list, so other
  programs can find it with `FindPort`.

---

## `port`: `AmigaMsgPort`

`AmigaMsgPort` is a message port of the calling task, freed on drop. Use
it as a device I/O reply port, or as a public port other programs find by
name. For messages between your own tasks, `msg` (below) is safer.

| Method | Meaning |
|---|---|
| `AmigaMsgPort::new()` | A private port with a signal bit of its own. |
| `AmigaMsgPort::public(name)` | A public port; `name` ends in `\0`, exec copies it. Removed from the public list on drop. |
| `signal_mask()` | The port's signal, to `Wait` on with others. |
| `wait()` | Sleep until a message is queued (it stays queued). |
| `get_raw()` | Take the next message (`GetMsg`) as a raw `*mut Message`. You must reply or free it as its sender expects. |
| `as_ptr()` | The raw `MsgPort` for OS calls. |

```rust
use amigaos4::amstr;
use amigaos4::port::AmigaMsgPort;

fn public_port() -> amigaos4::Result<()> {
    let port = AmigaMsgPort::public(amstr!("MYAPP.1"))?;
    port.wait();
    while let Some(_m) = port.get_raw() {
        // Interpret and reply the message (exec_reply_msg) as the
        // protocol of whoever sent it says.
    }
    Ok(())
}
```

The port's signal bit belongs to the task that made it, so
`AmigaMsgPort` is neither `Send` nor `Sync`.

---

## `msg`: typed messages between tasks

`msg` carries any `T: Send + 'static` from one task to another over exec
ports.

| Type | Role |
|---|---|
| `Port<T>` | A port of the calling task that receives `Message<T>`s. Not `Send`. |
| `Sender<T>` | A handle to a port: `Send` and `Clone`, usable from any task. |
| `Message<T>` | A `T` inside an exec message (`MEMF_SHARED`), optionally with a reply port. |
| `Received<T>` | A message taken from a port. Derefs to `T`. |

### Sending and receiving

```rust
use amigaos4::amstr;
use amigaos4::msg::{Message, Port};
use amigaos4::task;

fn producer_consumer() -> amigaos4::Result<()> {
    let port: Port<u32> = Port::new()?;
    let tx = port.sender();

    let h = task::spawn(amstr!("producer"), move || {
        for i in 0..10u32 {
            if let Ok(m) = Message::new(i) {
                // A send to a port that has gone gives the message back.
                let _ = tx.send(m);
            }
        }
    })?;

    let mut sum = 0;
    for _ in 0..10 {
        let m = port.recv()?;   // waits
        sum += *m;              // dropped here: freed (no reply port)
    }
    h.join()?;
    assert_eq!(sum, 45);
    Ok(())
}
```

### `Port<T>` methods

| Method | Meaning |
|---|---|
| `Port::new()` | A port with a new signal bit of the calling task. |
| `sender()` | A `Sender<T>` for any task. |
| `signal_mask()` | The port's signal, to wait for it with others. |
| `try_recv()` | `Ok(Some(msg))` if one is queued, `Ok(None)` if not. |
| `recv()` | Wait for the next message. |
| `recv_or(signals)` | Wait for a message or one of `signals`: `Ok(Err(arrived))` when the signals came first. |
| `recv_timeout(&mut waiter, ms)` | Feature `timer`. Wait at most `ms` milliseconds: `Ok(None)` on timeout, `Err(Cancelled)` on Ctrl-C. Takes a `timer::Waiter`. |

The receive calls return `Err(WrongTask)` if called from a task other
than the one that made the port.

### Replies

`Message::with_reply(value, &reply_port)` makes a message that the
receiver sends back to `reply_port` (a port of the sending task). On the
receiving side:

- `received.reply()` sends it back (`ReplyMsg`).
- Dropping a `Received` that wants a reply **replies it too**, so a sender
  always gets its message back.
- `is_reply()` is true for one of your own messages coming back.
- `wants_reply()` says whether the sender wants it back.
- `into_message()` keeps it as a `Message<T>` (no reply on drop).

On the sending side, `Message::into_inner()` takes the value out and frees
the message.

```rust
use amigaos4::msg::{Message, Port, Sender};

struct Work { a: u32, b: u32, sum: u32 }

fn ask(server: &Sender<Work>, reply: &Port<Work>) -> amigaos4::Result<u32> {
    let m = Message::with_reply(Work { a: 2, b: 3, sum: 0 }, reply)?;
    if server.send(m).is_err() {
        return Ok(0); // the server's port is gone
    }
    let back = reply.recv()?;        // our message, replied
    Ok(back.into_message().into_inner().sum)
}

fn serve(port: &Port<Work>) -> amigaos4::Result<()> {
    let mut m = port.recv()?;
    m.sum = m.a + m.b;
    m.reply();
    Ok(())
}
```

### Safety properties

- A port's open/closed state sits behind an exec mutex that every send
  and reply holds, so nothing is ever put on a port whose owner has
  dropped it. Such a send returns the message as `Err`.
- Dropping a `Port` replies to the messages still queued in it.
- Limitation: `T` may be aligned to at most 8 bytes (`Message::new`
  panics otherwise).

---

## `hook`: OS callbacks into Rust

Many OS calls take a **hook**: a `struct Hook` holding a function the OS
calls back with an "object" and a "message" pointer. `Hook<F>` builds one
(`AllocSysObject(ASOT_HOOK)`, `MEMF_SHARED`) that calls your closure.

```rust
use amigaos4::hook::Hook;
use amigaos4::sys::APTR;

fn hook_demo() -> amigaos4::Result<()> {
    let hook = Hook::new(|object: APTR, message: APTR| -> u32 {
        let _ = (object, message);
        1 // the hook's 32-bit result
    })?;
    let raw = hook.as_ptr(); // pass to an OS call that uses it while `hook` lives
    let _ = raw;
    Ok(())
}
```

- The closure must be `Fn(APTR, APTR) -> u32 + Send + Sync`: the OS may
  call a hook from any task.
- The hook lives as long as the `Hook` value. Pass it only to calls that
  use it while it is borrowed; never to one that keeps it after you drop
  it.

---

## `list`: `NameList`

An **exec list** is a doubly linked list of `Node`s, each with a name, a
priority and a type. Many OS calls take or return one. `NameList` is an
exec list that owns its nodes (names are copied by exec).

| Method | exec call | Meaning |
|---|---|---|
| `NameList::new(list_type)` | | An empty list; `list_type` is an `NT_` value (0 is `NT_UNKNOWN`). |
| `push_back(name, pri)` / `push_front(name, pri)` | AddTail / AddHead | Add at either end. |
| `enqueue(name, pri)` | Enqueue | Insert by priority, highest first. |
| `pop_front()` / `pop_back()` | RemHead / RemTail | Remove; returns a `NodeInfo { name, pri }`. |
| `find(name)` / `find_ignore_case(name)` | FindName / FindIName | A borrowed `NodeRef`. |
| `remove(name)` | FindName + Remove | Remove the first node with that name. |
| `append(&mut other)` | MoveList | Move all of `other`'s nodes to the end. |
| `iter()`, `len()`, `is_empty()` | | Walk and count. |
| `sort_by(cmp)` | SortList | Feature `utility`; needs utility.library V54.2 (`Err(Unsupported)` otherwise). |
| `as_ptr()` | | The raw list, for OS calls that only read it. |

```rust
use amigaos4::list::NameList;

fn list_demo() -> amigaos4::Result<()> {
    let mut l = NameList::new(0)?;
    l.push_back("beta", 0)?;
    l.enqueue("alpha", 10)?;           // goes first (higher priority)
    for n in l.iter() {
        let _ = (n.name(), n.pri());
    }
    let _found = l.find("beta").is_some();
    Ok(())
}
```

Every change needs `&mut self`, so iterating can never meet a change. The
list is not locked against other tasks (an exec list never is).

---

## `avl`: `AvlMap<V>`

`AvlMap<V>` is an ordered map from `u32` keys to values, on exec's AVL
tree functions. It works like a small `BTreeMap<u32, V>`.

| Method | Meaning |
|---|---|
| `AvlMap::new()` | Empty (a `const fn`). |
| `insert(key, value)` | Insert; returns the old value if the key existed. |
| `get(key)`, `get_mut(key)`, `contains_key(key)` | Look up. |
| `remove(key)` | Remove; returns the value. |
| `iter()` | `(key, &value)` in key order. |
| `range_from(key)` | Entries with keys from `key` upwards. |
| `len()`, `is_empty()` | Count. |

```rust
use amigaos4::avl::AvlMap;

fn avl_demo() {
    let mut m = AvlMap::new();
    m.insert(30, "c");
    m.insert(10, "a");
    m.insert(20, "b");
    let keys: Vec<u32> = m.iter().map(|(k, _)| k).collect();
    assert_eq!(keys, [10, 20, 30]);
}
```

Each entry is a block from the program's allocator. Changes need
`&mut self`; exec does not lock a tree.

---

## `tag`: tag lists

A **tag list** is an array of `TagItem { ti_Tag, ti_Data }` pairs ending
with `TAG_DONE`. Many AmigaOS calls take their options this way.
`TagListBuilder` builds one:

```rust
use amigaos4::tag::TagListBuilder;
use amigaos4::sys::{AVT_TYPE, AVT_CLEAR_WITH_VALUE, MEMF_SHARED};

fn tags_demo(clear: bool) {
    let tags = TagListBuilder::new()
        .tag(AVT_TYPE, MEMF_SHARED)
        .tag_if(clear, AVT_CLEAR_WITH_VALUE, 0)   // TAG_IGNORE when false
        .build();                                 // Vec<TagItem>, TAG_DONE added
    let ptr = tags.as_ptr();
    // Pass `ptr` to the OS call; keep `tags` alive across the call.
    let _ = ptr;
}
```

`ti_Data` is 32 bits. A 64-bit value must be passed by reference (a
pointer to it in `ti_Data`); passed by value it is cut short silently.

---

## `cstr`: OS strings

AmigaOS calls take C strings: bytes ending in a NUL (`\0`). The crate's
safe wrappers that take `&[u8]` names require that trailing `\0`.

- `amstr!("text")` builds a `&'static [u8]` with the `\0` added at compile
  time. Prefer it to writing `b"text\0"` by hand.
- `cstr::require_nul(bytes)` checks a slice at run time and returns its
  pointer, or `Err(AmigaError::NotNulTerminated)`. Every wrapper calls it
  before handing a pointer to the OS, so a missing terminator is an error,
  never an out-of-bounds read.

```rust
use amigaos4::amstr;

fn names() {
    let s: &'static [u8] = amstr!("RAM:test.txt");
    assert_eq!(s.last(), Some(&0u8));
}
```

Which calls take which form: some safe wrappers take `&str` and add the
NUL themselves (for example `library::Library::open("utility.library", 50)`,
`named::alloc`, `NameList::push_back`). Others take a NUL-terminated
`&[u8]` because exec keeps or copies the pointer (for example
`task::spawn`, `AmigaMsgPort::public`, `devio::Device::open`,
`interrupt::IntServer::add`). Check each function's signature.

---

## `io`: `Read` and `Write`

`amigaos4::io::Read` and `amigaos4::io::Write` are small versions of the
std traits that return `AmigaError`. They work without std, so drivers can
use them too.

- `Read::read(&mut buf) -> Result<usize>` (0 at end of file), and the
  provided `read_exact`.
- `Write::write(&buf) -> Result<usize>`, and the provided `write_all` and
  `flush` (a no-op by default).

`dos::fs::File` and `dos::Stream` (feature `dos`) implement them. Your own
types can too.

---

## `error`: `AmigaError` and `Result`

Every safe wrapper returns `amigaos4::Result<T>`, an alias for
`core::result::Result<T, AmigaError>`. `AmigaError` is `Copy`, has a
`Display`, and with the `std` feature implements `std::error::Error`.

| Variant | Meaning |
|---|---|
| `NullPointer` | An OS call returned NULL unexpectedly (often "not found"). |
| `AllocationFailed` | Memory or another resource could not be allocated. |
| `IoError(i32)` | A clib4 call set errno. |
| `DosError(i32)` | A DOS error code from `IoErr()`. |
| `UnexpectedEof` | A read or write stopped early. |
| `NotNulTerminated` | A byte string lacked its `\0`. |
| `HostNotFound` | A host name did not resolve. |
| `DmaFailed` | `StartDMA` refused the region. |
| `Device(&str)` | A device or object misbehaved; the text says how. |
| `WouldDeadlock` | The calling task already holds the lock. |
| `WrongTask` | The call must run on another task. |
| `IoErr(i8)` | An exec I/O request ended with this `io_Error`, or `OpenDevice` refused with it. |
| `Exists` | The name is taken. |
| `InvalidArgument(&str)` | An argument the OS cannot take; the text says which. |
| `Unsupported(&str)` | The installed library is too old for the call. |
| `InUse` | Someone else holds it (for example a PCI device). |
| `Cancelled` | The user pressed Ctrl-C while it waited. |

```rust
use amigaos4::{AmigaError, Result};

fn describe(r: Result<()>) {
    match r {
        Ok(()) => {}
        Err(AmigaError::Cancelled) => amigaos4::serial_println!("stopped by Ctrl-C"),
        Err(e) => amigaos4::serial_println!("failed: {}", e),
    }
}
```
