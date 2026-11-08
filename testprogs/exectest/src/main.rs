//! exectest: the `task`, `signal` and `sync` modules on a real exec.
//!
//!   exectest [rounds]
//!
//! `rounds` (default 1000) is the number of spawn-and-join rounds per child
//! priority. Prints one line per check and "exectest: N/M passed"; returns
//! 0 when every check passed, 20 otherwise. The children call no DOS
//! (exec tasks may not); only the main task prints.
//!
//! 1. spawn + join, children that return at once, rounds x 3 priorities
//!    (5 above, equal to and 5 below the main task): every value comes
//!    back. Children that busy-wait run 1 below the main task (above it
//!    they would starve it).
//! 2. 8 children alive at once, joined in reverse order.
//! 3. A handle dropped without join waits for its child.
//! 4. Mutex<T>: 8 children x 20000 locked read-pause-write increments: the
//!    total is exact, and the lock changed hands more than 8 times (the
//!    children really interleaved: more than 2 per child). lock, try_lock and lock_or_signal by
//!    the holder are refused. Against another holder, lock_or_signal gives
//!    up on a signal already set and on one that arrives while it waits.
//!    (Shown first: what exec's own MutexAttemptWithSignal answers the
//!    holder of a non-recursive mutex.)
//! 5. Semaphore<T>: write-then-write and write-then-read by the owner are
//!    refused (WouldDeadlock); 4 writers move a pair together while 4
//!    readers check it is never seen apart. A writer half-way through an
//!    update ticks every reader (CTRL-E, readers one priority above): their
//!    first try_read must be refused (counted), and the read that follows
//!    must see the pair together. Writer-to-writer hand-offs are shown.
//! 6. msg: 1000 messages from a child; 500 requests answered by a child
//!    server (replied explicitly and by drop), a send to a dropped port
//!    comes back, a port dropped with messages queued replies to them.
//!    devio: a missing device, timer.device DoIO / send + abort / drop in
//!    flight / two requests at once, and 4 KiB through RAM:ram.device (the
//!    crate's example, copied there first).
//! 6a. pool/mem/named/hook: a MemPool of 2000 mixed blocks; an ItemPool
//!    shared by 8 children, and its item limit; a ResourceMap filled,
//!    disjoint, aligned; CopyMem/CopyMemQuick; MemLock; TypeOfMem; named
//!    memory made, duplicate refused, locked, a child's lock waiting for
//!    the main task's, freed;
//!    a Hook called through CallHookPkt.
//! 6c. list/avl/library/softint/sysinfo/stack/debug: NameList order,
//!    Enqueue, FindName/FindIName, remove, MoveList, pops; AvlMap of 2000
//!    keys in order, replace, remove half, range_from; utility.library and
//!    its interface, missing ones refused; a SoftInt caused 100 times, each
//!    run before Cause returns; GetCPUInfo; 600 KiB of recursion on a stack
//!    of its own; this function's debug symbol and a backtrace.
//! 6d. bids (Procure/Vacate): granted on a free semaphore, the guard
//!    releases; the owner's bids refused; a shared bid beside a shared
//!    lock; against another task's lock pending, then granted through its
//!    signal; a pending bid and a granted untaken one dropped (Vacate)
//!    leave the lock free; the ports' signals freed. MemHandler: a failed
//!    huge allocation calls it with the request size, not after the drop.
//!    Debug output watch: a signal after a debug print, none after the end.
//! 6b. Signal: Signal::alloc/drop, take, SignalTarget::send to self.
//! 7. can_block, cpu_count, set_priority.
//! 8. After it all: the main task has as many free signal bits as at the
//!    start, and free memory around one more set of spawn rounds (after a
//!    warm-up) is shown.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use amigaos4::allocator::SharedExecAllocator;
use amigaos4::devio::{Device, Request};
use amigaos4::avl::AvlMap;
use amigaos4::hook::Hook;
use amigaos4::interrupt::{MemHandler, SoftInt};
use amigaos4::library::Library;
use amigaos4::list::NameList;
use amigaos4::{stack, sysinfo};
use amigaos4::mem;
use amigaos4::msg::{Message, Port, Sender};
use amigaos4::named;
use amigaos4::pool::{ItemPool, MemPool, ResourceMap};
use amigaos4::error::AmigaError;
use amigaos4::signal::{self, Signal};
use amigaos4::sync::{Mutex, Semaphore};
use amigaos4::sys;
use amigaos4::task::{self, Builder};

// Children free what the main task allocated (the Arcs), so the heap is
// MEMF_SHARED.
#[global_allocator]
static ALLOCATOR: SharedExecAllocator = SharedExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

struct Out;
impl Write for Out {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buf = [0u8; 128];
        for chunk in s.as_bytes().chunks(buf.len() - 1) {
            buf[..chunk.len()].copy_from_slice(chunk);
            buf[chunk.len()] = 0;
            // SAFETY: a NUL-terminated buffer on the stack.
            unsafe { sys::dos_put_str(buf.as_ptr() as _) };
        }
        Ok(())
    }
}
macro_rules! say {
    ($($arg:tt)*) => {{ let _ = writeln!(Out, $($arg)*); }};
}

struct Tally {
    pass: u32,
    total: u32,
}
impl Tally {
    fn check(&mut self, ok: bool, what: fmt::Arguments) {
        self.total += 1;
        if ok {
            self.pass += 1;
        }
        say!("{} {}", if ok { "PASS" } else { "FAIL" }, what);
    }
}

/// Burn some time without calling the OS.
fn pause(n: u32) {
    for i in 0..n {
        core::hint::black_box(i);
    }
}

/// Free signal bits of the calling task: allocate until refused, free them.
fn free_signal_bits() -> u32 {
    let mut got = Vec::new();
    while let Ok(s) = Signal::alloc() {
        got.push(s);
    }
    got.len() as u32
}

fn avail() -> u32 {
    // SAFETY: AvailMem only reads.
    unsafe { sys::exec_avail_mem(0) }
}

/// The main task's priority plus `d`. A child that busy-waits must be
/// below the main task, or the main task never runs again (a Shell job
/// may run below 0, so an absolute 0 can be above it).
fn rel(d: i8) -> i8 {
    task::priority().saturating_add(d)
}

/// rounds spawn+join of children that return at once, at the main task's
/// priority plus `d`.
fn spawn_rounds(rounds: u32, d: i8) -> Result<u32, AmigaError> {
    let mut good = 0;
    for i in 0..rounds {
        let h = Builder::new(b"exectest child\0").priority(rel(d)).stack(16384).spawn(move || i.wrapping_mul(3) + 1)?;
        if h.join()? == i.wrapping_mul(3) + 1 {
            good += 1;
        }
    }
    Ok(good)
}

fn test_spawn(t: &mut Tally, rounds: u32) {
    for (pri, label) in [(5i8, "above"), (0, "equal"), (-5, "below")] {
        match spawn_rounds(rounds, pri) {
            Ok(good) => t.check(good == rounds, format_args!("spawn+join {} children {} the parent: {}/{}", rounds, label, good, rounds)),
            Err(e) => t.check(false, format_args!("spawn+join {} the parent: {}", label, e)),
        }
    }

    // 8 alive at once, joined in reverse.
    let gate = Arc::new(AtomicBool::new(false));
    let mut hs = Vec::new();
    let mut spawned = true;
    for k in 0..8u32 {
        let g = gate.clone();
        match Builder::new(b"exectest kid\0").priority(rel(-1)).spawn(move || {
            while !g.load(Ordering::Acquire) {
                pause(1000);
            }
            k * k
        }) {
            Ok(h) => hs.push(h),
            Err(_) => spawned = false,
        }
    }
    let alive = hs.iter().filter(|h| !h.is_finished()).count();
    gate.store(true, Ordering::Release);
    let mut good = 0;
    let mut k = hs.len() as u32;
    while let Some(h) = hs.pop() {
        k -= 1;
        if h.join() == Ok(k * k) {
            good += 1;
        }
    }
    t.check(spawned && alive == 8 && good == 8, format_args!("8 children alive at once ({} waiting), joined in reverse: {}/8", alive, good));

    // Drop without join.
    let flag = Arc::new(AtomicU32::new(0));
    {
        let f = flag.clone();
        let _h = Builder::new(b"exectest dropped\0").priority(rel(-5)).spawn(move || {
            pause(2_000_000);
            f.store(7, Ordering::Release);
        });
    }
    t.check(flag.load(Ordering::Acquire) == 7, format_args!("a dropped handle waited for its child (flag {})", flag.load(Ordering::Acquire)));

    // A name without its NUL is refused.
    let r = task::spawn(b"no nul", || 0u32);
    t.check(matches!(r, Err(AmigaError::NotNulTerminated)), format_args!("a name without NUL is refused"));
}

/// What exec itself answers when the holder of a non-recursive mutex calls
/// MutexAttemptWithSignal with one of the signals already set (a scratch
/// mutex, freed afterwards; shown, not judged: the safe Mutex never lets
/// this call through).
fn probe_exec_attempt_with_signal() {
    let tags = [
        sys::TagItem { ti_Tag: sys::ASOMUTEX_RECURSIVE, ti_Data: 0 },
        sys::TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
    ];
    let Ok(s) = Signal::alloc() else { return };
    // SAFETY: a scratch mutex this function creates, locks and frees; the
    // signal is this task's own bit.
    unsafe {
        let mx = sys::exec_alloc_sys_object(sys::ASOT_MUTEX, tags.as_ptr());
        if mx.is_null() {
            return;
        }
        sys::exec_mutex_obtain(mx);
        s.target().send();
        let r = sys::exec_mutex_attempt_with_signal(mx, s.mask());
        s.clear();
        say!(
            "info: exec MutexAttemptWithSignal by the holder of a non-recursive mutex, signal 0x{:08x} set: returned 0x{:08x}{}",
            s.mask(),
            r,
            if r == 0 { " (granted it again)" } else { "" }
        );
        sys::exec_free_sys_object(sys::ASOT_MUTEX, mx);
    }
}

/// Shared by the lock tests: a counter and how often the lock changed
/// hands (a holder that is not the previous one). Hand-offs show the
/// children really ran interleaved; without them an exact total proves
/// nothing.
struct Counted {
    n: u32,
    last: u32,
    handoffs: u32,
}

fn test_mutex(t: &mut Tally) {
    const KIDS: u32 = 8;
    const EACH: u32 = 20_000;
    let m = match Mutex::new(Counted { n: 0, last: u32::MAX, handoffs: 0 }) {
        Ok(m) => Arc::new(m),
        Err(e) => return t.check(false, format_args!("Mutex::new: {}", e)),
    };
    let mut hs = Vec::new();
    for k in 0..KIDS {
        let m = m.clone();
        if let Ok(h) = Builder::new(b"exectest mutex\0").priority(rel(-1)).spawn(move || {
            for _ in 0..EACH {
                let mut g = m.lock().expect("mutex");
                if g.last != k {
                    g.last = k;
                    g.handoffs += 1;
                }
                let v = g.n;
                pause(1000); // a read-to-write window the scheduler can land in
                g.n = v + 1;
            }
        }) {
            hs.push(h);
        }
    }
    let n = hs.len() as u32;
    for h in hs {
        let _ = h.join();
    }
    let (total, handoffs) = match m.lock() {
        Ok(g) => (g.n, g.handoffs),
        Err(_) => (0, 0),
    };
    t.check(
        n == KIDS && total == KIDS * EACH && handoffs > 2 * KIDS,
        format_args!(
            "Mutex: {} children x {} locked increments = {} (want {}), {} hand-offs (want more than {})",
            n, EACH, total, KIDS * EACH, handoffs, 2 * KIDS
        ),
    );

    // The holder asking again.
    {
        let _g = m.lock();
        t.check(matches!(m.lock(), Err(AmigaError::WouldDeadlock)), format_args!("Mutex: lock by the holder is refused"));
        t.check(m.try_lock().is_none(), format_args!("Mutex: try_lock by the holder is None"));
        t.check(
            matches!(m.lock_or_signal(sys::SIGBREAKF_CTRL_C), Err(AmigaError::WouldDeadlock)),
            format_args!("Mutex: lock_or_signal by the holder is refused"),
        );
    }
    t.check(m.try_lock().is_some(), format_args!("Mutex: free again after the guard is dropped"));

    // lock_or_signal against another holder: a signal already set, then
    // one that arrives while waiting.
    let (Ok(held), Ok(abort), Ok(pre)) = (Signal::alloc(), Signal::alloc(), Signal::alloc()) else {
        return t.check(false, format_args!("Signal::alloc for the lock_or_signal test"));
    };
    let release = Arc::new(AtomicBool::new(false));
    let (tm, rel_flag, t_held, t_abort) = (m.clone(), release.clone(), held.target(), abort.target());
    let h = Builder::new(b"exectest holder\0").priority(rel(-1)).spawn(move || {
        let g = tm.lock().expect("holder");
        // SAFETY: the main task owns these bits until it has joined us.
        unsafe { t_held.send() };
        pause(200_000);
        // SAFETY: as above.
        unsafe { t_abort.send() };
        while !rel_flag.load(Ordering::Acquire) {
            pause(1000);
        }
        drop(g);
    });
    let Ok(h) = h else {
        return t.check(false, format_args!("spawn the holder"));
    };
    held.wait();
    // SAFETY: our own task and bit.
    unsafe { pre.target().send() };
    let r1 = m.lock_or_signal(pre.mask());
    t.check(
        matches!(r1, Ok(Err(mask)) if mask & pre.mask() != 0),
        format_args!("Mutex: lock_or_signal gives up on a signal already set (held by another task)"),
    );
    drop(r1);
    pre.clear();
    let r2 = m.lock_or_signal(abort.mask());
    t.check(
        matches!(r2, Ok(Err(mask)) if mask & abort.mask() != 0),
        format_args!("Mutex: lock_or_signal gives up on a signal that arrives while it waits"),
    );
    drop(r2);
    abort.clear();
    release.store(true, Ordering::Release);
    let _ = h.join();
    t.check(m.try_lock().is_some(), format_args!("Mutex: free after the holder ended"));
}

fn test_semaphore(t: &mut Tally) {
    let s = match Semaphore::new(((0u32, 0u32), u32::MAX, 0u32)) {
        Ok(s) => Arc::new(s),
        Err(e) => return t.check(false, format_args!("Semaphore::new: {}", e)),
    };
    {
        let w = s.write();
        t.check(w.is_ok(), format_args!("Semaphore: write"));
        t.check(matches!(s.write(), Err(AmigaError::WouldDeadlock)), format_args!("Semaphore: write by the writer is refused"));
        t.check(matches!(s.read(), Err(AmigaError::WouldDeadlock)), format_args!("Semaphore: read by the writer is refused"));
        t.check(matches!(s.try_write(), Err(AmigaError::WouldDeadlock)), format_args!("Semaphore: try_write by the writer is refused"));
    }
    {
        let r1 = s.read();
        let r2 = s.read();
        t.check(r1.is_ok() && r2.is_ok(), format_args!("Semaphore: two shared locks by one task"));
    }

    // Readers wait for a CTRL-E tick, one priority above the writers. A
    // writer half-way through an update (the pair apart, the lock held)
    // ticks every reader: the reader tries to read at once, which must fail
    // (counted as contended), then blocks until the write is done and must
    // see the pair together. This does not rely on time slicing between
    // busy tasks of equal priority, which exec here barely does.
    const W: u32 = 4;
    const R: u32 = 4;
    const EACH: u32 = 2_000;
    let apart = Arc::new(AtomicU32::new(0));
    let reads = Arc::new(AtomicU32::new(0));
    let contended = Arc::new(AtomicU32::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let mut readers = Vec::new();
    for _ in 0..R {
        let (s, apart, reads, contended, stop) = (s.clone(), apart.clone(), reads.clone(), contended.clone(), stop.clone());
        if let Ok(h) = Builder::new(b"exectest reader\0").priority(rel(-1)).spawn(move || loop {
            signal::wait(sys::SIGBREAKF_CTRL_E);
            if stop.load(Ordering::Acquire) {
                break;
            }
            let g = match s.try_read().expect("reader") {
                Some(g) => g,
                None => {
                    contended.fetch_add(1, Ordering::Relaxed);
                    s.read().expect("reader")
                }
            };
            if g.0 .0 != g.0 .1 {
                apart.fetch_add(1, Ordering::Relaxed);
            }
            drop(g);
            reads.fetch_add(1, Ordering::Relaxed);
        }) {
            readers.push(h);
        }
    }
    let ticks: Arc<Vec<signal::SignalTarget>> =
        Arc::new(readers.iter().map(|h| signal::SignalTarget::new(h.task(), sys::SIGBREAKF_CTRL_E)).collect());
    let mut writers = Vec::new();
    for k in 0..W {
        let (s, ticks) = (s.clone(), ticks.clone());
        if let Ok(h) = Builder::new(b"exectest writer\0").priority(rel(-2)).spawn(move || {
            for _ in 0..EACH {
                let mut g = s.write().expect("writer");
                if g.1 != k {
                    g.1 = k;
                    g.2 += 1;
                }
                g.0 .0 += 1;
                for t in ticks.iter() {
                    // SAFETY: the readers live until the main task stops
                    // them, after every writer has been joined.
                    unsafe { t.send() };
                }
                pause(1000);
                g.0 .1 += 1;
            }
        }) {
            writers.push(h);
        }
    }
    let nw = writers.len() as u32;
    let nr = readers.len() as u32;
    for h in writers {
        let _ = h.join();
    }
    stop.store(true, Ordering::Release);
    for t in ticks.iter() {
        // SAFETY: the readers are alive until joined below.
        unsafe { t.send() };
    }
    for h in readers {
        let _ = h.join();
    }
    let ((a, b), _, handoffs) = *s.read().expect("final read");
    let (apart, reads, contended) = (apart.load(Ordering::Relaxed), reads.load(Ordering::Relaxed), contended.load(Ordering::Relaxed));
    t.check(
        nw == W && nr == R && a == W * EACH && b == W * EACH && apart == 0 && contended > 0,
        format_args!(
            "Semaphore: {} writers x {} = ({}, {}); {} readers, {} reads, {} of them refused at first because a write was half done; the pair seen apart {} times (want more than 0 refused, 0 apart)",
            nw, EACH, a, b, nr, reads, contended, apart
        ),
    );
    say!("info: Semaphore: the lock passed from one writer to another {} times", handoffs);
}

/// Procure/Vacate through Semaphore::bid_write/bid_read.
fn test_bids(t: &mut Tally) {
    let s = match Semaphore::new(0u32) {
        Ok(s) => Arc::new(s),
        Err(e) => return t.check(false, format_args!("bids: Semaphore::new: {}", e)),
    };
    let bits0 = free_signal_bits();

    // A free semaphore: granted at once.
    match s.bid_write() {
        Ok(b) => {
            let mut g = b.wait();
            *g += 1;
            // Who exec records as the owner of a Procured lock decides
            // whether the holder's own write() is caught or would block.
            let again = s.try_write();
            say!("info: bids: try_write by the holder of a granted bid: {}", match &again {
                Err(AmigaError::WouldDeadlock) => "WouldDeadlock (ss_Owner is the task)",
                Ok(None) => "busy (ss_Owner is not the task)",
                Ok(Some(_)) => "GRANTED",
                Err(_) => "error",
            });
            t.check(!matches!(again, Ok(Some(_))), format_args!("bids: a granted write bid is not granted twice to its task"));
        }
        Err(e) => t.check(false, format_args!("bids: bid_write on a free semaphore: {}", e)),
    }
    t.check(
        matches!(s.try_write(), Ok(Some(ref g)) if **g == 1),
        format_args!("bids: the guard from a bid releases the lock (try_write succeeds after)"),
    );

    // The owner's bids are refused, like its write/read.
    {
        let _w = s.write();
        t.check(matches!(s.bid_write(), Err(AmigaError::WouldDeadlock)), format_args!("bids: bid_write by the writer is refused"));
        t.check(matches!(s.bid_read(), Err(AmigaError::WouldDeadlock)), format_args!("bids: bid_read by the writer is refused"));
    }

    // Shared bids beside a shared lock of this task: granted.
    {
        let _r = s.read();
        let b = s.bid_read();
        t.check(matches!(b.map(|b| b.take().is_ok()), Ok(true)), format_args!("bids: bid_read beside a shared lock is granted at once"));
    }

    // Against another task's lock: pending, then granted when it lets go;
    // and a pending bid cancelled by drop.
    let (Ok(held), Ok(other)) = (Signal::alloc(), Signal::alloc()) else {
        return t.check(false, format_args!("bids: Signal::alloc"));
    };
    for cancel in [false, true] {
        let release = Arc::new(AtomicBool::new(false));
        let (ts, rf, th) = (s.clone(), release.clone(), held.target());
        let h = Builder::new(b"exectest bid holder\0").priority(rel(-1)).spawn(move || {
            let mut g = ts.write().expect("holder");
            *g += 10;
            // SAFETY: the main task owns this bit until it has joined us.
            unsafe { th.send() };
            while !rf.load(Ordering::Acquire) {
                pause(1000);
            }
            drop(g);
        });
        let Ok(h) = h else {
            return t.check(false, format_args!("bids: spawn the holder"));
        };
        held.wait();
        let mut bid = match s.bid_write() {
            Ok(b) => b,
            Err(e) => {
                release.store(true, Ordering::Release);
                let _ = h.join();
                return t.check(false, format_args!("bids: bid_write against a holder: {}", e));
            }
        };
        let pending = !bid.is_granted();
        if cancel {
            drop(bid);
            release.store(true, Ordering::Release);
            let _ = h.join();
            t.check(
                pending && matches!(s.try_write(), Ok(Some(_))),
                format_args!("bids: a pending bid dropped (Vacate) leaves the lock free once the holder lets go"),
            );
        } else {
            // Wait on the bid's signal together with another: the bid's
            // must be the one that wakes us.
            release.store(true, Ordering::Release);
            let got = signal::wait(bid.signal_mask() | other.mask());
            let granted = bid.is_granted();
            let r = bid.take();
            t.check(
                pending && got & other.mask() == 0 && granted && matches!(r, Ok(ref g) if **g == 11),
                format_args!("bids: pending while held, then granted via its signal when the holder lets go (value {:?})", r.as_ref().map(|g| **g).ok()),
            );
            drop(r);
            let _ = h.join();
        }
    }

    // A granted bid dropped without taking it releases the lock (Vacate).
    if let Ok(mut b) = s.bid_write() {
        let mut n = 0;
        while !b.is_granted() && n < 1000 {
            n += 1;
            pause(100);
        }
        drop(b);
        t.check(matches!(s.try_write(), Ok(Some(_))), format_args!("bids: a granted bid dropped untaken releases the lock"));
    } else {
        t.check(false, format_args!("bids: bid_write for the drop test"));
    }
    drop((held, other));
    let bits1 = free_signal_bits();
    t.check(bits1 == bits0, format_args!("bids: the bids' port signals were all freed ({} / {})", bits1, bits0));
}

struct MemSeen {
    calls: AtomicU32,
    size: AtomicU32,
}

unsafe extern "C" fn mem_handler(_sysbase: *mut core::ffi::c_void, req: *const sys::MemHandlerData, data: *mut core::ffi::c_void) -> i32 {
    // SAFETY: data is the MemSeen the test keeps alive across the handler;
    // req is exec's MemHandlerData for this call.
    let (d, r) = unsafe { (&*(data as *const MemSeen), *req) };
    d.calls.fetch_add(1, Ordering::Relaxed);
    d.size.store(r.memh_RequestSize, Ordering::Relaxed);
    sys::MEM_DID_NOTHING
}

fn test_mem_handler(t: &mut Tally) {
    let seen = MemSeen { calls: AtomicU32::new(0), size: AtomicU32::new(0) };
    // SAFETY: the handler only stores into atomics of `seen`, which
    // outlives it (dropped first, below); it allocates nothing.
    let h = unsafe { MemHandler::add(50, b"exectest memhandler\0", mem_handler, &seen as *const MemSeen as *mut core::ffi::c_void) };
    let h = match h {
        Ok(h) => h,
        Err(e) => return t.check(false, format_args!("MemHandler::add: {}", e)),
    };
    const HUGE: u32 = 0x7000_0000;
    // SAFETY: an allocation that cannot succeed; freed if it somehow does.
    let p = unsafe { sys::exec_alloc_vec(HUGE, 0) };
    if !p.is_null() {
        // SAFETY: the block just allocated.
        unsafe { sys::exec_free_vec(p) };
    }
    let (calls, size) = (seen.calls.load(Ordering::Relaxed), seen.size.load(Ordering::Relaxed));
    drop(h);
    t.check(
        p.is_null() && calls >= 1 && size >= HUGE,
        format_args!("MemHandler: a failed {:#x}-byte allocation called the handler ({} calls, request {:#x})", HUGE, calls, size),
    );
    // SAFETY: as above, with the handler removed.
    let p = unsafe { sys::exec_alloc_vec(HUGE, 0) };
    t.check(p.is_null() && seen.calls.load(Ordering::Relaxed) == calls, format_args!("MemHandler: not called after the drop"));
}

fn test_debug_watch(t: &mut Tally) {
    let (Ok(d), Ok(sig)) = (amigaos4::debug::Debug::open(), Signal::alloc()) else {
        return t.check(false, format_args!("debug watch: open or Signal::alloc"));
    };
    let w = match d.watch_output(&sig) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("debug watch: watch_output: {}", e)),
    };
    sig.clear();
    amigaos4::serial_println!("exectest: a line for the debug output watch");
    let mut ticks = 0;
    while !sig.take() && ticks < 100 {
        // SAFETY: the main task is a Process; Delay(1) is 1/50 s.
        unsafe { sys::dos_delay(1) };
        ticks += 1;
    }
    t.check(ticks < 100, format_args!("debug watch: the signal came after a debug print ({} ticks)", ticks));
    drop(w);
    sig.clear();
    amigaos4::serial_println!("exectest: a line after the watch ended");
    // SAFETY: as above.
    unsafe { sys::dos_delay(10) };
    t.check(!sig.take(), format_args!("debug watch: no signal after the watch ended"));
}

/// A spawned task that panics: its join gives an error, what it had
/// registered is released (track), the program goes on.
fn test_task_panic(t: &mut Tally) {
    let before = amigaos4::track::count(signal::current_task());
    let h = Builder::new(b"exectest panicker\0").priority(rel(-1)).spawn(|| -> u32 {
        let s = Signal::alloc();
        let p = amigaos4::port::AmigaMsgPort::new();
        let n = amigaos4::track::count(signal::current_task()) as u32;
        if core::hint::black_box(s.is_ok() && p.is_ok()) {
            panic!("exectest: a spawned task panics on purpose ({} registered)", n);
        }
        n
    });
    let r = h.map(|h| h.join());
    let after = amigaos4::track::count(signal::current_task());
    t.check(
        matches!(r, Ok(Err(_))) && after == before,
        format_args!("track: a spawned task that panics gives a join error, the main task's registrations unchanged ({} / {})", before, after),
    );
    // The same task again: the machine and the scheduler are fine.
    let again = Builder::new(b"exectest after\0").spawn(|| 7u32).and_then(|h| h.join());
    t.check(again == Ok(7), format_args!("track: a task spawned after the panic runs normally ({:?})", again));
}

fn test_signal_misc(t: &mut Tally) {
    match Signal::alloc() {
        Ok(s) => {
            t.check(!s.take(), format_args!("Signal: a new bit ({}) starts clear", s.bit()));
            // SAFETY: our own task and bit.
            unsafe { s.target().send() };
            t.check(signal::peek(s.mask()) != 0, format_args!("Signal: send to self sets it"));
            t.check(s.take() && !s.take(), format_args!("Signal: take clears it"));
        }
        Err(e) => t.check(false, format_args!("Signal::alloc: {}", e)),
    }
    t.check(task::can_block(), format_args!("can_block in the main task"));
    let n = task::cpu_count();
    t.check(n >= 1, format_args!("cpu_count = {}", n));
    let old = task::set_priority(1);
    let back = task::set_priority(old);
    t.check(back == 1, format_args!("set_priority returns the old priority ({} -> 1 -> {})", old, old));
}

// ---------------------------------------------------------------------------
// Group A: msg (typed ports and messages), devio (device I/O)
// ---------------------------------------------------------------------------

/// Milliseconds since `t0` (a timebase reading).
fn ms_since(t0: u64) -> u64 {
    let mut hz = 0u64;
    let tags = [
        sys::TagItem { ti_Tag: sys::GCIT_TIMEBASE_SPEED, ti_Data: &mut hz as *mut u64 as u32 },
        sys::TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: the tag points at a u64 on this stack.
    unsafe { sys::exec_get_cpuinfo(tags.as_ptr()) };
    (amigaos4::arch::timebase() - t0) * 1000 / hz.max(1)
}

struct Work {
    a: u32,
    b: u32,
    sum: u32,
}

fn test_msg(t: &mut Tally) {
    // One-way: a child sends 1000 numbers; the main task adds them up.
    let port: Port<u32> = match Port::new() {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("Port::new: {}", e)),
    };
    let tx = port.sender();
    let h = Builder::new(b"exectest msg sender\0").priority(rel(-1)).spawn(move || {
        let mut refused = 0u32;
        for i in 1..=1000u32 {
            match Message::new(i) {
                Ok(m) => {
                    if tx.send(m).is_err() {
                        refused += 1;
                    }
                }
                Err(_) => refused += 1,
            }
        }
        refused
    });
    let mut sum = 0u64;
    let mut n = 0;
    while n < 1000 {
        match port.recv() {
            Ok(m) => {
                sum += *m as u64;
                n += 1;
            }
            Err(_) => break,
        }
    }
    let refused = h.map(|h| h.join().unwrap_or(1)).unwrap_or(1);
    t.check(n == 1000 && sum == 500_500 && refused == 0, format_args!("msg: 1000 messages from a child, sum {} (want 500500)", sum));

    // Request and reply: the server is a child with a port of its own; it
    // hands its Sender over in a message first.
    let reply: Port<Work> = match Port::new() {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("Port::new: {}", e)),
    };
    let hello: Port<Sender<Work>> = match Port::new() {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("Port::new: {}", e)),
    };
    let hello_tx = hello.sender();
    let server = Builder::new(b"exectest msg server\0").priority(rel(-1)).spawn(move || {
        let Ok(mine) = Port::<Work>::new() else { return 0u32 };
        let Ok(m) = Message::new(mine.sender()) else { return 0 };
        if hello_tx.send(m).is_err() {
            return 0;
        }
        let mut served = 0;
        loop {
            let Ok(mut w) = mine.recv() else { break };
            if w.a == u32::MAX {
                break; // dropped here: replied, so the client sees it come back
            }
            w.sum = w.a + w.b;
            served += 1;
            if served % 2 == 0 {
                w.reply();
            } // odd ones are replied by their drop
        }
        served
    });
    let Ok(server) = server else { return t.check(false, format_args!("spawn the server")) };
    let srv = match hello.recv() {
        Ok(m) => m.into_message().into_inner(),
        Err(e) => return t.check(false, format_args!("the server's Sender: {}", e)),
    };
    let mut good = 0;
    for i in 0..500u32 {
        let Ok(m) = Message::with_reply(Work { a: i, b: 2 * i, sum: 0 }, &reply) else { break };
        if srv.send(m).is_err() {
            break;
        }
        if let Ok(back) = reply.recv() {
            if back.is_reply() && back.sum == 3 * i {
                good += 1;
            }
        }
    }
    let stop = Message::with_reply(Work { a: u32::MAX, b: 0, sum: 0 }, &reply).ok();
    let stop_ok = stop.map(|m| srv.send(m).is_ok()).unwrap_or(false);
    let stop_back = reply.recv().map(|m| m.is_reply()).unwrap_or(false);
    let served = server.join().unwrap_or(0);
    // The server's port is gone now: a send must come back.
    let after = Message::new(Work { a: 1, b: 1, sum: 0 }).map(|m| srv.send(m).is_err()).unwrap_or(false);
    t.check(
        good == 500 && served == 500 && stop_ok && stop_back && after,
        format_args!(
            "msg: 500 requests answered by a child ({} right, {} served; replied explicitly and by drop), the stop message came back, a send after the server ended came back",
            good, served
        ),
    );

    // A port dropped with messages queued replies to them.
    let Ok(p2) = Port::<Work>::new() else { return t.check(false, format_args!("Port::new")) };
    let tx2 = p2.sender();
    let mut queued = 0;
    for i in 0..3 {
        if let Ok(m) = Message::with_reply(Work { a: i, b: 0, sum: 0 }, &reply) {
            if tx2.send(m).is_ok() {
                queued += 1;
            }
        }
    }
    drop(p2);
    let mut back = 0;
    while let Ok(Some(m)) = reply.try_recv() {
        if m.is_reply() {
            back += 1;
        }
    }
    t.check(queued == 3 && back == 3, format_args!("msg: a port dropped with 3 messages queued sent all 3 back ({})", back));
}

const TR_ADDREQUEST: u16 = 9;
const TIMEREQUEST_SIZE: u32 = 40;

fn set_delay(r: &mut Request<'_>, ms: u32) {
    r.set_command(TR_ADDREQUEST);
    let x = r.extra_mut();
    x[0..4].copy_from_slice(&(ms / 1000).to_be_bytes());
    x[4..8].copy_from_slice(&((ms % 1000) * 1000).to_be_bytes());
}

fn test_devio(t: &mut Tally) {
    let r = Device::open(b"nosuch.device\0", 0, 0, 48);
    t.check(matches!(r, Err(AmigaError::IoErr(_))), format_args!("devio: opening a missing device fails with its io_Error"));

    let mut timer = match Device::open(b"timer.device\0", 0, 0, TIMEREQUEST_SIZE) {
        Ok(d) => d,
        Err(e) => return t.check(false, format_args!("devio: open timer.device: {}", e)),
    };
    let t0 = amigaos4::arch::timebase();
    set_delay(timer.request(), 50);
    let ok = timer.request().do_io().is_ok();
    let ms = ms_since(t0);
    t.check(ok && (45..1000).contains(&ms), format_args!("devio: DoIO of a 50 ms timer request took {} ms", ms));

    set_delay(timer.request(), 5000);
    let t0 = amigaos4::arch::timebase();
    let f = timer.request().send();
    let not_yet = !f.is_done();
    let r = f.abort();
    let ms = ms_since(t0);
    t.check(
        not_yet && matches!(r, Err(AmigaError::IoErr(-2))) && ms < 1000,
        format_args!("devio: a 5 s request sent, not done, aborted: IOERR_ABORTED after {} ms", ms),
    );

    set_delay(timer.request(), 5000);
    let t0 = amigaos4::arch::timebase();
    drop(timer.request().send());
    let ms = ms_since(t0);
    t.check(ms < 1000, format_args!("devio: dropping a request in flight aborts it ({} ms)", ms));

    // Two duplicates of the device's request, in flight at once.
    let (mut a, mut b) = match (timer.new_request(), timer.new_request()) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return t.check(false, format_args!("devio: new_request")),
    };
    set_delay(&mut a, 300);
    set_delay(&mut b, 100);
    let t0 = amigaos4::arch::timebase();
    let fa = a.send();
    let fb = b.send();
    let rb = fb.wait();
    let ms_b = ms_since(t0);
    let a_pending = !fa.is_done();
    let ra = fa.wait();
    let ms_a = ms_since(t0);
    t.check(
        rb.is_ok() && ra.is_ok() && a_pending && (90..300).contains(&ms_b) && (280..1000).contains(&ms_a),
        format_args!("devio: two requests at once: 100 ms done at {} ms while the 300 ms one ran, done at {} ms", ms_b, ms_a),
    );
    drop((a, b));
    drop(timer);

    // Data through the crate's example ram.device (copied to RAM: by the
    // run script).
    let mut ram = match Device::open(b"RAM:ram.device\0", 0, 0, 48) {
        Ok(d) => d,
        Err(e) => return t.check(false, format_args!("devio: open RAM:ram.device: {}", e)),
    };
    let mut out = [0u8; 4096];
    for (i, b) in out.iter_mut().enumerate() {
        *b = (i * 7 + 3) as u8;
    }
    let w = ram.request().do_io_data(3, &mut out, 8192); // CMD_WRITE
    let mut back = [0u8; 4096];
    let rd = ram.request().do_io_data(2, &mut back, 8192); // CMD_READ
    t.check(
        w == Ok(4096) && rd == Ok(4096) && back == out,
        format_args!("devio: 4 KiB written and read back through ram.device ({:?}, {:?})", w, rd),
    );
    let mut back2 = [0u8; 4096];
    let r = match ram.request().send_data(2, &mut back2, 8192) {
        Ok(f) => f.wait_actual(),
        Err(e) => Err(e),
    };
    t.check(r == Ok(4096) && back2 == out, format_args!("devio: the same read sent and waited for ({:?})", r));
}

// ---------------------------------------------------------------------------
// Group B: pool (MemPool, ItemPool, ResourceMap), mem (copy, MemLock,
// queries), named memory, hook
// ---------------------------------------------------------------------------

fn test_mem(t: &mut Tally) {
    // MemPool: 2000 blocks of mixed sizes, patterned, checked, half freed,
    // refilled, all checked again.
    let m0 = mem::avail(0);
    {
        let pool = match MemPool::new(sys::MEMF_SHARED, 8192, 2048) {
            Ok(p) => p,
            Err(e) => return t.check(false, format_args!("MemPool::new: {}", e)),
        };
        let mut blocks = Vec::new();
        let mut zeroed = true;
        for i in 0..2000usize {
            let size = 1 + (i * 37) % 3000;
            let Some(mut b) = pool.alloc(size) else { break };
            zeroed &= b.iter().all(|&x| x == 0);
            for (j, x) in b.iter_mut().enumerate() {
                *x = (i + j) as u8;
            }
            blocks.push((i, b));
        }
        let n = blocks.len();
        let mut i = 0;
        blocks.retain(|_| {
            i += 1;
            i % 2 == 0
        });
        for i in 2000..3000usize {
            let size = 1 + (i * 37) % 3000;
            let Some(mut b) = pool.alloc(size) else { break };
            for (j, x) in b.iter_mut().enumerate() {
                *x = (i + j) as u8;
            }
            blocks.push((i, b));
        }
        let good = blocks.iter().all(|(i, b)| b.iter().enumerate().all(|(j, &x)| x == (i + j) as u8));
        t.check(n == 2000 && zeroed && good && blocks.len() == 2000, format_args!("MemPool: 2000 zeroed blocks, half freed, 1000 more, all intact ({})", blocks.len()));
    }
    let m1 = mem::avail(0);
    say!("info: free memory around the MemPool test: {} -> {} ({:+} bytes)", m0, m1, m1 as i64 - m0 as i64);

    // ItemPool shared by 8 children.
    let items: Arc<ItemPool<[u32; 8]>> = match ItemPool::new() {
        Ok(p) => Arc::new(p),
        Err(e) => return t.check(false, format_args!("ItemPool::new: {}", e)),
    };
    let mut hs = Vec::new();
    for k in 0..8u32 {
        let items = items.clone();
        if let Ok(h) = Builder::new(b"exectest items\0").priority(rel(-1)).spawn(move || {
            let mut held = Vec::new();
            let mut bad = 0u32;
            for round in 0..20u32 {
                for i in 0..100u32 {
                    match items.alloc([k, round, i, 0, 0, 0, 0, k ^ i]) {
                        Ok(it) => held.push(it),
                        Err(_) => bad += 1,
                    }
                }
                for it in &held {
                    if it[0] != k || it[7] != k ^ it[2] {
                        bad += 1;
                    }
                }
                held.clear();
            }
            bad
        }) {
            hs.push(h);
        }
    }
    let n = hs.len();
    let bad: u32 = hs.into_iter().map(|h| h.join().unwrap_or(1)).sum();
    items.gc();
    t.check(n == 8 && bad == 0, format_args!("ItemPool: 8 children x 2000 items through one protected pool, {} wrong", bad));
    let small: ItemPool<u64> = match ItemPool::with_limit(64, 16) {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("ItemPool::with_limit: {}", e)),
    };
    let mut held = Vec::new();
    while let Ok(it) = small.alloc(held.len() as u64) {
        held.push(it);
        if held.len() > 1000 {
            break;
        }
    }
    let full = held.len();
    held.pop();
    let again = small.alloc(7).is_ok();
    t.check(full == 64 && again, format_args!("ItemPool: a 64-item pool gives 64 ({}) and one more after a free", full));

    // ResourceMap.
    let map = match ResourceMap::new(0x10000, 0x10000, 16) {
        Ok(m) => m,
        Err(e) => return t.check(false, format_args!("ResourceMap::new: {}", e)),
    };
    let mut ranges = Vec::new();
    while let Some(r) = map.alloc(0x1000) {
        ranges.push(r);
        if ranges.len() > 100 {
            break;
        }
    }
    let count = ranges.len();
    let mut spans: Vec<(u32, u32)> = ranges.iter().map(|r| (r.start(), r.start() + r.size())).collect();
    spans.sort();
    let disjoint = spans.windows(2).all(|w| w[0].1 <= w[1].0);
    let inside = spans.iter().all(|&(a, b)| a >= 0x10000 && b <= 0x20000);
    ranges.truncate(count / 2);
    let al = map.alloc_aligned(0x800, 0x2000);
    let aligned = al.as_ref().map(|r| r.start() % 0x2000 == 0).unwrap_or(false);
    t.check(
        count == 16 && disjoint && inside && aligned,
        format_args!("ResourceMap: 16 ranges of 4 KiB fill 64 KiB, disjoint, inside; an aligned range after freeing half ({:?})", al.as_ref().map(|r| r.start())),
    );
    drop(al);
    drop(ranges);
    t.check(ResourceMap::new(0, 16, 0).is_err(), format_args!("ResourceMap: base 0 is refused"));

    // Copies, locks, queries.
    let src: Vec<u8> = (0..4096u32).map(|i| (i * 13) as u8).collect();
    let mut dst = alloc::vec![0u8; 4096];
    mem::copy(&src, &mut dst);
    let srcw: Vec<u32> = (0..1024u32).map(|i| i.wrapping_mul(0x9e37_79b9)).collect();
    let mut dstw = alloc::vec![0u32; 1024];
    mem::copy_quick(&srcw, &mut dstw);
    t.check(dst == src && dstw == srcw, format_args!("mem: CopyMem and CopyMemQuick copy 4 KiB exactly"));
    test_glue_mem(t);
    let locked = mem::MemLock::new(&src).is_some();
    t.check(locked, format_args!("mem: MemLock on a heap buffer"));
    t.check(mem::type_of(&*src) != 0 && mem::avail(0) > 0, format_args!("mem: TypeOfMem 0x{:x} for a heap buffer", mem::type_of(&*src)));
}

/// glue/freestanding_mem.c's memcpy, memmove and memset on the target (big
/// endian: the shifted word copy for a source at another alignment), every
/// length 0..200 at source and destination alignments 0..7, against
/// byte-at-a-time references read and written volatile (so LLVM does not
/// make them memcpy calls themselves).
fn test_glue_mem(t: &mut Tally) {
    const N: usize = 240;
    let src: Vec<u8> = (0..N).map(|i| (i * 7 + 3) as u8).collect();
    let mut a = alloc::vec![0u8; N];
    let mut b = alloc::vec![0u8; N];
    let (mut checks, mut wrong) = (0u32, 0u32);
    let fill = |v: &mut [u8], x: u8| v.iter_mut().for_each(|p| vwrite(p, x));
    for so in 0..8 {
        for d in 0..8 {
            for n in 0..=200usize {
                let n = core::hint::black_box(n);
                fill(&mut a, 0xEE);
                fill(&mut b, 0xEE);
                // SAFETY: so + n and d + n are within the N-byte buffers,
                // which do not overlap.
                unsafe { core::ptr::copy_nonoverlapping(src.as_ptr().add(so), a.as_mut_ptr().add(d), n) };
                for k in 0..n {
                    vwrite(&mut b[d + k], src[so + k]);
                }
                checks += 1;
                wrong += (a != b) as u32;
            }
        }
    }
    for shift in [-9isize, -5, -4, -3, -1, 1, 2, 3, 4, 7, 9] {
        for n in [0usize, 1, 3, 15, 16, 17, 64, 101, 200] {
            let n = core::hint::black_box(n);
            let at = 16usize;
            let to = (at as isize + shift) as usize;
            a.copy_from_slice(&src);
            b.copy_from_slice(&src);
            // SAFETY: at + n and to + n are within the buffer; copy allows
            // the overlap.
            unsafe { core::ptr::copy(a.as_ptr().add(at), a.as_mut_ptr().add(to), n) };
            let tmp: Vec<u8> = (0..n).map(|k| src[at + k]).collect();
            for k in 0..n {
                vwrite(&mut b[to + k], tmp[k]);
            }
            checks += 1;
            wrong += (a != b) as u32;
        }
    }
    for d in 0..8 {
        for n in 0..=200usize {
            let n = core::hint::black_box(n);
            fill(&mut a, 0xEE);
            fill(&mut b, 0xEE);
            // SAFETY: d + n is within the buffer.
            unsafe { core::ptr::write_bytes(a.as_mut_ptr().add(d), 0x5A, n) };
            for k in 0..n {
                vwrite(&mut b[d + k], 0x5A);
            }
            checks += 1;
            wrong += (a != b) as u32;
        }
    }
    t.check(wrong == 0 && checks > 0, format_args!("mem: the glue's memcpy/memmove/memset match a byte loop at every alignment: {} checks, {} wrong", checks, wrong));
}

/// One byte written volatile.
fn vwrite(p: &mut u8, x: u8) {
    // SAFETY: a valid, exclusive reference.
    unsafe { core::ptr::write_volatile(p, x) };
}

fn test_named(t: &mut Tally) {
    const SP: Option<&str> = Some("exectest");
    let before = named::exists(SP, "block");
    let cleaned = named::lock(SP, "block").map(|l| l.free());
    let made = named::alloc(SP, "block", 256);
    let dup = named::alloc(SP, "block", 256);
    say!(
        "info: named: existed before {}, cleanup {:?}, alloc {:?}, again {:?}, exists {}, lock {}",
        before, cleaned, made, dup, named::exists(SP, "block"), named::lock(SP, "block").map(|l| l.len()).unwrap_or(0)
    );
    t.check(made.is_ok() && matches!(dup, Err(AmigaError::Exists)), format_args!("named: a block made, the same name again is Exists"));
    let mut size = 0;
    if let Some(mut l) = named::lock(SP, "block") {
        size = l.len();
        for (i, x) in l.iter_mut().enumerate() {
            *x = i as u8;
        }
    }
    t.check(size == 256 && named::exists(SP, "block"), format_args!("named: locked, {} bytes, written", size));

    // A child's lock waits while the main task holds the block (300 ms on
    // timer.device), and then sees what the main task wrote.
    let held = named::lock(SP, "block");
    let t0 = amigaos4::arch::timebase();
    let h = Builder::new(b"exectest named\0").priority(rel(-1)).spawn(move || {
        let l = named::lock(Some("exectest"), "block");
        let ms = ms_since(t0);
        let ok = l.as_ref().map(|l| l.iter().enumerate().all(|(i, &x)| x == (i as u8) ^ 0x5a)).unwrap_or(false);
        (ok, ms)
    });
    if let Some(mut l) = held {
        for (i, x) in l.iter_mut().enumerate() {
            *x = (i as u8) ^ 0x5a;
        }
        if let Ok(mut timer) = Device::open(b"timer.device\0", 0, 0, TIMEREQUEST_SIZE) {
            set_delay(timer.request(), 300);
            let _ = timer.request().do_io();
        }
    }
    let (ok, ms) = h.map(|h| h.join().unwrap_or((false, 0))).unwrap_or((false, 0));
    t.check(ok && ms >= 280, format_args!("named: a child's lock waited {} ms for the main task's and saw its writes", ms));

    let freed = named::lock(SP, "block").map(|l| l.free()).unwrap_or(false);
    t.check(freed && !named::exists(SP, "block") && named::lock(SP, "block").is_none(), format_args!("named: freed through its lock, gone after"));
}

fn test_hook(t: &mut Tally) {
    use core::sync::atomic::AtomicU32;
    let calls = Arc::new(AtomicU32::new(0));
    let c = calls.clone();
    let hook = match Hook::new(move |obj: sys::APTR, msg: sys::APTR| -> u32 {
        c.fetch_add(1, Ordering::Relaxed);
        (obj as u32).wrapping_add(msg as u32)
    }) {
        Ok(h) => h,
        Err(e) => return t.check(false, format_args!("Hook::new: {}", e)),
    };
    // Called the way an OS call does: through h_Entry.
    // SAFETY: a live hook and plain values as object and message.
    let r = unsafe { sys::utility_call_hook_pkt(hook.as_ptr(), 40 as sys::APTR, 2 as sys::APTR) };
    t.check(r == 42 && calls.load(Ordering::Relaxed) == 1, format_args!("hook: CallHookPkt reaches the closure ({})", r));
}

/// Free memory around four MemPool cycles: a leak grows each cycle, a
/// cache does not.
fn pool_cycles() {
    for c in 0..4 {
        let m0 = mem::avail(0);
        if let Ok(pool) = MemPool::new(sys::MEMF_SHARED, 8192, 2048) {
            let mut v = Vec::new();
            for i in 0..2000usize {
                if let Some(b) = pool.alloc(1 + (i * 37) % 3000) {
                    v.push(b);
                }
            }
        }
        let m1 = mem::avail(0);
        say!("info: MemPool cycle {}: free memory {} -> {} ({:+})", c, m0, m1, m1 as i64 - m0 as i64);
    }
}

// ---------------------------------------------------------------------------
// Group C: list, avl
// ---------------------------------------------------------------------------

fn test_list(t: &mut Tally) {
    let mut l = match NameList::new(0) {
        Ok(l) => l,
        Err(e) => return t.check(false, format_args!("NameList::new: {}", e)),
    };
    let ok = l.push_back("b", 0).is_ok() && l.push_back("c", 0).is_ok() && l.push_front("a", 0).is_ok();
    let order: Vec<&str> = l.iter().map(|n| n.name()).collect();
    t.check(ok && order == ["a", "b", "c"] && l.len() == 3, format_args!("list: push_front/push_back keep order {:?}", order));

    let mut q = match NameList::new(0) {
        Ok(l) => l,
        Err(e) => return t.check(false, format_args!("NameList::new: {}", e)),
    };
    for (n, p) in [("low", -5i8), ("high", 10), ("mid", 0), ("high2", 10), ("mid2", 0)] {
        let _ = q.enqueue(n, p);
    }
    let order: Vec<&str> = q.iter().map(|n| n.name()).collect();
    t.check(order == ["high", "high2", "mid", "mid2", "low"], format_args!("list: Enqueue sorts by priority, FIFO within one {:?}", order));

    let found = l.find("b").map(|n| n.name()) == Some("b") && l.find("B").is_none() && l.find_ignore_case("B").map(|n| n.name()) == Some("b");
    let removed = l.remove("b").map(|i| i.name) == Some(alloc::string::String::from("b")) && l.find("b").is_none() && l.len() == 2;
    t.check(found && removed, format_args!("list: FindName, FindIName, remove by name"));

    l.append(&mut q);
    let order: Vec<&str> = l.iter().map(|n| n.name()).collect();
    t.check(
        q.is_empty() && q.iter().next().is_none() && order == ["a", "c", "high", "high2", "mid", "mid2", "low"] && l.len() == 7,
        format_args!("list: MoveList appends and empties the other {:?}", order),
    );
    let first = l.pop_front().map(|i| i.name);
    let last = l.pop_back().map(|i| (i.name, i.pri));
    t.check(
        first.as_deref() == Some("a") && last == Some((alloc::string::String::from("low"), -5)) && l.len() == 5,
        format_args!("list: pop_front / pop_back with name and priority"),
    );

    // Memory: 20 rounds of 500 nodes made and dropped.
    let m0 = mem::avail(0);
    for _ in 0..20 {
        if let Ok(mut big) = NameList::new(0) {
            for i in 0..500 {
                let _ = big.enqueue(if i % 2 == 0 { "even" } else { "odd" }, (i % 7) as i8);
            }
        }
    }
    say!("info: list: free memory around 10000 nodes: {:+} bytes", mem::avail(0) as i64 - m0 as i64);
}

fn test_avl(t: &mut Tally) {
    let mut m: AvlMap<u64> = AvlMap::new();
    // 2000 keys in a scrambled order.
    for i in 0..2000u32 {
        let k = i.wrapping_mul(2_654_435_761) % 1_000_003;
        m.insert(k, k as u64 * 3);
    }
    let n = m.len();
    let keys: Vec<u32> = m.iter().map(|(k, _)| k).collect();
    let sorted = keys.windows(2).all(|w| w[0] < w[1]);
    let values = m.iter().all(|(k, v)| *v == k as u64 * 3);
    t.check(n == 2000 && keys.len() == 2000 && sorted && values, format_args!("avl: 2000 keys inserted, iterated in order with their values"));

    let k0 = keys[100];
    let replaced = m.insert(k0, 7);
    let got = m.get(k0).copied();
    if let Some(v) = m.get_mut(k0) {
        *v += 1;
    }
    t.check(replaced == Some(k0 as u64 * 3) && got == Some(7) && m.get(k0) == Some(&8) && m.len() == 2000, format_args!("avl: insert replaces, get, get_mut"));

    let mut removed = 0;
    for (i, k) in keys.iter().enumerate() {
        if i % 2 == 0 && m.remove(*k).is_some() {
            removed += 1;
        }
    }
    let rest = m.iter().count();
    let gone = keys.iter().enumerate().all(|(i, k)| m.contains_key(*k) == (i % 2 == 1));
    t.check(removed == 1000 && rest == 1000 && m.len() == 1000 && gone, format_args!("avl: half removed, the other half still there"));

    let mid = keys[1001];
    let from: Vec<u32> = m.range_from(mid).map(|(k, _)| k).take(3).collect();
    let between = keys[1000] + 1; // removed key + 1 (no entry unless it is keys[1001])
    let from2 = m.range_from(between).next().map(|(k, _)| k);
    t.check(
        from.first() == Some(&mid) && from.windows(2).all(|w| w[0] < w[1]) && from2 == Some(mid),
        format_args!("avl: range_from finds the key or the next higher ({:?})", from2),
    );
}

// ---------------------------------------------------------------------------
// Group D: library, SoftInt
// ---------------------------------------------------------------------------

fn test_library(t: &mut Tally) {
    let lib = match Library::open("utility.library", 50) {
        Ok(l) => l,
        Err(e) => return t.check(false, format_args!("library: open utility.library: {}", e)),
    };
    let ver = lib.version();
    let id = alloc::string::String::from(lib.id_string().unwrap_or(""));
    let iface = lib.interface::<sys::UtilityIFace>("main", 1);
    let ok = iface.is_ok();
    drop(iface);
    t.check(ver >= 50 && id.contains("utility") && ok, format_args!("library: utility.library {}.{} \"{}\", its main interface", ver, lib.revision(), id.trim_end()));
    t.check(Library::open("nosuch.library", 0).is_err(), format_args!("library: a missing library is an error"));
    t.check(lib.interface::<sys::UtilityIFace>("nosuch", 1).is_err(), format_args!("library: a missing interface is an error"));
    t.check(amigaos4::library::open_resource("nosuch.resource").is_none(), format_args!("library: a missing resource is None"));
}

struct SoftData {
    count: core::sync::atomic::AtomicU32,
}

unsafe extern "C" fn soft_handler(_unused: i32, _sysbase: *mut core::ffi::c_void, data: *mut core::ffi::c_void) {
    // SAFETY: data is the SoftData the test keeps alive across the SoftInt.
    let d = unsafe { &*(data as *const SoftData) };
    d.count.fetch_add(1, Ordering::Relaxed);
}

fn test_softint(t: &mut Tally) {
    let data = SoftData { count: core::sync::atomic::AtomicU32::new(0) };
    // SAFETY: the handler only bumps an atomic in `data`, which outlives
    // the SoftInt (dropped first, below).
    let si = unsafe { SoftInt::new(0, b"exectest softint\0", soft_handler, &data as *const SoftData as *mut core::ffi::c_void) };
    let si = match si {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("SoftInt::new: {}", e)),
    };
    let mut immediate = 0;
    for i in 1..=100u32 {
        si.cause();
        if data.count.load(Ordering::Relaxed) == i {
            immediate += 1;
        }
    }
    drop(si);
    t.check(
        data.count.load(Ordering::Relaxed) == 100 && immediate == 100,
        format_args!("SoftInt: 100 Causes ran the handler 100 times, each before Cause returned ({})", immediate),
    );
    // SAFETY: as above; an invalid priority is refused before anything is made.
    let bad = unsafe { SoftInt::new(5, b"x\0", soft_handler, core::ptr::null_mut()) };
    t.check(bad.is_err(), format_args!("SoftInt: priority 5 is refused (exec has -32/-16/0/16/32)"));
}

// ---------------------------------------------------------------------------
// Group E: sysinfo, stack, debug
// ---------------------------------------------------------------------------

fn test_sysinfo(t: &mut Tally) {
    let c = sysinfo::cpu();
    say!(
        "info: CPU {} ({}) {} MHz, bus {} MHz, timebase {} Hz, L1 {} L2 {} line {}, pages {}/{}, vector {}, ext \"{}\", core {} of {}",
        c.model_name, c.version_name, c.cpu_speed / 1_000_000, c.bus_speed / 1_000_000, c.timebase_speed,
        c.l1_cache, c.l2_cache, c.cache_line, c.cpu_page_size, c.exec_page_size, c.vector_unit, c.extensions, c.current_cpu, c.count
    );
    t.check(
        c.count >= 1 && c.timebase_speed > 0 && c.cache_line.is_power_of_two() && c.cache_line >= 16 && !c.model_name.is_empty() && c.cpu_page_size >= 4096,
        format_args!("sysinfo: GetCPUInfo gives a CPU name, a timebase, a cache line {} and pages", c.cache_line),
    );
}

#[inline(never)]
fn deep(n: u32) -> u32 {
    let buf = core::hint::black_box([n as u8; 1024]);
    if n == 0 { buf[0] as u32 } else { deep(n - 1).wrapping_add(buf[1023] as u32) }
}

fn test_stack(t: &mut Tally) {
    let outer = 0u8;
    let outer_addr = &outer as *const u8 as usize;
    // 600 frames of over 1 KiB: well past a Shell's default stack.
    let r = stack::run_with_stack(2 * 1024 * 1024, || {
        let inner = 0u8;
        let inner_addr = &inner as *const u8 as usize;
        (deep(600), inner_addr)
    });
    let ok = matches!(r, Ok((v, inner)) if v == (1..=600u32).fold(0u32, |a, n| a.wrapping_add(n as u8 as u32)) && inner.abs_diff(outer_addr) > 64 * 1024);
    t.check(ok, format_args!("stack: 600 frames of 1 KiB on a 2 MiB stack of its own, away from the task's stack ({:?})", r.map(|(v, a)| (v, a))));
}

fn test_debug(t: &mut Tally) {
    let d = match amigaos4::debug::Debug::open() {
        Ok(d) => d,
        Err(e) => return t.check(false, format_args!("debug: open: {}", e)),
    };
    let here = test_debug as fn(&mut Tally) as usize;
    let s = d.symbol(here);
    t.check(
        s.as_ref().map(|s| s.module.contains("exectest") && s.segment_offset > 0).unwrap_or(false),
        format_args!("debug: this function's symbol: {:?}", s.as_ref().map(|s| (s.module.as_str(), s.segment, s.segment_offset))),
    );
    let frames = d.backtrace(64);
    let mods: Vec<alloc::string::String> = frames
        .iter()
        .map(|&a| d.symbol(a).map(|s| alloc::format!("{:08x} {}+{:x}", a, s.module, s.segment_offset.max(s.offset))).unwrap_or(alloc::format!("{:08x} ?", a)))
        .collect();
    for m in mods.iter().take(4) {
        say!("info: frame {}", m);
    }
    let ours = frames.iter().filter(|&&a| d.symbol(a).map(|s| s.module.contains("exectest")).unwrap_or(false)).count();
    t.check(frames.len() >= 2 && ours >= 1, format_args!("debug: a backtrace of the calling task: {} frames, {} in exectest", frames.len(), ours));
}

#[no_mangle]
pub extern "C" fn rust_main(args: *const u8, arglen: i32) -> i32 {
    // SAFETY: the Shell's argument string, `arglen` bytes.
    let text = unsafe { core::slice::from_raw_parts(args, arglen.max(0) as usize) };
    let text = core::str::from_utf8(text).unwrap_or("");
    let rounds: u32 = text.split_whitespace().next().and_then(|w| w.parse().ok()).unwrap_or(1000);

    let mut t = Tally { pass: 0, total: 0 };
    let bits0 = free_signal_bits();
    say!(
        "exectest: {} rounds per priority, main task at priority {}, {} free signal bits, {} CPU(s)",
        rounds,
        task::priority(),
        bits0,
        task::cpu_count()
    );

    test_spawn(&mut t, rounds);
    probe_exec_attempt_with_signal();
    test_mutex(&mut t);
    test_semaphore(&mut t);
    test_msg(&mut t);
    test_devio(&mut t);
    test_mem(&mut t);
    pool_cycles();
    test_named(&mut t);
    test_hook(&mut t);
    test_list(&mut t);
    test_avl(&mut t);
    test_library(&mut t);
    test_softint(&mut t);
    test_sysinfo(&mut t);
    test_stack(&mut t);
    test_debug(&mut t);
    test_bids(&mut t);
    test_mem_handler(&mut t);
    test_debug_watch(&mut t);
    test_task_panic(&mut t);
    test_signal_misc(&mut t);

    // Memory around one more set of rounds, after the warm-up above.
    let m0 = avail();
    let ok = (spawn_rounds(rounds, 0) == Ok(rounds)) as u32;
    let m1 = avail();
    say!("info: free memory around {} more spawn rounds: {} -> {} ({:+} bytes)", rounds, m0, m1, m1 as i64 - m0 as i64);
    t.check(ok == 1, format_args!("the extra {} rounds", rounds));

    let bits1 = free_signal_bits();
    t.check(bits1 == bits0, format_args!("free signal bits at the end: {} (start {})", bits1, bits0));

    say!("exectest: {}/{} passed", t.pass, t.total);
    if t.pass == t.total { 0 } else { 20 }
}
