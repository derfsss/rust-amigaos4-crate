# Using exec devices

This page is for programs that **use** a device: open it, send it
requests, and close it. To write a device driver, see
[Writing device drivers](writing-devices.md).

| Module | What it gives you | Cargo feature |
|---|---|---|
| `devio` | `Device`, `Request`, `InFlight`: open a device and run I/O requests | `device` |
| `timer` | timer.device: `Waiter`, `AmigaTimer`, `TimerVal`, `get_up_time`, ... | `timer` (turns on `device`) |

Both work in both build modes: a std program on clib4, and a `#![no_std]`
build without a C runtime.

```toml
[dependencies]
# a std program
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["timer"] }
# or a no-C-runtime build
# amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", default-features = false, features = ["timer"] }
```

## How exec devices work

An **exec device** is a driver with a standard interface (`timer.device`,
`serial.device`, `ram.device`, a disk or network driver). You talk to it
with **I/O requests**:

1. You make a **reply port** (a message port of your task) and an **I/O
   request** (an `IORequest`, or a larger device-specific structure such
   as an `IOStdReq` or a `timerequest`).
2. `OpenDevice(name, unit, request, flags)` connects the request to the
   device and one of its **units** (a device can drive several drives,
   ports or timers; the unit number picks one).
3. You fill in the request: a **command** (`CMD_READ`, `CMD_WRITE`,
   `TR_ADDREQUEST`, ...) and its data. An `IOStdReq` has `io_Data` (a
   buffer), `io_Length`, `io_Offset` and, afterwards, `io_Actual` (how many
   bytes were moved) and `io_Error` (0 for success).
4. You run it:
   - `DoIO`: synchronous; returns when the request is done.
   - `SendIO`: asynchronous; the device replies the request to your port
     when it is done. `CheckIO` asks whether it is done, `WaitIO` waits
     for it, `AbortIO` asks the device to stop it early.
5. `CloseDevice` when you are finished.

`devio` does all of this with ownership rules that make the risky parts
(a buffer the device is still writing, a request left with the device)
impossible in safe code.

---

## `devio`: open a device, run requests

### `Device`

| Method | Meaning |
|---|---|
| `Device::open(name, unit, flags, size)` | Open `unit` of `name` (NUL-terminated, e.g. `b"timer.device\0"`) with a request of `size` bytes. `size` is at least `devio::IOREQUEST_SIZE` (32); use `devio::IOSTDREQ_SIZE` (48) for an `IOStdReq`, 40 for a `timerequest`. `Err(IoErr(e))` when `OpenDevice` refuses with error `e`. |
| `request()` | The request the device was opened with (`&mut Request`). |
| `request_ref()` | The same, to read. |
| `new_request()` | Another request on the same open device and reply port, for I/O in parallel. |
| `new_request_on(&port)` | Another request that replies to a port of your own. |
| `signal_mask()` | The reply port's signal, to wait on several things at once. |
| `base()` | The device base (`io_Device`), for `GetInterface` on a device that has one. |

Dropping a `Device` aborts a started request, closes the device and frees
the request and port. The reply port's signal belongs to the task that
opened the device, so a `Device` and its requests are not `Send`.

### `Request`: running it

| Method | Meaning |
|---|---|
| `set_command(cmd)`, `command()` | `io_Command`. |
| `set_flags(f)`, `flags()` | `io_Flags`. |
| `extra()`, `extra_mut()` | The bytes after the 32-byte `IORequest` header: the device-specific fields (a `timerequest`'s time value, for example). |
| `do_io()` | Run it synchronously (`DoIO`). `Err(IoErr(io_Error))` on failure. |
| `do_io_data(cmd, &mut data, offset)` | Run `cmd` on `data` at `offset` (sets `io_Data`, `io_Length`, `io_Offset`) and return `io_Actual`. Needs an `IOStdReq`-sized request. |
| `send()` | Send it (`SendIO`); returns an `InFlight`. |
| `send_data(cmd, &mut data, offset)` | Send `cmd` on `data`; the buffer stays borrowed until it completes. |
| `error()`, `actual()`, `size()` | `io_Error`, `io_Actual`, the request's size. |

### `InFlight`: a request the device owns

`send()` and `send_data()` return an `InFlight`, which borrows the request
**and the data buffer** until the request completes. The compiler
therefore stops you touching the buffer while the device may be writing
it.

| Method | Meaning |
|---|---|
| `is_done()` | Has it completed (`CheckIO`)? |
| `wait()` | Wait for it (`WaitIO`); `Err(IoErr(e))` if it failed. |
| `wait_actual()` | Wait and return `io_Actual`. |
| `abort()` | `AbortIO` then wait. An aborted request gives `Err(IoErr(-2))` (`IOERR_ABORTED`); one that had already finished keeps its result. |

Dropping an `InFlight` aborts the request and waits for it, so a request
is never left with the device.

### Requests without a buffer: `start` / `check` / `finish` / `cancel_pending`

For a command with no data buffer (a timer's), you can start a request
without holding a borrow:

| Method | Meaning |
|---|---|
| `start()` | Send it (`SendIO`) and remember it is pending. |
| `is_pending()` | Started and not yet collected. |
| `check()` | Has it completed? (True when none is pending.) |
| `finish()` | Collect it (`WaitIO`) with its result. |
| `cancel_pending()` | `AbortIO` + `WaitIO`. |

Anything that touches a started request (setting its command, flags or
extra bytes, running it again, dropping it) cancels it first.

When you wait on several kinds of request at once, give each kind its
own reply port with `new_request_on`: a `WaitIO` on one port then never
takes another request's signal.

---

## Worked example 1: reading and writing a device

This reads and writes the example `ram.device` (from
`examples/ram-device`), whose unit 0 is a 64 KiB RAM buffer. `CMD_WRITE`
is 3 and `CMD_READ` is 2 (`device::cmd` has the names).

```rust
use amigaos4::devio::{Device, IOSTDREQ_SIZE};
use amigaos4::device::cmd::{CMD_READ, CMD_WRITE};

fn ram_roundtrip() -> amigaos4::Result<()> {
    let mut ram = Device::open(b"RAM:ram.device\0", 0, 0, IOSTDREQ_SIZE)?;

    let mut out = [0u8; 4096];
    for (i, b) in out.iter_mut().enumerate() {
        *b = i as u8;
    }
    let written = ram.request().do_io_data(CMD_WRITE, &mut out, 8192)?;

    let mut back = [0u8; 4096];
    let read = ram.request().do_io_data(CMD_READ, &mut back, 8192)?;
    assert!(written == 4096 && read == 4096 && back == out);

    // The same read, sent asynchronously; `back` is borrowed meanwhile.
    let f = ram.request().send_data(CMD_READ, &mut back, 8192)?;
    // ... do other work, or wait on f's signal with others ...
    let n = f.wait_actual()?;
    assert_eq!(n, 4096);
    Ok(())
} // the device is closed here
```

The device feature is `device`:

```toml
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["device"] }
```

### Two requests at once

```rust
use amigaos4::devio::{Device, Request};

const TR_ADDREQUEST: u16 = 9;        // devices/timer.h
const TIMEREQUEST_SIZE: u32 = 40;    // IORequest (32) + TimeVal (8)

fn set_delay(r: &mut Request<'_>, ms: u32) {
    r.set_command(TR_ADDREQUEST);
    let x = r.extra_mut();
    x[0..4].copy_from_slice(&(ms / 1000).to_be_bytes());          // seconds
    x[4..8].copy_from_slice(&((ms % 1000) * 1000).to_be_bytes()); // microseconds
}

fn two_timers() -> amigaos4::Result<()> {
    let timer = Device::open(b"timer.device\0", 0, 0, TIMEREQUEST_SIZE)?;
    let mut a = timer.new_request()?;
    let mut b = timer.new_request()?;
    set_delay(&mut a, 300);
    set_delay(&mut b, 100);
    let fa = a.send();
    let fb = b.send();
    fb.wait()?;      // done after about 100 ms
    fa.wait()?;      // done after about 300 ms
    Ok(())
}
```

For timing, the `timer` module below is simpler; this shows the raw
pattern for any device with device-specific request fields.

---

## `timer`: timer.device

Feature `timer` (it turns on `device`).

### `TimerVal`: a time value

`TimerVal { secs, micros }` is always normalised (`micros < 1_000_000`),
as timer.device requires.

| Item | Meaning |
|---|---|
| `TimerVal::new(secs, micros)` | Microseconds of a second or more carry into the seconds. |
| `from_millis(ms)`, `from_micros(us)` | Build from a count. |
| `as_millis()`, `as_micros()` | Total as `u64`. |
| `checked_add(o)`, `checked_sub(o)` | `None` on overflow / when `o` is later. |
| `TimerVal::ZERO` | Zero. |

`TimerVal` is ordered (earlier is less) and hashable.

### Clock functions (no device to open)

| Function | Meaning |
|---|---|
| `timer::get_up_time()` | Time since the system started. Never set, so monotonic: use it to measure. |
| `timer::sys_time()` | The system (wall-clock) time. May be set from the battery clock or by software: not monotonic. |
| `timer::eclock()` | `Option<EClock>`: an E-clock count and its frequency. `EClock::micros_since(&earlier)` gives elapsed microseconds. |
| `timer::micro_delay(us)` | Busy-wait. Short waits only: the task spins, using the CPU. |

These use the program's `ITimer` global when it is set, otherwise one the
crate opens itself (timer.device opened once, never closed). If
timer.device cannot be reached, `get_up_time` and `sys_time` return
`TimerVal::ZERO`.

### `Waiter`: wait for signals with a time limit

`Waiter` is the easiest way to sleep with a timeout. It owns a timer
request and can be reused.

| Method | Meaning |
|---|---|
| `Waiter::new()` | Open a MicroHz timer for waiting. |
| `wait(mask, timeout)` | Wait for any signal in `mask`, at most `timeout` (`None`: no limit). Returns `Waited::Signals(arrived)` or `Waited::TimedOut`. |
| `wait_ms(mask, ms)` | The same with the limit in milliseconds. |

Signals already set count at once. **Ctrl-C ends any wait** with
`Err(AmigaError::Cancelled)` (and is cleared), whether or not `mask`
names it.

### Worked example 2: a timed wait

Wait up to two seconds for a signal from another task, stopping early on
Ctrl-C:

```rust
use amigaos4::amstr;
use amigaos4::signal::Signal;
use amigaos4::task;
use amigaos4::timer::{Waited, Waiter};
use amigaos4::AmigaError;

fn timed_wait() -> amigaos4::Result<()> {
    let done = Signal::alloc()?;
    let target = done.target();
    let worker = task::spawn(amstr!("slow worker"), move || {
        // ... work ...
        // SAFETY: the parent joins this task before `done` is dropped.
        unsafe { target.send() };
    })?;

    let mut waiter = Waiter::new()?;
    match waiter.wait_ms(done.mask(), 2000) {
        Ok(Waited::Signals(_)) => amigaos4::serial_println!("worker finished"),
        Ok(Waited::TimedOut) => amigaos4::serial_println!("still busy after 2 s"),
        Err(AmigaError::Cancelled) => amigaos4::serial_println!("Ctrl-C"),
        Err(e) => return Err(e),
    }
    worker.join()?; // waits for the worker in any case
    Ok(())
}
```

`msg::Port::recv_timeout(&mut waiter, ms)` uses a `Waiter` in the same
way to receive a typed message with a time limit (see
[Messages and ports](../exec/messages-and-ports.md)).

### `AmigaTimer`: an open timer unit

`AmigaTimer::open(unit)` opens a timer.device unit. The units
(`TimerUnit`):

| Unit | Meaning |
|---|---|
| `MicroHz` | Microsecond precision; may drift under load. |
| `VBlank` | Vertical-blank precision (about 1/60 s); stable, low overhead. |
| `EClock` | E-clock ticks (rate depends on the system). |
| `WaitUntil` | Wait until an absolute system time. |

| Method | Meaning |
|---|---|
| `delay(duration)` | Wait `duration` (for `WaitUntil`: until that system time). Runs on its own request and reply port, so a started request is left running. |
| `delay_ticks(ticks)` | Wait E-clock ticks (the `EClock` unit only; others refuse with `InvalidArgument`). |
| `start(duration)` | Start a timer request without waiting. Starting again cancels the pending one. |
| `signal_mask()` | The signal a started request sets when it completes. |
| `is_done()` | Has the started request completed? (True when none is pending.) |
| `finish()` | Collect it (`WaitIO`). |
| `abort()` | Cancel it **and clear its signal** (see below). |
| `up_time()`, `get_sys_time()`, `eclock()` | Clock reads through the unit's own `ITimer`. |

On the `EClock` unit, a `TimerVal` given to `delay` or `start` is
converted to ticks with the E-clock frequency.

```rust
use amigaos4::signal;
use amigaos4::timer::{AmigaTimer, TimerUnit, TimerVal};

fn tick_loop(work_signal: u32) -> amigaos4::Result<()> {
    let mut timer = AmigaTimer::open(TimerUnit::MicroHz)?;
    timer.delay(TimerVal::from_millis(500))?;     // a plain sleep

    timer.start(TimerVal::from_millis(100));      // a periodic tick
    for _ in 0..50 {
        let got = signal::wait(timer.signal_mask() | work_signal);
        if got & work_signal != 0 {
            // ... handle the work ...
        }
        if timer.is_done() {
            timer.finish();
            // ... the 100 ms tick ...
            timer.start(TimerVal::from_millis(100));
        }
    }
    Ok(()) // dropping the timer aborts a started request
}
```

Why `abort` clears the signal: an aborted request's reply still sets its
port's signal, and `WaitIO` takes the message but leaves the signal. Left
alone, the next `Wait` on it would return at once. A loop that aborts and
restarts its timer on every other wakeup would then never sleep and use
all the CPU. `AmigaTimer::abort` clears it for you; the timer's port is
its own, so no other reply's signal is lost.

---

## Tips

- Use `Device::open`'s `size` to match the device's request type. Data
  commands (`do_io_data`, `send_data`) need at least `IOSTDREQ_SIZE`.
- Errors from requests are `AmigaError::IoErr(code)`; the common codes are
  in `device::ioerr` (`IOERR_ABORTED` = -2, `IOERR_NOCMD` = -3, ...).
  `AmigaTimer::delay` reports a request error as `AmigaError::IoError(code)`
  instead.
- `SendIO` clears `io_Flags`. A client that needs a flag the device reads
  (for example a SANA-II client that needs `SANA2IOF_RAW`) must call
  exec's `BeginIO` itself (`sys::exec_begin_io`, unsafe) instead.
