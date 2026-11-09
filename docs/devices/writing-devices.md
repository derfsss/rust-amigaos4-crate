# Writing device drivers

This page is for writing an exec **device driver** in Rust: a `.device`
that other programs open with `OpenDevice` and send I/O requests to. It
also covers the hardware side: DMA, PCI, interrupts, SANA-II network
drivers and virtio devices.

To **use** a device from a program, see [Using devices](using-devices.md)
first; it explains I/O requests, `DoIO`, `SendIO` and the rest.

| Module | What it gives you | Cargo feature |
|---|---|---|
| `device` | `IoReq`, `RequestQueue`, `Worker`, `answer_device_query`, command and error constants | `device` |
| `mem` | `DmaBuffer`, `DmaMapping` | none |
| `pci` | `Pci`, `Query`, `PciDevice`, `PciLock`, `IoRegion`, `MmioRegion` | `pci` |
| `interrupt` | `IntServer`, `ResetCallback` | none |
| `sana2` | SANA-II requests, constants and `BufferManagement` | `sana2` (turns on `device` and `utility`) |
| `virtio` | Rings, `Virtqueue`, `Transport`, `PciTransport`, `Notifier`, `virtio::net` | `virtio` |

A driver has no C runtime. Its `Cargo.toml` turns default features off:

```toml
[lib]
crate-type = ["staticlib"]

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", default-features = false, features = ["device", "dos", "timer", "pci"] }
```

Some items exist only when building for AmigaOS (they use PowerPC
instructions) and are missing from a host `cargo check` or `cargo doc`:
all of `arch`, `pci::MmioRegion`'s methods, and
`virtio::{PciTransport, ModernPci, Notifier, Regs}`.

---

## Where to start

| Starting point | What it is |
|---|---|
| `examples/ram-device` | A complete exec device: one unit backed by a 64 KiB RAM buffer, `CMD_READ`/`CMD_WRITE`/`CMD_CLEAR`/`CMD_RESET`, quick I/O. Its C glue (`device_glue.c`) provides the `Resident` structure and the device manager interface (Open, Close, Expunge, BeginIO, AbortIO) and calls the Rust functions `rust_dev_init`, `rust_dev_open`, `rust_dev_close`, `rust_dev_begin_io` and `rust_dev_abort_io`. Copy this to make a device. |
| `templates/driver` | A minimal no-C-runtime handler or driver with its own `_start` (in `driver_glue.c`), which sets `IExec` and calls `rust_handler_main`. |

`examples/ram-device` reads the request fields by hand, at fixed offsets,
and finishes everything inside `BeginIO`. The `device` module below gives
you the same fields by name, plus the pieces for a driver that queues
requests for a **unit task**.

The build: rustc makes a static library; the package's `Makefile` links
it with the C glue using `ppc-amigaos-gcc -nostartfiles -nodefaultlibs`
and `-lgcc`, so clib4 never enters the binary.

### Two rules for every driver

1. **The heap must be `MEMF_SHARED`.** A device's `Open`, `Close` and
   `BeginIO` run in the **caller's** task and may allocate there.
   `MEMF_PRIVATE` is "only visible to the allocator task". Use
   `SharedExecAllocator`:

   ```rust
   #[global_allocator]
   // MEMF_SHARED: BeginIO, Open and Close run in the callers' tasks.
   static ALLOCATOR: amigaos4::allocator::SharedExecAllocator =
       amigaos4::allocator::SharedExecAllocator;

   #[panic_handler]
   fn panic(info: &core::panic::PanicInfo) -> ! {
       amigaos4::panic::default_panic_handler(info)
   }
   ```

2. **No Forbid/Permit.** Use the crate's mutexes (`sync::Mutex`,
   `device::RequestQueue`) and atomics.

---

## The `device` module

### `IoReq`: a caller's request by name

`IoReq` wraps a pointer to a caller's `IOStdReq`. It is `Copy`: it names
the request, it does not own it. `IoReq::from_raw(ptr)` is `unsafe`: the
pointer must be a live request of at least `device::IOSTDREQ_SIZE` (48)
bytes for as long as you use it. It returns `None` for NULL.

| Accessors (`x()` reads, `set_x(v)` writes) | Field |
|---|---|
| `command` | `io_Command` (u16) |
| `flags` | `io_Flags` (u8) |
| `error` | `io_Error` (i8) |
| `actual` | `io_Actual` (u32) |
| `length` | `io_Length` (u32) |
| `data` | `io_Data` (`*mut u8`) |
| `offset` | `io_Offset` (u32) |
| `device`, `unit` | `io_Device`, `io_Unit` (driver private) |

| Method | Meaning |
|---|---|
| `is_quick()` | The caller asked for quick I/O (`IOF_QUICK`). |
| `finish(error, actual)` | **For BeginIO.** Set `io_Error` and `io_Actual`, and reply the request unless it is quick (then it is done in place). `unsafe`: the request is yours to end. |
| `mark_pending()` | Clear `IOF_QUICK` and `io_Error`, and mark the message as not yet replied, before you keep it for later. (`RequestQueue::push` does this.) |
| `reply(error, actual)` | **For a queued request.** Set the fields and reply. `unsafe`, as `finish`. |

### Quick I/O

When a caller uses `DoIO`, exec sets `IOF_QUICK` before calling your
`BeginIO`. If you finish the request inside `BeginIO` and leave the flag
set, the caller sees it done at once and no reply message is sent. If you
cannot finish at once, clear the flag (`mark_pending`), keep the request,
and `reply` it later from your unit task. `IoReq::finish` does the right
thing for a request finished inside `BeginIO`.

### Constants

| Module / item | Contents |
|---|---|
| `device::cmd` | `CMD_INVALID`, `CMD_RESET`, `CMD_READ`, `CMD_WRITE`, `CMD_UPDATE`, `CMD_CLEAR`, `CMD_STOP`, `CMD_START`, `CMD_FLUSH`, `CMD_NONSTD` (the first device-specific command), `NSCMD_DEVICEQUERY` |
| `device::ioerr` | `IOERR_SUCCESS` (0), `IOERR_OPENFAIL` (-1), `IOERR_ABORTED` (-2), `IOERR_NOCMD` (-3), `IOERR_BADLENGTH` (-4), `IOERR_BADADDRESS` (-5), `IOERR_UNITBUSY` (-6), `IOERR_SELFTEST` (-7) |
| `device::td` | trackdisk-style commands for block devices, including the 64-bit `TD_READ64`, `NSCMD_TD_READ64`, `NSCMD_TD_GETGEOMETRY64`, `HD_SCSICMD`, ... |
| `device::tderr` | trackdisk and SCSI `io_Error` values (`TDERR_*`, `HFERR_*`) |
| `IOF_QUICK`, `IOSTDREQ_SIZE`, `NSDQR_SIZE`, `NSDEVTYPE_UNKNOWN`, `NSDEVTYPE_TRACKDISK` | Flags, sizes and device types |

### `answer_device_query`

`NSCMD_DEVICEQUERY` asks a "new-style" device what it is and which
commands it supports. `answer_device_query(io, device_type, subtype,
&COMMANDS)` fills the caller's buffer and returns the `(io_Error,
io_Actual)` to finish with. The command list must be a `static` ending
in 0. `unsafe`: `io_Data`, if long enough, must be writable for
`io_Length` bytes.

### A synchronous driver

A driver that finishes every request inside `BeginIO`, like
`examples/ram-device`, needs only `IoReq`:

```rust
use amigaos4::device::{answer_device_query, cmd, ioerr, IoReq, NSDEVTYPE_UNKNOWN};
use amigaos4::sys::IORequest;

static COMMANDS: [u16; 4] = [cmd::CMD_READ, cmd::CMD_WRITE, cmd::NSCMD_DEVICEQUERY, 0];

#[no_mangle]
pub extern "C" fn rust_dev_begin_io(ior: *mut IORequest) {
    // SAFETY: BeginIO gets a live request of the size the caller opened
    // the device with (an IOStdReq here).
    let Some(io) = (unsafe { IoReq::from_raw(ior) }) else { return };
    let (error, actual) = match io.command() {
        cmd::NSCMD_DEVICEQUERY => {
            // SAFETY: the caller's io_Data is its query buffer.
            unsafe { answer_device_query(io, NSDEVTYPE_UNKNOWN, 0, &COMMANDS) }
        }
        cmd::CMD_READ | cmd::CMD_WRITE => {
            // ... move io.length() bytes at io.offset() to/from io.data() ...
            (0, io.length())
        }
        _ => (ioerr::IOERR_NOCMD, 0),
    };
    // SAFETY: this request is ours to end.
    unsafe { io.finish(error, actual) };
}
```

---

## Unit tasks: `RequestQueue` and `Worker`

Real hardware is slow. `BeginIO` runs in the caller's task and must not
wait for the hardware, so a driver hands requests to a **unit task**: a
task (usually a DOS process) of the driver's own that does the work and
replies each request when it is done. The `device` module has the two
shared pieces, both `const`-constructible so they can live in `static`s:

- **`RequestQueue`**: a unit's waiting requests plus the one in progress,
  under an exec mutex (no Forbid). `BeginIO` pushes; the unit task takes;
  `AbortIO` and `CMD_FLUSH` reach both. Every method takes the mutex, so
  they are for tasks, never interrupts.
- **`Worker`**: the unit task's work signal. The task publishes itself
  when it is ready; until then, and after it stops, `wake()` returns
  `false`, so `BeginIO` can fail a request instead of queueing it for
  nobody. It also carries a stop request.

| `RequestQueue` | Meaning |
|---|---|
| `RequestQueue::new()` | `const fn`, for a `static`. |
| `init()` | Create the mutex. `unsafe`: call it while you are provably the only user (at init or first open), never lazily on the path it protects. |
| `push(io)` | Queue a request (marks it pending first). |
| `take_next()` | Unit task: take the next request and make it current. |
| `abort_requested()` | Unit task: has AbortIO or a flush asked the current request to stop? Check between pieces of work. |
| `done()` | Unit task: the current request is finished. Call it **before** the reply, so an AbortIO after this point finds nothing and leaves the reply alone. |
| `abort(io)` | AbortIO: `Abort::Removed` (it was queued and is removed: reply it `IOERR_ABORTED`), `Abort::Flagged` (it is in progress: the unit task ends it), `Abort::NotFound` (already replied: do nothing). |
| `abort_all()` | `CMD_RESET`/`CMD_FLUSH`: flag the current request and return all queued ones, for you to reply `IOERR_ABORTED`. |
| `destroy()` | Free the mutex and reset the queue so `init` may run again. Requests still queued are forgotten, not replied: end them first. `unsafe`: no task uses the queue any more. |

| `Worker` | Meaning |
|---|---|
| `Worker::new()` | `const fn`. |
| `publish(task, mask)` | Unit task, once ready: wake me with `mask`. |
| `withdraw()` | Unit task, as it stops. |
| `is_ready()` | Is the task there? |
| `wake()` | Signal the task; `false` if it is not ready or is stopping. |
| `request_stop()` | Ask the task to stop, and wake it. |
| `stop_requested()` | Unit task: has someone asked me to stop? |

### A sketch of a queued driver

This shows how the pieces fit. It is a skeleton, not a complete driver:
the hardware work, the C glue and the open-count bookkeeping are yours.

```rust
use amigaos4::device::{cmd, ioerr, Abort, IoReq, RequestQueue, Worker};
use amigaos4::signal::{self, Signal};
use amigaos4::sys::IORequest;

static QUEUE: RequestQueue = RequestQueue::new();
static WORKER: Worker = Worker::new();

#[no_mangle]
pub extern "C" fn rust_dev_begin_io(ior: *mut IORequest) {
    // SAFETY: BeginIO gets a live IOStdReq-sized request.
    let Some(io) = (unsafe { IoReq::from_raw(ior) }) else { return };
    match io.command() {
        cmd::CMD_READ | cmd::CMD_WRITE => {
            if !WORKER.is_ready() {
                // SAFETY: ours to end.
                unsafe { io.finish(ioerr::IOERR_UNITBUSY, 0) };
                return;
            }
            QUEUE.push(io);   // no longer quick: replied later
            // The task may have stopped between the check and the push:
            // take the request back rather than leave it for nobody.
            if !WORKER.wake() && QUEUE.abort(io) == Abort::Removed {
                // SAFETY: removed from the queue: ours to end.
                unsafe { io.reply(ioerr::IOERR_UNITBUSY, 0) };
            }
        }
        cmd::CMD_FLUSH | cmd::CMD_RESET => {
            for r in QUEUE.abort_all() {
                // SAFETY: removed from the queue: ours to end.
                unsafe { r.reply(ioerr::IOERR_ABORTED, 0) };
            }
            // SAFETY: ours to end.
            unsafe { io.finish(0, 0) };
        }
        // SAFETY: ours to end.
        _ => unsafe { io.finish(ioerr::IOERR_NOCMD, 0) },
    }
}

#[no_mangle]
pub extern "C" fn rust_dev_abort_io(ior: *mut IORequest) {
    // SAFETY: AbortIO gets a request of this device.
    let Some(io) = (unsafe { IoReq::from_raw(ior) }) else { return };
    if QUEUE.abort(io) == Abort::Removed {
        // SAFETY: removed from the queue: ours to end.
        unsafe { io.reply(ioerr::IOERR_ABORTED, 0) };
    }
    // Abort::Flagged: the unit task sees abort_requested() and ends it.
}

/// The unit task's body.
fn unit_task() {
    // The signal bit must be allocated by the task that waits on it.
    let Ok(work) = Signal::alloc() else { return };
    WORKER.publish(signal::current_task(), work.mask());
    loop {
        work.wait();
        while let Some(io) = QUEUE.take_next() {
            let mut error = 0;
            let actual = io.length();
            // ... do the hardware work in pieces, checking
            //     QUEUE.abort_requested() between them ...
            if QUEUE.abort_requested() {
                error = ioerr::IOERR_ABORTED;
            }
            QUEUE.done();                       // before the reply
            // SAFETY: the current request, ours to end.
            unsafe { io.reply(error, actual) };
        }
        if WORKER.stop_requested() {
            break;
        }
    }
    WORKER.withdraw();
    for io in QUEUE.abort_all() {
        // SAFETY: removed from the queue: ours to end.
        unsafe { io.reply(ioerr::IOERR_ABORTED, 0) };
    }
    // ... tell the closer it is done (for example a SignalTarget it gave) ...
}
```

Call `QUEUE.init()` before the unit task starts (at device init, or in
the first `Open`, where you are the only user).

### Starting the unit task

A unit task outlives the `Open` call that starts it, so it must not
belong to the caller. `dos::process::ProcessBuilder::spawn_detached`
(feature `dos`) starts a closure in a DOS process that belongs to no one,
with the same start-up handshake as `task::spawn`, and returns its PID:

```rust
use amigaos4::dos::process::ProcessBuilder;

fn start_unit() -> amigaos4::Result<u32> {
    ProcessBuilder::new("mydev.device unit 0")?   // a name of its own
        .priority(5)
        .spawn_detached(unit_task)
}
```

Give it a unique name: the shutdown rule below finds it by name.

### Stopping a unit task and expunging

A device loaded from disk must unload cleanly. These rules come from the
crate's own testing:

1. **The heap is `MEMF_SHARED`** (above), or a caller's task can corrupt
   memory when it exits.
2. **Wait for the unit task to be gone.** The unit task's "I have
   stopped" signal cannot be its last instruction (without Forbid it may
   still be running your code after sending it). So the closer (the last
   `Close`) calls `WORKER.request_stop()`, then waits until
   `FindTask(name)` returns NULL, and only then frees the unit's memory
   (`QUEUE.destroy()`) or lets Expunge unload the code. While the process
   still exists, its code must stay loaded.

   ```rust
   use amigaos4::sys;
   use amigaos4::timer::{AmigaTimer, TimerUnit, TimerVal};

   /// `name` is the unit task's name, NUL-terminated.
   fn wait_until_gone(name: &[u8]) -> amigaos4::Result<()> {
       let mut t = AmigaTimer::open(TimerUnit::VBlank)?;
       // SAFETY: FindTask with a NUL-terminated name only looks it up.
       while !unsafe { sys::exec_find_task(name.as_ptr()) }.is_null() {
           t.delay(TimerVal::from_millis(20))?;
       }
       Ok(())
   }
   ```

   Check `task::can_block()` first if you are not sure the caller may
   wait.
3. **Expunge through Close.** When the last `Close` finds that an Expunge
   was deferred (`LIBF_DELEXP`, because the device was open when it was
   asked), it calls the Expunge function itself, which removes the device
   from exec's list before handing back the segment list. Returning the
   segment list from `Close` alone leaves exec's list pointing into freed
   code. `examples/ram-device/src/device_glue.c` does this.
4. **Kickstart drivers keep their tasks.** During boot an ended task stays
   findable by name for several seconds, so a driver in the Kickstart
   must not stop its unit tasks; it lives for the whole session.
5. **Test it.** `testprogs/expunge` opens a device, calls `RemDevice`
   (which defers, as it is open) and closes it; the device must then be
   gone from exec's list and load afresh on the next open.

---

## DMA the right way

**DMA** means the device itself reads or writes RAM. On AmigaOS a driver
must go through exec's `StartDMA`, `GetDMAList` and `EndDMA` for every
transfer. They:

- flush or invalidate the CPU caches correctly for the machine. Some
  machines (the Sam460 among them) do not keep caches coherent with DMA:
  without this, the device reads stale data or the CPU reads stale cache
  lines;
- give the device **physical** addresses, which can differ from the CPU's
  (virtual) addresses, and can be split into several blocks.

Never hand a device a CPU pointer, and never replace these calls with
`arch::cache_flush` plus a virtual address. The `mem` module wraps them.

### `DmaBuffer` and `DmaMapping`

| Item | Meaning |
|---|---|
| `DmaBuffer::alloc(size)` | Zeroed `MEMF_SHARED` memory. |
| `DmaBuffer::alloc_contiguous(size, align)` | One physically contiguous block, aligned to `align` (a power of two) virtually and physically. For structures a device addresses by one base address (a descriptor ring, a virtio queue). |
| `buf.map(dir)` | `StartDMA` + `GetDMAList` for the whole buffer; returns a `DmaMapping` that borrows the buffer mutably. |
| `DmaMapping::new(addr, len, dir)` | `unsafe`: map any region, such as a caller's `io_Data`. You promise the memory stays allocated and the CPU does not touch it until the mapping is dropped. |
| `mapping.entries()` | The physical blocks (`sys::DMAEntry { PhysicalAddress, BlockLength }`) in ascending address order; their lengths add up to the mapped size. |
| drop the mapping | `EndDMA` with the same address, size and flags. |
| `mapping.abort()` | End a mapping for a transfer that never started, or was aborted before the device wrote anything (`DMAF_NoModify`). |

`DmaDirection::ToDevice` means the device reads the buffer (RAM to
device); `DmaDirection::FromDevice` means the device writes it.

```rust
use amigaos4::mem::{DmaBuffer, DmaDirection, DmaMapping};

fn read_from_device(len: usize) -> amigaos4::Result<()> {
    let mut buf = DmaBuffer::alloc(len)?;
    {
        let map = buf.map(DmaDirection::FromDevice)?;
        for e in map.entries() {
            // one hardware descriptor per block:
            let _ = (e.PhysicalAddress, e.BlockLength);
        }
        // ... start the transfer, wait for the device's interrupt ...
        // If the transfer never started: map.abort();
    } // EndDMA: the CPU may now read what the device wrote
    let _first = buf.as_slice()[0];
    Ok(())
}

/// Mapping a caller's buffer directly (zero copy).
///
/// # Safety
/// `data`/`len` is the caller's io_Data, untouched until the mapping ends.
unsafe fn map_caller_buffer(data: *mut u8, len: usize) -> amigaos4::Result<()> {
    // SAFETY: the caller's contract above.
    let map = unsafe { DmaMapping::new(data, len, DmaDirection::ToDevice)? };
    let _blocks = map.entries().len();
    // ... program the device, wait for completion ...
    Ok(())
}
```

`StartDMA` refusing the region is `Err(AmigaError::DmaFailed)`; a zero
length is refused the same way.

### Long-lived mappings

Some structures are read and written by the device at any time (a
descriptor ring, a virtio queue). Map them once with `DmaMapping::new` on
`DmaBuffer::as_mut_ptr()` and keep the mapping for the structure's whole
life, dropping it before the buffer. The CPU's own accesses then need
explicit cache maintenance around the device's, as `virtio::ring` does:
it flushes the descriptor and avail areas before the device reads them,
and invalidates the used entries before the CPU reads them. That is
correct on non-coherent machines and harmless on coherent ones.
`virtio::Virtqueue` is a ready example.

### Cache-line edges

On a machine without cache coherency, cache maintenance works on whole
cache lines. If a buffer the device writes shares its first or last cache
line with other data, invalidating that line can throw away the CPU's
newer writes to the neighbouring data, and writing it back can overwrite
what the device wrote. The crate's own DMA memory avoids the question:
a `DmaBuffer` is an allocation of its own, and `alloc_contiguous` lets
you align it. For a caller's buffer whose start or end is not on a cache
line boundary (`sysinfo::cpu().cache_line` gives the size), the safe
choice is to copy through a `DmaBuffer` (a bounce buffer) rather than map
it directly. Driver code must stay correct on every supported machine,
coherent or not.

---

## PCI

Feature `pci`. The `pci` module wraps expansion.library's `pci`
interface. `Pci::get()` returns the interface (the program's `IPCI`
global if set, otherwise one the crate opens).

### Finding devices

```rust
use amigaos4::pci::{Pci, Query};

fn find_virtio_net() -> amigaos4::Result<()> {
    let pci = Pci::get()?;
    let q = Query::any().vendor(0x1AF4).device(0x1000);
    for dev in pci.devices(&q)? {
        let (bus, slot, func) = dev.address();
        amigaos4::serial_println!("{:02x}:{:02x}.{} rev {}", bus, slot, func, dev.revision());
    }
    Ok(())
}
```

| `Query` | Meaning |
|---|---|
| `Query::any()` | Every device. |
| `.vendor(id)`, `.device(id)` | Match IDs. |
| `.class(class, mask)` | Class code `0x00BBSSII` under `mask` (`0x00FF0000`: base class only). |
| `Query::at(bus, dev, func)` | One address. Exclusive: combined with any other condition the query finds nothing. |

`pci.find(&q, index)` returns the `index`th match; `pci.devices(&q)` all of
them. Each result is a `PciDevice`, freed (`FreeDevice`) on drop.

### Reading a device

Reading is allowed on any device, as a scanner does:
`vendor_id()`, `device_id()`, `revision()`, `class_code()`,
`header_type()`, `subsystem()`, `command()`, `status()`,
`interrupt_pin()`, `interrupt_line()`, `read_config8/16/32(off)`
(offsets below 256, aligned), `address()`, `flags()`, `bar(i)` / `bars()`
(a copied `Bar { index, kind, base, physical, size, flags }`),
`capabilities()`, `find_capability(id)`, and `interrupt()`, the exec
interrupt number for an `IntServer` (`None` with no interrupt pin).

### Changing a device: `PciLock`

Everything that changes the device needs it locked:
`dev.lock(LockMode::Exclusive)` (or `Shared`, for parts of a multi-function
chip) gives a `PciLock`, unlocked on drop. `lock` never waits: a device
held by another driver gives `Err(InUse)`.

| `PciLock` method | Meaning |
|---|---|
| `set_decoding(io, memory)` | Turn I/O and memory decoding on or off. |
| `enable_bus_master()` | `unsafe`: from now on, every address the device is given must be memory you own for that use, set up through `DmaMapping`. |
| `disable_bus_master()` | Stop the device mastering the bus. |
| `write_config8/16/32(off, v)` | `unsafe`: configuration writes can move BARs, bus numbers and decoding. |
| `io(index)` | An I/O BAR as a bounds-checked `IoRegion`. |
| `mmio(index)` | `unsafe`: a memory BAR as a bounds-checked `MmioRegion`. |

`IoRegion` (`read8/16/32(off)`, `write8/16/32(off, v)`, FIFO block
transfers `read_fifo8/16/32`, `write_fifo8/32`) goes through the
library's I/O calls, which handle the bridge and return little-endian
registers as native values. Every offset is checked against the BAR.

`MmioRegion` (`read8/16/32`, `write8/16/32`) uses the CPU's byte-reversed
loads and stores (`arch`). It is `unsafe` to create because **the CPU
cannot reach memory BARs behind every bridge**: on the AmigaOne's Articia
S a load there faults. Check first, for example by writing a register the
device defines and reading it back.

```rust
use amigaos4::pci::{LockMode, PciDevice};

fn bring_up(dev: &mut PciDevice) -> amigaos4::Result<()> {
    let lock = dev.lock(LockMode::Exclusive)?;
    lock.set_decoding(true, false);
    let regs = lock.io(0)?;            // BAR 0 is an I/O BAR on this device
    let status = regs.read8(0x12)?;
    let _ = status;
    Ok(())
}
```

Not wrapped: resource allocation and BAR programming (hot-plug set-up),
user data, `SetEndian`, the Zorro calls.

---

## Interrupt servers

A device signals the CPU with an **interrupt**. On AmigaOS a driver adds
an **interrupt server** to the interrupt's chain; exec calls every server
on the chain in priority order until one returns non-zero.

For a PCI device, the chain number comes from `PciDevice::interrupt()`.
`IntServer::add(int_num, pri, name, code, data)` adds the server;
dropping the `IntServer` removes it.

Rules (from the exec autodocs):

- A server runs as an interrupt: no waiting, no allocation, no locks,
  nothing that can block. `Signal` is allowed: wake the unit task and let
  it do the real work.
- A PCI INTx line is **shared**: return 0 unless your device raised the
  interrupt.
- It is **level-triggered**: before returning 1, make the device drop the
  line (usually by reading or acknowledging its status register), or the
  chain is entered again at once.
- Everything the server reads must be in place **before** `add`: it can
  run the instant it is on the chain.
- If the server uses atomics, end it with `arch::clear_reservation()`.

```rust
use core::ffi::c_void;
use amigaos4::interrupt::IntServer;
use amigaos4::signal::SignalTarget;

/// What the server needs, kept alive (and unchanged) while it is added.
struct IrqData {
    unit_task: SignalTarget,
    // ... how to read and acknowledge the device's status register ...
}

unsafe extern "C" fn irq(_ctx: *mut c_void, _sysbase: *mut c_void, data: *mut c_void) -> u32 {
    // SAFETY: `data` is the IrqData given to IntServer::add, alive
    // until the server is dropped.
    let d = unsafe { &*(data as *const IrqData) };
    let mine = true; // read the device's status: did it raise the line?
    if !mine {
        return 0;    // a shared line: let the next server look
    }
    // ... acknowledge it so the device drops the line ...
    // SAFETY: the unit task outlives the server (it is removed first).
    unsafe { d.unit_task.send() };
    1
}

/// # Safety
/// `data` must stay valid until the returned server is dropped.
unsafe fn install(int_num: u32, data: &IrqData) -> amigaos4::Result<IntServer> {
    // SAFETY: `irq` follows the interrupt rules; `data` outlives the
    // server (the caller's contract).
    unsafe { IntServer::add(int_num, 0, b"mydev.device\0", irq, data as *const IrqData as *mut c_void) }
}
```

### Reset callbacks

A device doing DMA must stop when the machine reboots, or it may write
into memory the next boot has handed out again. Register a
`ResetCallback` (see [Tasks and sync](../exec/tasks-and-sync.md#resetcallback-stop-hardware-on-reboot))
that only stops the hardware. A driver that can be unloaded drops it
first; a Kickstart driver may keep it for good with `core::mem::forget`.

---

## SANA-II network drivers

Feature `sana2`. **SANA-II** is AmigaOS's standard for network device
drivers: a protocol stack (TCP/IP) opens the driver and sends
`IOSana2Req` requests. The `sana2` module covers Revision 7 of the
standard. The `sana2ext` module (same feature) adds unofficial extensions
for batches of frames, checksum offload and segmentation; see
[sana2ext.md](sana2ext.md).

### Requests: `IOSana2Req` and `S2Req`

`IOSana2Req` is the layout (an `IORequest` followed by `ios2_WireError`,
`ios2_PacketType`, source and destination addresses, `ios2_DataLength`,
`ios2_Data`, `ios2_StatData`, `ios2_BufferManagement`). `S2Req` wraps a
pointer to one, like `IoReq`:

- `S2Req::from_raw(ptr)` (`unsafe`; a live 88-byte request),
- field accessors `wire_error`, `packet_type`, `src_addr`, `dst_addr`,
  `data_length`, `data`, `stat_data`, `buffer_management` and their
  `set_` forms,
- `src_mac()`, `dst_mac()`, `set_src_mac()`, `set_dst_mac()` for Ethernet,
- `io()` for the generic `IoReq` fields,
- `finish(error, wire_error)` (honours quick I/O) and
  `reply(error, wire_error)`, both `unsafe` as for `IoReq`.

Constants: the commands (`S2_DEVICEQUERY`, `S2_GETSTATIONADDRESS`,
`S2_CONFIGINTERFACE`, `S2_ONLINE`, ..., `S2_SANA2HOOK`), errors
(`S2ERR_*`) and wire errors (`S2WERR_*`), flags (`SANA2IOF_RAW`,
`SANA2IOF_BCAST`, ...), wire types (`S2_WIRETYPE_ETHERNET`, ...) and the
statistics structures.

### `BufferManagement`: copying packets

A SANA-II driver never touches the stack's packet buffers directly. At
`OpenDevice` the stack gives the driver **copy functions** in a tag list
(`ios2_BufferManagement` points at it): `S2_CopyToBuff`,
`S2_CopyFromBuff`, optionally `S2_PacketFilter`. A newer stack may later
send `S2_SANA2HOOK` with a hook that replaces them.

| Method | Meaning |
|---|---|
| `BufferManagement::from_tags(tags)` | `unsafe`: read the OpenDevice tag list. Follows `TAG_IGNORE`, `TAG_MORE`, `TAG_SKIP`; falls back to the 16/32-bit variants. |
| `complete()` | Both mandatory functions are present (as tags or through the hook). Refuse the open otherwise. |
| `install_hook(hook, len)` | `unsafe`: handle `S2_SANA2HOOK`. `Err((io_error, wire_error))` to refuse it. |
| `has_hook()` | The stack installed its hook. |
| `copy_to_buff(ior, cookie, from, n)` | `unsafe`: copy a received packet into the stack's buffer (a `CMD_READ`'s `ios2_Data`). `false` if the stack refused. |
| `copy_from_buff(ior, to, cookie, n)` | `unsafe`: copy a packet to send out of the stack's buffer (a `CMD_WRITE`'s `ios2_Data`). |
| `filter(ior, data)` | Ask the stack's packet filter before delivering a packet (true: deliver). Set `ios2_DataLength` and the addresses first. |

The tag functions are often 68k code; the crate calls them through
exec's `Emulate` (or natively when they say they are native), and calls
the hook through utility.library. A driver keeps one `BufferManagement`
per opener and stores its address in `ios2_BufferManagement`: every later
request carries it back as a cookie.

```rust
use amigaos4::sana2::{BufferManagement, S2Req, S2ERR_BAD_ARGUMENT, S2WERR_NULL_POINTER};
use amigaos4::sys::TagItem;

/// In the driver's Open: take the stack's copy functions.
///
/// # Safety
/// `ior` is the IOSana2Req being opened.
unsafe fn open_buffers(ior: S2Req) -> Option<alloc::boxed::Box<BufferManagement>> {
    let tags = ior.buffer_management() as *const TagItem;
    // SAFETY: at OpenDevice, ios2_BufferManagement is the stack's tag list.
    let bm = unsafe { BufferManagement::from_tags(tags) };
    if !bm.complete() {
        // The open fails: say why in the request (OpenDevice is not replied).
        ior.io().set_error(S2ERR_BAD_ARGUMENT);
        ior.set_wire_error(S2WERR_NULL_POINTER);
        return None;
    }
    let mut b = alloc::boxed::Box::new(bm);
    ior.set_buffer_management(&mut *b as *mut BufferManagement as *mut _); // the cookie
    Some(b)
}
```

(Your C glue then fails the OpenDevice call; the sketch only shows the
buffer-management part.)

---

## virtio devices

Feature `virtio`. **virtio** is the standard virtual hardware of
emulators such as QEMU. The `virtio` module holds the parts every virtio
driver needs; what goes in the buffers stays in your driver.

| Item | Meaning |
|---|---|
| `virtio::ring` | The split virtqueue layout (`RingLayout`, `SplitRing`) with descriptor, avail and used operations, in either byte order (`Endian::Native` for legacy, `Endian::Little` for modern), with the cache maintenance around the device's accesses. Constants `DESC_F_NEXT`, `DESC_F_WRITE`, `DESC_F_INDIRECT`. |
| `Virtqueue::new(size, endian)` | A ring in one physically contiguous, 4096-aligned `DmaBuffer`, mapped for the device for its whole life. `ring()`, `layout()`, `desc_phys()`, `avail_phys()`, `used_phys()`. |
| `Transport` | The trait for a device's registers: reset, status, feature negotiation, `queue_max`, `setup_queue`, `notify`, `read_isr`, device configuration reads, `ring_endian()`. All register methods are `unsafe`. |
| `LegacyPci` | The legacy interface (I/O BAR 0); works behind every PCI bridge AmigaOS 4 runs on. |
| `ModernPci` | The modern (virtio 1.x) interface over memory BARs. Target only. |
| `PciTransport::open(dev)` | `unsafe`: pick the modern interface if it has one and a probe shows the CPU can reach its registers, otherwise the legacy one. Returns the transport and, on fallback, why. Target only. |
| `PciTransport::regs()` → `Regs` | Copied register addresses for an interrupt server or reset callback: `read_isr()`, `write_status(0)` (reset). |
| `PciTransport::notifier(q)` → `Notifier` | A queue's notify register, for another task to kick the device after posting buffers. |
| `init_device(&mut t, wanted)` / `driver_ok(&mut t)` | The standard initialisation sequence: reset, ACKNOWLEDGE, DRIVER, negotiate features (returns those accepted), FEATURES_OK; then, after the queues are set up, DRIVER_OK. |
| `enable_pci(dev, memory)` | `unsafe`: turn on bus mastering and decoding, which firmware may leave off. |
| Constants | `PCI_VENDOR`, `TYPE_NET`/`TYPE_BLOCK`/`TYPE_SCSI`/..., `pci_id_modern`, `pci_id_transitional`, `STATUS_*`, `F_VERSION_1`, `F_INDIRECT_DESC`, `F_EVENT_IDX`, `ISR_QUEUE`, `ISR_CONFIG`. |
| `virtio::net` | virtio-net: queue numbers (`RX_QUEUE`, `TX_QUEUE`, `CTRL_QUEUE`), feature bits (`F_MAC`, `F_STATUS`, `F_MRG_RXBUF`, ...), configuration offsets (`CFG_MAC`, `CFG_STATUS`, ...), the packet header (`hdr_len`, `rx_num_buffers`) and control commands (`mac_table`). |
| `virtio::scsi` | virtio-scsi definitions. |

The bring-up order, in outline (AmigaOS target only, since it uses
`PciTransport`):

```rust
use amigaos4::pci::{LockMode, PciDevice};
use amigaos4::virtio::{self, net, PciTransport, Transport, Virtqueue};

fn bring_up_net(dev: &mut PciDevice) -> amigaos4::Result<()> {
    let lock = dev.lock(LockMode::Exclusive)?;
    // SAFETY: a live device we hold locked for as long as the transport lives.
    let (mut t, _why_legacy) = unsafe { PciTransport::open(lock.device().as_ptr())? };
    // SAFETY: the transport addresses the locked device; one task uses it.
    unsafe {
        let _features = virtio::init_device(&mut t, net::F_MAC | net::F_STATUS)?;
        let size = t.queue_max(net::RX_QUEUE).min(256);
        let rx = Virtqueue::new(size, t.ring_endian())?;
        t.setup_queue(net::RX_QUEUE, &rx)?;
        // ... the transmit queue the same way; fill receive buffers ...
        virtio::driver_ok(&mut t)?;
        // keep `t`, `rx` and the lock for the driver's life
    }
    Ok(())
}
```

Before adding the interrupt server, copy `t.regs()` into the server's
data: the server calls `regs.read_isr()`, which also clears the interrupt
and drops the line, and returns 0 when it reads 0 (another device on a
shared line).

---

## Checklist

- [ ] `default-features = false`, a static library, `panic = "abort"`.
- [ ] `SharedExecAllocator` as the global allocator.
- [ ] `BeginIO`: finish quick requests with `IoReq::finish`; queue the rest
      with `RequestQueue::push` and `Worker::wake`.
- [ ] `AbortIO`: `RequestQueue::abort`; reply `Removed` requests with
      `IOERR_ABORTED`.
- [ ] Unit task: `done()` before `reply()`; check `abort_requested()`.
- [ ] DMA only through `DmaBuffer`/`DmaMapping`.
- [ ] Interrupt server: shared line (return 0 if not yours), drop the
      line, `clear_reservation()` if it uses atomics.
- [ ] Reset callback that stops DMA.
- [ ] Shutdown: `request_stop()`, wait for `FindTask(name)` to be NULL,
      then free; deferred Expunge run from `Close`.
- [ ] Test unloading with `testprogs/expunge`.
