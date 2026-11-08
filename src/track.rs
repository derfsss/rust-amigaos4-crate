//! Resource tracking, and what a panic does with it.
//!
//! AmigaOS does not take back what a task held when it ends: files stay
//! open, locks held, windows on the screen, signal bits and ports gone. In
//! Rust each wrapper frees its resource when it is dropped, but programs
//! build with `panic = "abort"`, so a panic runs no destructor. This module
//! closes that gap:
//!
//! - **Registry.** The crate's wrappers for long-lived OS resources (files,
//!   locks, signals, message ports, windows) register each one with the
//!   task that made it and the source line that asked for it
//!   (`#[track_caller]`), and remove it when dropped. The registry is a
//!   fixed table in MEMF_SHARED memory behind an exec mutex: registering
//!   never allocates.
//! - **Catch points.** [`main`] (a program's body) and the crate's task and
//!   process spawning run their closures through [`catch`]: on a panic the
//!   handler jumps back there and the closure's call returns
//!   `Err(Panicked)`. The panicking frames are abandoned, as with abort:
//!   their destructors do not run, which is why the registry exists.
//! - **The panic path** ([`on_panic`], called by
//!   [`panic::default_panic_handler`](crate::panic::default_panic_handler)):
//!   print the message and a backtrace (walked from the panic's own stack
//!   frame) to the serial debug output and, in a DOS Process, to
//!   `T:<program>.crash`; release everything the panicking task still has
//!   registered, newest first, by raw release calls (never Rust `Drop`: the
//!   panicking code's state may be half-changed); then return to the catch
//!   point. A task without one (a device's unit task, an interrupt) waits
//!   forever instead of spinning. A program whose child tasks still run
//!   does not return to its Shell either (they run in its code): it waits.
//! - **Leaks.** When [`main`]'s closure returns, whatever the main task
//!   still has registered was leaked (`mem::forget`, a static): each is
//!   reported to the serial output with its source line, and released.
//!
//! The catch point is a register save (PowerPC: r1, r2, r13-r31, f14-f31,
//! CR, LR) and the throw restores it: about forty instructions in
//! `global_asm!`, no C runtime. AltiVec registers are not saved (the target
//! does not use them).

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::fmt::Write;
use core::panic::{Location, PanicInfo};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, Ordering};

#[cfg(target_os = "amigaos")]
use crate::fmt::Serial;

/// The serial debug output on the target; nothing on a host (where the
/// crate's tests link without the target's C glue).
#[cfg(not(target_os = "amigaos"))]
struct Serial;

#[cfg(not(target_os = "amigaos"))]
impl Write for Serial {
    fn write_str(&mut self, _: &str) -> core::fmt::Result {
        Ok(())
    }
}
use crate::signal::current_task;
use crate::sys::{self, Task, APTR};

// ---------------------------------------------------------------------------
// Catch and throw
// ---------------------------------------------------------------------------

/// Saved registers: 0 LR, 4 r1, 8 r2, 12 CR, 16..92 r13-r31, 96..240
/// f14-f31.
#[repr(C, align(8))]
struct JumpBuf([u32; 64]);

#[cfg(target_os = "amigaos")]
extern "C" {
    /// Save the registers into `buf`, call `f(arg)`, return its result; or,
    /// if `amigaos4_throw(buf, code)` is called meanwhile, return `code`.
    fn amigaos4_catch(f: extern "C" fn(*mut c_void) -> i32, arg: *mut c_void, buf: *mut JumpBuf) -> i32;
    /// Restore `buf` and return `code` from the `amigaos4_catch` that
    /// filled it. `buf`'s catch must still be running.
    fn amigaos4_throw(buf: *mut JumpBuf, code: i32) -> !;
}

#[cfg(target_os = "amigaos")]
core::arch::global_asm!(
    ".section .text.amigaos4_catch,\"ax\",@progbits",
    ".globl amigaos4_catch",
    ".type amigaos4_catch,@function",
    ".p2align 2",
    "amigaos4_catch:",
    "mflr 0",
    "stw 0, 0(5)",
    "stw 1, 4(5)",
    "stw 2, 8(5)",
    "stw 0, 4(1)",
    "mfcr 0",
    "stw 0, 12(5)",
    "stmw 13, 16(5)",
    "stfd 14, 96(5)",
    "stfd 15, 104(5)",
    "stfd 16, 112(5)",
    "stfd 17, 120(5)",
    "stfd 18, 128(5)",
    "stfd 19, 136(5)",
    "stfd 20, 144(5)",
    "stfd 21, 152(5)",
    "stfd 22, 160(5)",
    "stfd 23, 168(5)",
    "stfd 24, 176(5)",
    "stfd 25, 184(5)",
    "stfd 26, 192(5)",
    "stfd 27, 200(5)",
    "stfd 28, 208(5)",
    "stfd 29, 216(5)",
    "stfd 30, 224(5)",
    "stfd 31, 232(5)",
    "stwu 1, -32(1)",
    "stw 5, 8(1)",
    "mtctr 3",
    "mr 3, 4",
    "bctrl",
    "lwz 5, 8(1)",
    "addi 1, 1, 32",
    "lwz 0, 0(5)",
    "mtlr 0",
    "blr",
    ".size amigaos4_catch, . - amigaos4_catch",
    ".globl amigaos4_throw",
    ".type amigaos4_throw,@function",
    ".p2align 2",
    "amigaos4_throw:",
    "lwz 0, 0(3)",
    "mtlr 0",
    "lwz 1, 4(3)",
    "lwz 2, 8(3)",
    "lwz 0, 12(3)",
    "mtcrf 0xff, 0",
    "lmw 13, 16(3)",
    "lfd 14, 96(3)",
    "lfd 15, 104(3)",
    "lfd 16, 112(3)",
    "lfd 17, 120(3)",
    "lfd 18, 128(3)",
    "lfd 19, 136(3)",
    "lfd 20, 144(3)",
    "lfd 21, 152(3)",
    "lfd 22, 160(3)",
    "lfd 23, 168(3)",
    "lfd 24, 176(3)",
    "lfd 25, 184(3)",
    "lfd 26, 192(3)",
    "lfd 27, 200(3)",
    "lfd 28, 208(3)",
    "lfd 29, 216(3)",
    "lfd 30, 224(3)",
    "lfd 31, 232(3)",
    "mr 3, 4",
    "blr",
    ".size amigaos4_throw, . - amigaos4_throw",
    ".previous",
);

/// The closure of a [`catch`] did not return: it panicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panicked;

/// One task's innermost catch point.
struct Slot {
    task: AtomicPtr<Task>,
    buf: AtomicPtr<JumpBuf>,
    panicking: AtomicBool,
}

const SLOTS: usize = 128;
#[allow(clippy::declare_interior_mutable_const)]
const EMPTY_SLOT: Slot = Slot { task: AtomicPtr::new(core::ptr::null_mut()), buf: AtomicPtr::new(core::ptr::null_mut()), panicking: AtomicBool::new(false) };
static CATCH: [Slot; SLOTS] = [EMPTY_SLOT; SLOTS];
/// The task that runs [`main`], and how many child tasks or processes the
/// crate has started that are still in the program's code.
static MAIN_TASK: AtomicPtr<Task> = AtomicPtr::new(core::ptr::null_mut());
static LIVE_CHILDREN: AtomicI32 = AtomicI32::new(0);
/// A task with no catch slot that is panicking (one at a time).
static LONE_PANIC: AtomicPtr<Task> = AtomicPtr::new(core::ptr::null_mut());

fn slot_of(task: *mut Task) -> Option<&'static Slot> {
    CATCH.iter().find(|s| s.task.load(Ordering::Acquire) == task)
}

/// Make `buf` the task's catch point; the previous one (if any) is
/// returned for `pop`.
fn push(task: *mut Task, buf: *mut JumpBuf) -> Option<*mut JumpBuf> {
    if let Some(s) = slot_of(task) {
        return Some(s.buf.swap(buf, Ordering::AcqRel));
    }
    for s in CATCH.iter() {
        if s.task.compare_exchange(core::ptr::null_mut(), task, Ordering::AcqRel, Ordering::Acquire).is_ok() {
            s.panicking.store(false, Ordering::Release);
            s.buf.store(buf, Ordering::Release);
            return Some(core::ptr::null_mut());
        }
    }
    // Table full: this closure runs without a catch point.
    None
}

fn pop(task: *mut Task, prev: Option<*mut JumpBuf>) {
    let Some(prev) = prev else { return };
    if let Some(s) = slot_of(task) {
        s.buf.store(prev, Ordering::Release);
        if prev.is_null() {
            s.task.store(core::ptr::null_mut(), Ordering::Release);
        }
    }
}

struct CatchState<F, R> {
    f: Option<F>,
    r: Option<R>,
}

extern "C" fn catch_body<F: FnOnce() -> R, R>(arg: *mut c_void) -> i32 {
    // SAFETY: `arg` is the CatchState that catch() keeps on its stack for
    // the whole call.
    let st = unsafe { &mut *(arg as *mut CatchState<F, R>) };
    if let Some(f) = st.f.take() {
        st.r = Some(f());
    }
    0
}

/// Run `f`; `Err(Panicked)` if it panicked (with the panic handler of this
/// module: [`on_panic`]). What `f`'s frames held is abandoned, not dropped;
/// what the task had registered is released by the panic path.
pub fn catch<F: FnOnce() -> R, R>(f: F) -> Result<R, Panicked> {
    let mut st = CatchState { f: Some(f), r: None };
    let me = current_task();
    #[cfg(target_os = "amigaos")]
    {
        let mut buf = JumpBuf([0; 64]);
        let prev = push(me, &mut buf);
        let arg = &mut st as *mut CatchState<F, R> as *mut c_void;
        // SAFETY: catch_body::<F, R> takes our CatchState, alive across the
        // call; `buf` stays in this frame until pop. A throw returns 20.
        let r = unsafe { amigaos4_catch(catch_body::<F, R>, arg, &mut buf) };
        pop(me, prev);
        if r != 0 {
            // The closure was taken and abandoned; nothing to drop but the
            // empty state.
            return Err(Panicked);
        }
    }
    #[cfg(not(target_os = "amigaos"))]
    {
        let _ = (me, push, pop);
        catch_body::<F, R>(&mut st as *mut CatchState<F, R> as *mut c_void);
    }
    st.r.take().ok_or(Panicked)
}

/// A program's body: `f` run through [`catch`]. Returns `f`'s result, or
/// RETURN_FAIL (20) after a panic. Afterwards, whatever the task still has
/// registered is reported as leaked and released.
pub fn main<F: FnOnce() -> i32>(f: F) -> i32 {
    MAIN_TASK.store(current_task(), Ordering::Release);
    let r = catch(f).unwrap_or(20);
    let n = release_task(current_task(), true);
    if n > 0 {
        let _ = writeln!(Serial, "track: {} leaked resource(s) released at the end of the program", n);
    }
    r
}

/// Release what the calling task still has registered, reporting each as
/// leaked: the end of a detached process (`dos::process::spawn_detached`),
/// whose leftovers have no one to pass to.
#[cfg(feature = "dos")]
pub(crate) fn release_leftovers() {
    let n = release_task(current_task(), true);
    if n > 0 {
        let _ = writeln!(Serial, "track: {} leaked resource(s) released at the end of a detached process", n);
    }
}

/// A child task or process the crate started begins its work.
pub(crate) fn child_started() {
    LIVE_CHILDREN.fetch_add(1, Ordering::AcqRel);
}

/// A child task or process ends its work (normally or after a panic). What
/// it still has registered passes to `parent` (a resource it returned, or
/// leaked), except what belongs to the child task itself (signal bits,
/// ports), which ends with it and is only dropped from the registry.
pub(crate) fn child_finished(parent: *mut Task) {
    let me = current_task();
    if let Some(r) = registry() {
        r.with(|t| {
            for e in t.iter_mut().filter(|e| e.id != 0 && e.task == me) {
                if e.bound {
                    *e = EMPTY;
                } else {
                    e.task = parent;
                }
            }
        });
    }
    LIVE_CHILDREN.fetch_sub(1, Ordering::AcqRel);
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// How a resource is let go of: called with the two words given at
/// registration, in the task that registered it.
pub type ReleaseFn = unsafe fn(usize, usize);

#[derive(Clone, Copy)]
struct Entry {
    id: u32,
    a: usize,
    b: usize,
    kind: &'static str,
    release: Option<ReleaseFn>,
    task: *mut Task,
    /// Belongs to its task (a signal bit, a port): never released by
    /// another task.
    bound: bool,
    at: Option<&'static Location<'static>>,
}

const EMPTY: Entry = Entry { id: 0, a: 0, b: 0, kind: "", release: None, task: core::ptr::null_mut(), bound: false, at: None };
const CAPACITY: usize = 1024;

struct Registry {
    mx: APTR,
    entries: UnsafeCell<[Entry; CAPACITY]>,
}

static REGISTRY: AtomicPtr<Registry> = AtomicPtr::new(core::ptr::null_mut());
static NEXT_ID: AtomicU32 = AtomicU32::new(1);
/// Resources not tracked because the table was full or could not be made.
static UNTRACKED: AtomicU32 = AtomicU32::new(0);

fn registry() -> Option<&'static Registry> {
    let r = REGISTRY.load(Ordering::Acquire);
    if !r.is_null() {
        // SAFETY: published once, never freed.
        return Some(unsafe { &*r });
    }
    let size = core::mem::size_of::<Registry>() as u32;
    // SAFETY: a plain allocation, cleared (every entry empty: id 0), in
    // memory every task may use.
    let p = unsafe { sys::exec_alloc_vec(size, sys::MEMF_SHARED | sys::MEMF_CLEAR) } as *mut Registry;
    if p.is_null() {
        return None;
    }
    let tags = [
        sys::TagItem { ti_Tag: sys::ASOMUTEX_RECURSIVE, ti_Data: 0 },
        sys::TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: a valid tag list.
    let mx = unsafe { sys::exec_alloc_sys_object(sys::ASOT_MUTEX, tags.as_ptr()) };
    if mx.is_null() {
        // SAFETY: ours, never published.
        unsafe { sys::exec_free_vec(p as APTR) };
        return None;
    }
    // SAFETY: our fresh block; the cleared entries are valid EMPTY ones.
    unsafe { core::ptr::addr_of_mut!((*p).mx).write(mx) };
    match REGISTRY.compare_exchange(core::ptr::null_mut(), p, Ordering::AcqRel, Ordering::Acquire) {
        // SAFETY: published, never freed.
        Ok(_) => Some(unsafe { &*p }),
        Err(other) => {
            // SAFETY: ours, never published; another task's won.
            unsafe {
                sys::exec_free_sys_object(sys::ASOT_MUTEX, mx);
                sys::exec_free_vec(p as APTR);
            }
            // SAFETY: published, never freed.
            Some(unsafe { &*other })
        }
    }
}

impl Registry {
    fn with<R>(&self, f: impl FnOnce(&mut [Entry; CAPACITY]) -> R) -> R {
        // SAFETY: our mutex; the table is only touched while it is held.
        unsafe {
            sys::exec_mutex_obtain(self.mx);
            let r = f(&mut *self.entries.get());
            sys::exec_mutex_release(self.mx);
            r
        }
    }

    /// As `with`, or None if another holder has it now.
    fn try_with<R>(&self, f: impl FnOnce(&mut [Entry; CAPACITY]) -> R) -> Option<R> {
        // SAFETY: as in with(); MutexAttempt returns a BOOL.
        unsafe {
            if sys::exec_mutex_attempt(self.mx) == 0 {
                return None;
            }
            let r = f(&mut *self.entries.get());
            sys::exec_mutex_release(self.mx);
            Some(r)
        }
    }
}

/// A registered resource; dropping it removes the registration (the owner
/// then releases the resource itself).
#[must_use]
pub struct Tracked {
    id: u32,
}

impl Tracked {
    /// A registration that tracks nothing.
    pub const fn none() -> Self {
        Self { id: 0 }
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        if self.id == 0 {
            return;
        }
        if let Some(r) = registry() {
            let id = self.id;
            r.with(|t| {
                if let Some(e) = t.iter_mut().find(|e| e.id == id) {
                    *e = EMPTY;
                }
            });
        }
    }
}

/// Register a resource of the calling task: `release(a, b)` lets go of it.
/// The source line is the caller's (or its caller's, through
/// `#[track_caller]` constructors).
#[track_caller]
pub fn register(kind: &'static str, a: usize, b: usize, release: ReleaseFn) -> Tracked {
    register_inner(kind, a, b, release, false, Location::caller())
}

/// As [`register`], for a resource that belongs to the calling task itself
/// (a signal bit, a message port): when the task ends, it is forgotten,
/// never released by another task.
#[track_caller]
pub fn register_bound(kind: &'static str, a: usize, b: usize, release: ReleaseFn) -> Tracked {
    register_inner(kind, a, b, release, true, Location::caller())
}

fn register_inner(kind: &'static str, a: usize, b: usize, release: ReleaseFn, bound: bool, at: &'static Location<'static>) -> Tracked {
    let Some(r) = registry() else {
        UNTRACKED.fetch_add(1, Ordering::Relaxed);
        return Tracked::none();
    };
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let task = current_task();
    let ok = r.with(|t| match t.iter_mut().find(|e| e.id == 0) {
        Some(e) => {
            *e = Entry { id, a, b, kind, release: Some(release), task, bound, at: Some(at) };
            true
        }
        None => false,
    });
    if ok {
        Tracked { id }
    } else {
        UNTRACKED.fetch_add(1, Ordering::Relaxed);
        Tracked::none()
    }
}

/// How many resources `task` has registered now.
pub fn count(task: *mut Task) -> usize {
    registry().map(|r| r.with(|t| t.iter().filter(|e| e.id != 0 && e.task == task).count())).unwrap_or(0)
}

/// Resources that could not be registered (table full) since the start.
pub fn untracked() -> u32 {
    UNTRACKED.load(Ordering::Relaxed)
}

/// Release what `task` has registered, newest first, each entry removed
/// before its release runs; `report` prints each one. Returns how many.
/// Waits for the registry unless called on the panic path.
fn release_task(task: *mut Task, report: bool) -> usize {
    release_inner(task, report, false).unwrap_or(0)
}

fn release_inner(task: *mut Task, report: bool, try_only: bool) -> Option<usize> {
    let r = registry()?;
    let mut n = 0;
    loop {
        // Take the newest entry of the task out of the table, then release
        // it with the lock no longer held (a release may block briefly).
        let take = |t: &mut [Entry; CAPACITY]| {
            let k = t.iter().enumerate().filter(|(_, e)| e.id != 0 && e.task == task).max_by_key(|(_, e)| e.id).map(|(k, _)| k)?;
            let e = t[k];
            t[k] = EMPTY;
            Some(e)
        };
        let e = if try_only { r.try_with(take)? } else { r.with(take) };
        let Some(e) = e else { break };
        if report {
            match e.at {
                Some(at) => {
                    let _ = writeln!(Serial, "track: releasing {} made at {}:{}", e.kind, at.file(), at.line());
                }
                None => {
                    let _ = writeln!(Serial, "track: releasing {}", e.kind);
                }
            }
        }
        if let Some(f) = e.release {
            // SAFETY: the release function registered for this resource,
            // in the task that registered it; the entry is gone, so it is
            // released once.
            unsafe { f(e.a, e.b) };
        }
        n += 1;
    }
    Some(n)
}

// ---------------------------------------------------------------------------
// The panic path
// ---------------------------------------------------------------------------

/// struct Task's tc_SPLower and tc_SPUpper (tests/layout/exec.c).
#[cfg(target_os = "amigaos")]
const TC_SPLOWER: usize = 58;
#[cfg(target_os = "amigaos")]
const TC_SPUPPER: usize = 62;
const MAX_FRAMES: usize = 32;

/// Return addresses from the caller's stack frame outwards (the PowerPC
/// SysV back chain: word 0 of a frame is the caller's frame, the return
/// address is at +4 there), bounded by the task's stack.
fn backtrace(out: &mut [usize; MAX_FRAMES]) -> usize {
    #[cfg(target_os = "amigaos")]
    {
        let sp: usize;
        // SAFETY: reads r1.
        unsafe { core::arch::asm!("mr {0}, 1", out(reg) sp, options(nomem, nostack, preserves_flags)) };
        let t = current_task() as *const u8;
        // SAFETY: the running task's stack bounds.
        let (lo, hi) = unsafe { (core::ptr::read_unaligned(t.add(TC_SPLOWER) as *const usize), core::ptr::read_unaligned(t.add(TC_SPUPPER) as *const usize)) };
        let mut n = 0;
        let mut sp = sp;
        while n < MAX_FRAMES {
            if sp < lo || sp + 8 > hi || sp & 3 != 0 {
                break;
            }
            // SAFETY: inside the task's stack (checked above).
            let prev = unsafe { core::ptr::read(sp as *const usize) };
            if prev <= sp || prev + 8 > hi || prev & 3 != 0 {
                break;
            }
            // SAFETY: as above, for the caller's frame.
            let lr = unsafe { core::ptr::read((prev + 4) as *const usize) };
            if lr != 0 {
                out[n] = lr;
                n += 1;
            }
            sp = prev;
        }
        n
    }
    #[cfg(not(target_os = "amigaos"))]
    {
        let _ = out;
        0
    }
}

/// dos.library's interface for the crash file, opened through exec and
/// kept: not the program's IDOS global, which a driver or library does not
/// define (and must not get a weak stand-in: see tools/gen-weak-globals.py).
#[cfg(feature = "dos")]
fn dos_iface() -> core::result::Result<*mut sys::DOSIFace, ()> {
    static DOS: AtomicPtr<sys::DOSIFace> = AtomicPtr::new(core::ptr::null_mut());
    let d = DOS.load(Ordering::Acquire);
    if !d.is_null() {
        return Ok(d);
    }
    // SAFETY: exec calls with NUL-terminated names; the library stays open
    // and the interface obtained for the rest of the program (a panic path
    // may need it again).
    let i = unsafe {
        let lib = sys::exec_open_library(c"dos.library".as_ptr() as *const u8, 52);
        if lib.is_null() {
            return Err(());
        }
        let i = sys::exec_get_interface(lib, c"main".as_ptr() as *const u8, 1, core::ptr::null()) as *mut sys::DOSIFace;
        if i.is_null() {
            sys::exec_close_library(lib);
            return Err(());
        }
        i
    };
    match DOS.compare_exchange(core::ptr::null_mut(), i, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => Ok(i),
        // Another task got there first; ours stays obtained (harmless).
        Err(other) => Ok(other),
    }
}

/// Serial output, and a DOS file when one could be opened.
struct Report {
    file: u32,
}

impl Write for Report {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let _ = Serial.write_str(s);
        if self.file != 0 {
            #[cfg(feature = "dos")]
            if let Ok(i) = dos_iface() {
                // SAFETY: our open file and `s`'s bytes.
                unsafe { ((*i).Write)(i, self.file, s.as_ptr() as APTR, s.len() as i32) };
            }
        }
        Ok(())
    }
}

/// T:<name>.crash for the calling Process, from its task name (a Shell
/// command's is its path: the file part is used). 0 if not possible.
fn open_crash_file() -> u32 {
    #[cfg(feature = "dos")]
    {
        if !crate::dos::is_process() {
            return 0;
        }
        let Ok(i) = dos_iface() else { return 0 };
        // A Shell command's process is the Shell's ("Background CLI" under
        // a server): name the file after the command (GetCliProgramName),
        // else after the task.
        let mut cmd = [0u8; 104];
        // SAFETY: an open interface in a Process; GetCliProgramName writes
        // at most the length given, NUL-terminated.
        let have_cmd = unsafe { !((*i).Cli)(i).is_null() && ((*i).GetCliProgramName)(i, cmd.as_mut_ptr(), cmd.len() as i32 - 1) != 0 && cmd[0] != 0 };
        let name = if have_cmd {
            cmd.as_ptr()
        } else {
            let t = current_task() as *const u8;
            // SAFETY: the running task's ln_Name (offset 10), NUL-terminated.
            unsafe { core::ptr::read_unaligned(t.add(10) as *const *const u8) }
        };
        let mut path = [0u8; 128];
        path[..2].copy_from_slice(b"T:");
        let mut n = 2;
        if !name.is_null() {
            // SAFETY: as above; read up to its NUL, at most 100 bytes.
            let full = unsafe { core::ffi::CStr::from_ptr(name as *const core::ffi::c_char) }.to_bytes();
            let part = full.rsplit(|&c| c == b'/' || c == b':').next().unwrap_or(full);
            for &c in part.iter().take(100) {
                path[n] = if c == b' ' { b'_' } else { c };
                n += 1;
            }
        } else {
            path[n..n + 7].copy_from_slice(b"program");
            n += 7;
        }
        path[n..n + 6].copy_from_slice(b".crash");
        // path[n + 6] is the terminating NUL.
        // SAFETY: an open interface in a Process and a NUL-terminated name.
        unsafe { ((*i).Open)(i, path.as_ptr(), sys::MODE_NEWFILE) }
    }
    #[cfg(not(feature = "dos"))]
    {
        0
    }
}

fn close_crash_file(file: u32) {
    #[cfg(feature = "dos")]
    if file != 0 {
        if let Ok(i) = dos_iface() {
            // SAFETY: the file opened by open_crash_file, closed once.
            unsafe { ((*i).Close)(i, file) };
        }
    }
    #[cfg(not(feature = "dos"))]
    let _ = file;
}

/// Whether the running task is a DOS Process (its Node's ln_Type, offset 8).
#[cfg(feature = "debug")]
fn in_process() -> bool {
    let t = current_task() as *const u8;
    // SAFETY: the running task's node type.
    !t.is_null() && unsafe { *t.add(8) } == sys::NT_PROCESS
}

#[cfg(feature = "debug")]
fn describe(w: &mut Report, addr: usize) {
    use crate::sys::DebugIFace;
    // ObtainDebugSymbol never returned in a plain exec task (a panicking
    // crate task, exectest): symbols only in a Process.
    if !in_process() {
        let _ = writeln!(w, "  {:08x}", addr);
        return;
    }
    static DEBUG: AtomicPtr<DebugIFace> = AtomicPtr::new(core::ptr::null_mut());
    let mut d = DEBUG.load(Ordering::Acquire);
    if d.is_null() {
        // SAFETY: IExec's InterfaceData names exec's base (offset 0 of the
        // interface data, lib_base); GetInterface on it allocates nothing of
        // ours. The interface is kept for later panics.
        unsafe {
            let base = (*(sys::IExec as *const sys::InterfaceData)).lib_base;
            d = sys::exec_get_interface(base, c"debug".as_ptr() as *const u8, 1, core::ptr::null()) as *mut DebugIFace;
        }
        DEBUG.store(d, Ordering::Release);
    }
    if d.is_null() {
        let _ = writeln!(w, "  {:08x}", addr);
        return;
    }
    // SAFETY: an open debug interface; ObtainDebugSymbol only looks the
    // address up. The DebugSymbol (Name at 8, SegmentOffset 20,
    // SourceFunctionName 32: tests/layout/exec.c) is read, then released.
    unsafe {
        let s = ((*d).ObtainDebugSymbol)(d, addr as *const c_void, core::ptr::null_mut());
        if s.is_null() {
            let _ = writeln!(w, "  {:08x}", addr);
            return;
        }
        let p = s as *const u8;
        let name = core::ptr::read_unaligned(p.add(8) as *const *const core::ffi::c_char);
        let off = core::ptr::read_unaligned(p.add(20) as *const u32);
        let func = core::ptr::read_unaligned(p.add(32) as *const *const core::ffi::c_char);
        let text = |q: *const core::ffi::c_char| if q.is_null() { "" } else { core::ffi::CStr::from_ptr(q).to_str().unwrap_or("?") };
        let _ = writeln!(w, "  {:08x} {}+{:x} {}", addr, text(name), off, text(func));
        ((*d).ReleaseDebugSymbol)(d, s);
    }
}

#[cfg(not(feature = "debug"))]
fn describe(w: &mut Report, addr: usize) {
    let _ = writeln!(w, "  {:08x}", addr);
}

fn wait_forever() -> ! {
    loop {
        // SAFETY: Wait(0) sleeps for good, using no CPU.
        unsafe { sys::exec_wait(0) };
    }
}

/// Open the crash file and write the panic line, the task and the
/// backtrace (serial and file).
fn begin_report(loc: Option<&Location<'_>>, msg: &dyn core::fmt::Display) -> Report {
    let me = current_task();
    let file = open_crash_file();
    let mut w = Report { file };
    let _ = w.write_str("PANIC");
    if let Some(loc) = loc {
        let _ = write!(w, " at {}:{}", loc.file(), loc.line());
    }
    let mut probe = Probe(false);
    let _ = write!(probe, "{}", msg);
    if probe.0 {
        let _ = write!(w, ": {}", msg);
    }
    let t = me as *const u8;
    // SAFETY: the running task's ln_Name (offset 10).
    let name = unsafe { core::ptr::read_unaligned(t.add(10) as *const *const core::ffi::c_char) };
    let tname = if name.is_null() {
        "?"
    } else {
        // SAFETY: a task name, NUL-terminated, alive while the task runs.
        unsafe { core::ffi::CStr::from_ptr(name) }.to_str().unwrap_or("?")
    };
    let _ = writeln!(w, "\ntask {:08x} \"{}\"; backtrace:", me as usize, tname);
    let mut frames = [0usize; MAX_FRAMES];
    let n = backtrace(&mut frames);
    for &a in &frames[..n] {
        describe(&mut w, a);
    }
    w
}

/// Whether a message writes anything.
struct Probe(bool);
impl Write for Probe {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0 |= !s.is_empty();
        Ok(())
    }
}

/// For std (clib4) programs, which panic through Rust's std, not
/// [`on_panic`]: install a panic hook that writes the same report (serial
/// and `T:<command>.crash`). With `panic = "abort"` it also releases what
/// the task has registered, since std then aborts without running
/// destructors; with unwinding the destructors release it, so the hook
/// only reports. The previous hook (std's message on stderr) runs after.
#[cfg(feature = "std")]
pub fn install_std_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(std::boxed::Box::new(move |info| {
        let text: &dyn core::fmt::Display = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s
        } else if let Some(s) = info.payload().downcast_ref::<std::string::String>() {
            s
        } else {
            &""
        };
        let mut w = begin_report(info.location(), text);
        if cfg!(panic = "abort") {
            match release_inner(current_task(), true, true) {
                Some(k) => {
                    let _ = writeln!(w, "{} resource(s) released before the abort", k);
                }
                None => {
                    let _ = writeln!(w, "the resource registry was busy: nothing released");
                }
            }
        } else {
            let _ = writeln!(w, "unwinding: the destructors release what is held");
        }
        close_crash_file(w.file);
        prev(info);
    }));
}

/// The panic path: report, release the task's resources, return to its
/// catch point (or sleep). See the module documentation.
pub fn on_panic(info: &PanicInfo) -> ! {
    let me = current_task();
    let slot = slot_of(me);
    // A panic inside this path sleeps at once.
    match slot {
        Some(s) => {
            if s.panicking.swap(true, Ordering::AcqRel) {
                wait_forever();
            }
        }
        None => {
            if LONE_PANIC.compare_exchange(core::ptr::null_mut(), me, Ordering::AcqRel, Ordering::Acquire).is_err() {
                wait_forever();
            }
        }
    }

    let msg = info.message();
    let mut w = match msg.as_str() {
        Some(text) => begin_report(info.location(), &text),
        None => begin_report(info.location(), &msg),
    };
    let file = w.file;

    let children = LIVE_CHILDREN.load(Ordering::Acquire);
    let is_main = me == MAIN_TASK.load(Ordering::Acquire);
    if is_main && children > 0 {
        // The children may be using what the main task made.
        let _ = writeln!(w, "{} child task(s) still run: nothing released", children);
    } else {
        match release_inner(me, true, true) {
            Some(k) => {
                let _ = writeln!(w, "{} resource(s) released", k);
            }
            None => {
                let _ = writeln!(w, "the resource registry was busy: nothing released");
            }
        }
    }
    let buf = slot.map(|s| s.buf.load(Ordering::Acquire)).unwrap_or(core::ptr::null_mut());
    let end_here = buf.is_null() || (is_main && children > 0);
    if end_here {
        let _ = writeln!(
            w,
            "{}: the task waits for good",
            if buf.is_null() { "no catch point" } else { "child tasks still run in the program's code" }
        );
    } else {
        let _ = writeln!(w, "returning to the catch point (RETURN_FAIL)");
    }
    close_crash_file(file);

    if end_here {
        wait_forever();
    }
    if let Some(s) = slot {
        s.panicking.store(false, Ordering::Release);
    }
    #[cfg(target_os = "amigaos")]
    // SAFETY: `buf` is the task's innermost catch point, whose amigaos4_catch
    // is still running further up this stack.
    unsafe {
        amigaos4_throw(buf, 20)
    }
    #[cfg(not(target_os = "amigaos"))]
    wait_forever()
}
