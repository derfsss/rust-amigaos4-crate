# Tasks, signals and locks

This page covers running code in parallel on AmigaOS and keeping shared
data safe:

| Module | What it gives you | Cargo feature |
|---|---|---|
| `task` | Spawn a closure as an exec task and join it; priorities, CPU affinity | none |
| `signal` | Signal bits: wait, take, peek, signal another task, Ctrl-C | none |
| `sync` | `Mutex<T>` and a reader-writer `Semaphore<T>` | none |
| `interrupt` | Interrupt servers, reset callbacks, low-memory handlers, software interrupts | none |
| `stack` | Run a closure on a bigger stack of its own | none |
| `sysinfo` | What exec knows about the CPU | none |

All of them are always built and work in both build modes: a std program
on clib4, and a `#![no_std]` build without a C runtime.

## AmigaOS terms used here

- **Task**: exec's thread. A **process** is a task with extra DOS state; only
  a process may call dos.library.
- **Signal**: each task has 32 signal bits. Another task (or an interrupt)
  sets a bit with `Signal()`; the owner sleeps in `Wait()` until one of
  the bits it asked for is set. Most of AmigaOS's waiting is built on this.
- **Priority**: a number from -128 to 127. The highest-priority ready task
  runs. A Shell command often runs at priority 0 or below.
- **Forbid/Permit**: an old way to stop task switching. This crate never
  uses it; it uses exec mutexes and semaphores instead.

---

## `task`: spawning tasks

`task::spawn(name, closure)` runs the closure in a new exec task and gives
back a `JoinHandle`. `join()` waits for the closure to end and returns its
result.

```rust
use amigaos4::amstr;
use amigaos4::task::{self, Builder};

fn spawn_demo() -> amigaos4::Result<()> {
    // Default: priority 0, 64 KiB stack.
    let h = task::spawn(amstr!("my worker"), || 6 * 7)?;
    assert_eq!(h.join()?, 42);

    // With the Builder: name, priority, stack.
    let h = Builder::new(amstr!("my helper"))
        .priority(task::priority().saturating_sub(1)) // just below me
        .stack(16384)
        .spawn(move || 1 + 1)?;
    let two = h.join()?;
    let _ = two;
    Ok(())
}
```

### The `Builder`

| Method | Meaning |
|---|---|
| `Builder::new(name)` | `name` is a `&'static [u8]` ending in `\0` (use `amstr!`); exec keeps the pointer. |
| `.priority(pri)` | The task's priority (default 0). |
| `.stack(bytes)` | Stack size (default `task::DEFAULT_STACK`, 65536). Do not cut it too close. |
| `.lock_stack(true)` | Lock the stack in memory (`CT_LockStack`). |
| `.spawn(closure)` | Create the task. `Err(NotNulTerminated)` for a bad name, `Err(AllocationFailed)` if exec refuses. |

### `JoinHandle`

| Method | Meaning |
|---|---|
| `join()` | Wait and take the result. A child that panicked gives an error. |
| `is_finished()` | Has the closure finished? (It may still be on its way out.) |
| `task()` | The child's `Task` pointer, for display and comparison only. |

Dropping a `JoinHandle` also joins (and drops the result). `JoinHandle` is
not `Send`: only the task that spawned the child can join it, because the
"done" signal is a bit of the spawner. Joining from another task returns
`Err(WrongTask)`.

### The start-up handshake

exec may run a new higher-priority task at once, before `CreateTask` has
even returned to the parent. A child that ran in that gap has crashed
DOS. So `spawn` always uses a handshake: the child first waits for the
`CTRL-F` signal, and the parent sends it once it holds the child's handle.
You do not need to do anything; it is why the child never runs "too
early", whatever the priorities.

At the other end, the child's last act is a tail call into exec's
`Signal`. After the parent has been told, no instruction of your program
runs on the child any more. So your program may end, and be unloaded,
right after a `join`.

### Rules for the closure

- **No DOS and no std I/O.** An exec task may not call dos.library, nor
  anything that may load a disk-based library or device. Use
  `amigaos4::serial_println!` for debug output, or hand results back. For
  a child that may use DOS, see `amigaos4::dos::process` (feature `dos`).
- **Heap sharing.** The task's control block is `MEMF_SHARED` whatever
  your allocator. But what the closure captures lives where it was
  allocated. A `#![no_std]` program whose tasks share heap data should use
  `allocator::SharedExecAllocator` (see [Memory](memory.md)).
- **Priorities.** Spawn relative to `task::priority()`. A child above its
  parent that never waits starves the parent.
- **Panics.** A panic in the child is caught; `join` then returns an
  error.

### The calling task

| Function | What it does |
|---|---|
| `task::priority()` | The calling task's priority. |
| `task::set_priority(pri)` | Set it; returns the old one. |
| `task::pin_to_current_core()` | Keep the calling task on the CPU core it is on now (`SetTaskAffinity`, exec V54.48; nothing happens on a single-core system). |
| `task::clear_affinity()` | Let it run on any core again. |
| `task::can_block()` | May the caller block (Wait, lock)? False inside Forbid or Disable, or in an interrupt. |
| `task::cpu_count()` | Number of CPUs. |

Affinity applies to the calling task only; the `Builder` has no affinity
option, so a child that wants pinning calls `pin_to_current_core()`
itself.

---

## `signal`: signal bits

### Free functions

| Function | What it does |
|---|---|
| `signal::wait(mask)` | Sleep until one of `mask`'s signals arrives; returns those that did, and clears them. `mask` 0 waits forever. |
| `signal::take(mask)` | Which of `mask` are set now, clearing them. No waiting. |
| `signal::peek(mask)` | Which of `mask` are set now, leaving them set. |
| `signal::current_task()` | The calling task's `Task` pointer. |

A Shell program's Ctrl-C check is `take(SIGBREAKF_CTRL_C) != 0`.

### `Signal`: a bit of your own

`Signal::alloc()` allocates a free signal bit of the calling task
(`AllocSignal`), freed on drop.

| Method | Meaning |
|---|---|
| `bit()`, `mask()` | The bit number (0..31) and `1 << bit`. |
| `wait()` | Wait for this signal alone. |
| `wait_or(others)` | Wait for this or any of `others`; returns what arrived. |
| `take()` | Is it set? Clears it. |
| `clear()` | Clear it without waiting. |
| `target()` | A `SignalTarget` another task can signal. |

`Signal` is neither `Send` nor `Sync`. exec says signal bits are only
valid for the task that allocated them, so a `Signal` lives and dies on
that task.

### `SignalTarget`: signalling another task

A `SignalTarget` is a task and a mask. It is `Send`, `Sync` and `Copy`, so
you can hand it to another task. Sending is `unsafe`: you promise the task
still exists and still owns those bits.

```rust
use amigaos4::amstr;
use amigaos4::signal::Signal;
use amigaos4::task;

fn wake_me() -> amigaos4::Result<()> {
    let ready = Signal::alloc()?;
    let target = ready.target();
    let h = task::spawn(amstr!("waker"), move || {
        // SAFETY: the parent waits for this signal before `ready` is
        // dropped, so the task and its bit are still there.
        unsafe { target.send() };
    })?;
    ready.wait();
    h.join()?;
    Ok(())
}
```

`SignalTarget::new(task, mask)` builds one from parts, for example a
task's Ctrl-C: `SignalTarget::new(task, SIGBREAKF_CTRL_C)`.

### `Cancel`: Ctrl-C as a stop request

The Shell and the `Break` command send `SIGBREAKF_CTRL_C`. `Cancel` treats
it as "please stop":

- `Cancel::requested()`: has Ctrl-C come since the last look? Clears it.
- `Cancel::check()`: `Err(AmigaError::Cancelled)` if so, else `Ok(())`, for
  use with `?` in a long loop.
- `Cancel::MASK`: the bit, to include in your own waits.
  `timer::Waiter` does this for you.

```rust
use amigaos4::signal::Cancel;

fn long_job() -> amigaos4::Result<()> {
    for _chunk in 0..1000 {
        Cancel::check()?; // stop early on Ctrl-C
        // ... work ...
    }
    Ok(())
}
```

Rules from exec: never call `Wait` from an interrupt, and do not allocate
or free signals in exception code.

---

## `sync`: locks without Forbid

### `Mutex<T>`

`Mutex<T>` is data behind an exec mutex (exec V52). Lock it to get a
guard; the guard derefs to `T` and releases on drop.

```rust
use amigaos4::sync::Mutex;

fn mutex_demo() -> amigaos4::Result<()> {
    let counter = Mutex::new(0u32)?;   // creating it can fail
    {
        let mut g = counter.lock()?;    // waits as long as it takes
        *g += 1;
    }                                   // released here
    if let Some(mut g) = counter.try_lock() {
        *g += 1;
    }
    Ok(())
}
```

| Method | Meaning |
|---|---|
| `Mutex::new(value)` | A new unlocked mutex. Returns `Result`. |
| `lock()` | Wait for the lock. `Err(WouldDeadlock)` if this task holds it already. |
| `try_lock()` | Lock if free now; `None` if anyone holds it (this task too). |
| `lock_or_signal(signals)` | Lock, or give up when one of `signals` arrives: `Ok(Err(arrived))`. |
| `get_mut()`, `into_inner()` | Access without locking when you own the mutex. |

Why `WouldDeadlock`: the mutex is created **not recursive** (exec's
default is recursive, which would let one task hold two `&mut T`). exec
itself would either deadlock or grant the lock again, so the crate tracks
the owner and refuses a second lock by the holder.

To share a `Mutex` between tasks, put it in an `Arc` (in a no_std program
with a `SharedExecAllocator` heap).

### `Semaphore<T>`: a reader-writer lock

`Semaphore<T>` is data behind an exec SignalSemaphore. Many tasks may
read at once; one may write.

| Method | Meaning |
|---|---|
| `Semaphore::new(value)` | A new private semaphore. Returns `Result`. |
| `write()` / `read()` | Exclusive / shared lock, waiting. |
| `try_write()` / `try_read()` | `Ok(Some(guard))` if free now, `Ok(None)` if not. |
| `write_or_signal(s)` / `read_or_signal(s)` | Lock, or `Ok(Err(arrived))` when one of `s` comes first (exec V53.77). |
| `bid_write()` / `bid_read()` | Ask for the lock without blocking (see below). |
| `get_mut()`, `into_inner()` | Access without locking. |

Every lock call returns `Err(WouldDeadlock)` when the calling task holds
the exclusive lock already. One case cannot be caught: a task that holds
a **shared** lock and asks for the exclusive one waits forever. Don't do
that.

```rust
use amigaos4::sync::Semaphore;

fn semaphore_demo() -> amigaos4::Result<()> {
    let table = Semaphore::new([0u32; 16])?;
    {
        let r = table.read()?;
        let _first = r[0];
    }
    table.write()?[3] = 7;
    Ok(())
}
```

### Bids: asking for a lock without blocking

`bid_write()` and `bid_read()` post a request (exec's `Procure`) and
return at once with a `WriteBid` or `ReadBid`. The bid has a message port
of its own; its `signal_mask()` can join a `Wait` on other signals. When
exec grants the lock:

- `bid.is_granted()` becomes true,
- `bid.take()` turns it into the usual guard (or gives the bid back if not
  yet granted),
- `bid.wait()` waits for the grant and returns the guard.

Dropping a bid that was never taken cancels it (`Vacate`): a pending bid
is withdrawn, a granted one released.

```rust
use amigaos4::signal;
use amigaos4::sync::Semaphore;

fn bid_demo(s: &Semaphore<u32>, other_signals: u32) -> amigaos4::Result<()> {
    let mut bid = s.bid_write()?;
    loop {
        if bid.is_granted() {
            break;
        }
        let got = signal::wait(bid.signal_mask() | other_signals);
        if got & other_signals != 0 {
            return Ok(()); // dropping the bid withdraws it
        }
    }
    let mut g = bid.wait(); // already granted: returns at once
    *g += 1;
    Ok(())
}
```

### General rules

- Both lock types are `MEMF_SHARED` and not tracked to their creator, so
  the task that made one may end while others use it.
- They are for tasks only: never lock from an interrupt.
- Guards are not `Send`: the task that locked releases.

---

## `interrupt`: interrupt code

All four types run your code outside normal task context. Their
constructors are `unsafe`: you promise the function obeys interrupt rules
and that its `data` pointer stays valid until the object is dropped.
Names must end in `\0` (`amstr!`); exec copies them.

General interrupt rules (from the exec autodocs): no waiting, no
allocation, no locks, nothing that can block. `Signal` is allowed, and is
how interrupt code wakes the task that does the real work.

### `IntServer`: a hardware interrupt server

`IntServer::add(int_num, pri, name, code, data)` puts your function on
exec's interrupt chain `int_num` (`AddIntServer`). Dropping it removes it.
The function type is `ServerFn`:

```rust
unsafe extern "C" fn(ctx: *mut c_void, sysbase: *mut c_void, data: *mut c_void) -> u32
```

Return non-zero only if your device raised the interrupt. Drivers use this
most; see [Writing device drivers](../devices/writing-devices.md#interrupt-servers)
for a full example, including the PCI rules (shared, level-triggered
lines) and `arch::clear_reservation`.

### `ResetCallback`: stop hardware on reboot

`ResetCallback::add(pri, name, code, data)` runs `code` when the machine
reboots (`AddResetCallback`; priority 127 runs first). Its job is to stop
a device doing DMA, so the next boot does not find it writing into memory
that has been handed out again.

- It runs in user mode under Forbid on a reboot, but a power-off walks the
  same list in supervisor mode: do not depend on the mode.
- It may be entered twice: guard it. Take no lock; only stop the hardware.
- Dropping it removes it. Code that can be unloaded must drop it first,
  or exec calls freed memory at the next reboot. Code that is never
  unloaded (a Kickstart module) may keep it with `core::mem::forget`.

### `MemHandler`: help when memory runs out

`MemHandler::add(pri, name, code, data)` (`AddMemHandler`) is called when
an allocation fails, so you can free caches and let exec retry. The
function type is `MemHandlerFn`:

```rust
unsafe extern "C" fn(sysbase: *mut c_void, request: *const sys::MemHandlerData, data: *mut c_void) -> i32
```

Return `sys::MEM_DID_NOTHING`, `sys::MEM_ALL_DONE` (memory was freed:
retry, then go on to the next handler) or `sys::MEM_TRY_AGAIN` (retry and
call this handler again). It runs in the failing task under Forbid: no
`Wait`, no lock, no allocation. It may run before `add` returns. Positive
priorities run before libraries are expunged, negative ones after.

```rust
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, Ordering};
use amigaos4::interrupt::MemHandler;
use amigaos4::sys;

static CALLS: AtomicU32 = AtomicU32::new(0);

unsafe extern "C" fn low_mem(_sb: *mut c_void, _req: *const sys::MemHandlerData, _d: *mut c_void) -> i32 {
    CALLS.fetch_add(1, Ordering::Relaxed); // free a cache here
    sys::MEM_DID_NOTHING
}

fn install() -> amigaos4::Result<MemHandler> {
    // SAFETY: the handler only touches a static atomic; it allocates nothing.
    unsafe { MemHandler::add(50, b"my cache\0", low_mem, core::ptr::null_mut()) }
}
```

### `SoftInt`: a software interrupt

`SoftInt::new(pri, name, code, data)` makes a software interrupt; `cause()`
runs it (`Cause`). From a task, the handler runs before `cause` returns.
Priority must be -32, -16, 0, 16 or 32. The handler type is `SoftIntFn`:
`unsafe extern "C" fn(unused: i32, sysbase: *mut c_void, data: *mut c_void)`.
`SoftInt` is `Send` and `Sync`.

---

## `stack`: a bigger stack for one call

`stack::run_with_stack(size, closure)` runs the closure on a new stack of
`size` bytes (`NewStackRun`, exec V52.6) and returns its result. The
stack is allocated for the call and freed when it returns. Use it for deep
recursion or big local arrays, without making the whole task's stack
larger.

```rust
use amigaos4::stack;

fn deep() -> amigaos4::Result<u64> {
    stack::run_with_stack(2 * 1024 * 1024, || {
        let big = [1u8; 512 * 1024]; // too big for a small task stack
        big.iter().map(|&b| b as u64).sum()
    })
}
```

`Err(AllocationFailed)` if exec cannot provide the stack.

---

## `sysinfo`: the CPU

`sysinfo::cpu()` returns a `CpuInfo` with everything `GetCPUInfo` reports:

| Field | Meaning |
|---|---|
| `count` | Number of CPUs |
| `family`, `model`, `model_name` | `CPUFAMILY_*`, `CPUTYPE_*`, a name |
| `version`, `version_name` | CPU revision |
| `cpu_speed`, `bus_speed`, `timebase_speed` | In Hz; the timebase is what `arch::timebase()` counts |
| `l1_cache`, `l2_cache`, `l3_cache`, `cache_line` | Cache sizes and line size in bytes |
| `vector_unit` | `VECTORTYPE_*` (0: none) |
| `extensions` | Extension words |
| `cpu_page_size`, `exec_page_size` | Page sizes |
| `current_cpu` | The core the caller runs on (exec V54.48) |

```rust
fn show_cpu() {
    let c = amigaos4::sysinfo::cpu();
    amigaos4::serial_println!("{} x {} at {} MHz, {}-byte cache lines",
        c.count, c.model_name, c.cpu_speed / 1_000_000, c.cache_line);
}
```

---

## See also

- [Messages and ports](messages-and-ports.md): passing typed messages
  between tasks, built on these signals and locks.
- [Using devices](../devices/using-devices.md): `timer::Waiter` waits on
  signals with a time limit.
