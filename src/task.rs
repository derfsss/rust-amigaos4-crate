//! Exec tasks: spawn a closure as a task and join it, plus the calling
//! task's priority, CPU affinity, and whether it may block.
//!
//! [`spawn`] / [`Builder::spawn`] create the task with CreateTask (V50)
//! and always use a start-up handshake: the child first waits for CTRL-F,
//! and the parent sends it once CreateTask has returned. The child never
//! runs before the parent holds its handle, whatever the priorities (exec
//! 54.57 may run a new higher-priority task at once or later; a child that
//! ran in that gap has crashed DOS).
//!
//! An exec task may not call dos.library, nor anything that may load a
//! disk-based library or device (exec.doc, CreateTask). So no `std` I/O in
//! the closure: use `serial_println!` or hand results back. DOS processes
//! are in `crate::dos::process`.
//!
//! The control block (the closure, the result, the done flag) is MEMF_SHARED
//! memory of its own, whatever the program's allocator. What the closure
//! captures lives where it was allocated: with `ExecAllocator`
//! (MEMF_PRIVATE) another task should not use it; use
//! `SharedExecAllocator` for programs whose tasks share heap memory.
//!
//! [`JoinHandle`] is not `Send`: the done signal is a bit of the spawning
//! task, and only that task can wait for it. Dropping a handle joins.
//!
//! The end of a child: the closure's body (Rust) writes the result, sets
//! the done flag and returns the parent and the signal mask to a small
//! assembly trampoline, which jumps to exec's Signal with the trampoline's
//! own return address: a tail call. Signal then returns straight into
//! exec's task finalizer. Once the parent is signalled, no instruction of
//! the program runs on the child any more, so the program may end and be
//! unloaded right after a join, whatever the priorities. A child that
//! panics returns to its catch point (`track::catch`), what it registered
//! is released, and it signals as usual: its join gives an error.

use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{self, null_mut};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::error::{AmigaError, Result};
use crate::signal::{current_task, wait, Signal};
use crate::sys::{self, Task, TagItem, APTR, TAG_DONE};

/// Default stack for a spawned task.
pub const DEFAULT_STACK: u32 = 65536;

/// Create a task named `name` (NUL-terminated, static: exec keeps the
/// pointer) at priority 0 with [`DEFAULT_STACK`], running `f`.
pub fn spawn<F, R>(name: &'static [u8], f: F) -> Result<JoinHandle<R>>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    Builder::new(name).spawn(f)
}

/// A task's name, priority and stack before it is spawned.
pub struct Builder {
    name: &'static [u8],
    pri: i8,
    stack: u32,
    lock_stack: bool,
}

impl Builder {
    /// `name` is NUL-terminated (`amstr!`) and static.
    pub const fn new(name: &'static [u8]) -> Self {
        Self { name, pri: 0, stack: DEFAULT_STACK, lock_stack: false }
    }

    pub const fn priority(mut self, pri: i8) -> Self {
        self.pri = pri;
        self
    }

    /// Stack size in bytes ("don't cut it too close", exec.doc).
    pub const fn stack(mut self, bytes: u32) -> Self {
        self.stack = bytes;
        self
    }

    /// Lock the stack in memory (CT_LockStack).
    pub const fn lock_stack(mut self, on: bool) -> Self {
        self.lock_stack = on;
        self
    }

    pub fn spawn<F, R>(self, f: F) -> Result<JoinHandle<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        if self.name.last() != Some(&0) {
            return Err(AmigaError::NotNulTerminated);
        }
        let done = Signal::alloc()?;
        let pk = Packet::<F, R>::alloc(f, current_task(), done.mask())?;
        let body: extern "C" fn(u32) -> u64 = body::<F, R>;
        let tags = [
            TagItem { ti_Tag: sys::AT_PARAM1, ti_Data: pk as u32 },
            TagItem { ti_Tag: sys::AT_PARAM2, ti_Data: body as usize as u32 },
            TagItem { ti_Tag: sys::CT_LOCK_STACK, ti_Data: self.lock_stack as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a static NUL-terminated name, the trampoline (native,
        // taking the packet and the body in r3 and r4), a valid tag list.
        let task = unsafe {
            sys::exec_create_task(
                self.name.as_ptr(),
                self.pri as i32,
                amigaos4_task_trampoline as *const core::ffi::c_void,
                self.stack,
                tags.as_ptr(),
            )
        };
        if task.is_null() {
            // SAFETY: no task saw the packet.
            unsafe { Packet::<F, R>::free(pk) };
            return Err(AmigaError::AllocationFailed);
        }
        crate::track::child_started();
        // The handshake: the child waits for this before touching anything.
        // SAFETY: the child cannot have ended: it is waiting for CTRL-F.
        unsafe { sys::exec_signal(task, sys::SIGBREAKF_CTRL_F) };
        Ok(JoinHandle {
            task,
            packet: pk as *mut u8,
            // SAFETY: `pk` is the live packet; only its address is taken.
            done_flag: unsafe { ptr::addr_of!((*pk).done) },
            finish: Packet::<F, R>::finish,
            done,
            spawner: current_task(),
            joined: false,
            _not_send: PhantomData,
        })
    }
}

/// The child's control block, in MEMF_SHARED memory.
struct Packet<F, R> {
    f: Option<F>,
    result: Option<R>,
    parent: *mut Task,
    done_mask: u32,
    done: AtomicU32,
}

impl<F, R> Packet<F, R> {
    fn alloc(f: F, parent: *mut Task, done_mask: u32) -> Result<*mut Self> {
        let tags = [
            TagItem { ti_Tag: sys::AVT_TYPE, ti_Data: sys::MEMF_SHARED },
            TagItem { ti_Tag: sys::AVT_ALIGNMENT, ti_Data: align_of::<Self>().max(8) as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a valid tag list.
        let p = unsafe { sys::exec_alloc_vec_tag_list(size_of::<Self>() as u32, tags.as_ptr()) } as *mut Self;
        if p.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        // SAFETY: fresh memory of the right size and alignment.
        unsafe {
            p.write(Self { f: Some(f), result: None, parent, done_mask, done: AtomicU32::new(0) });
        }
        Ok(p)
    }

    /// Drop the contents and free the memory.
    ///
    /// # Safety
    /// `p` came from [`alloc`](Self::alloc), and no task uses it any more.
    unsafe fn free(p: *mut Self) {
        // SAFETY: the caller's contract: a live, initialised packet, unused.
        unsafe {
            ptr::drop_in_place(p);
            sys::exec_free_vec(p as APTR);
        }
    }

    /// After the child is done: take the result, free the packet.
    ///
    /// # Safety
    /// `p` is a `Packet<F, R>` from [`alloc`](Self::alloc) whose child has
    /// set `done` and signalled (it no longer touches the packet).
    unsafe fn finish(p: *mut u8) -> Option<R> {
        let p = p as *mut Self;
        // SAFETY: the caller's contract.
        unsafe {
            let r = (*p).result.take();
            Self::free(p);
            r
        }
    }
}

extern "C" {
    /// The child's entry (below): r3 = the packet, r4 = `body::<F, R>`.
    fn amigaos4_task_trampoline(packet: u32, body: extern "C" fn(u32) -> u64);
}

// The trampoline calls `body(packet)`, which returns the parent's Task in
// the high word (r3) and the done mask in the low word (r4) (PowerPC SysV:
// a u64 comes back in r3:r4). It then restores its own return address and
// jumps to IExec->Signal(IExec, parent, mask) without a frame: Signal
// returns to exec's task finalizer, never to this program's code.
#[cfg(target_os = "amigaos")]
core::arch::global_asm!(
    ".section .text.amigaos4_task_trampoline,\"ax\",@progbits",
    ".globl amigaos4_task_trampoline",
    ".type amigaos4_task_trampoline,@function",
    ".p2align 2",
    "amigaos4_task_trampoline:",
    "mflr 0",
    "stwu 1, -16(1)",
    "stw 0, 20(1)",
    "mtctr 4",
    "bctrl",
    "lwz 0, 20(1)",
    "addi 1, 1, 16",
    "mtlr 0",
    "mr 5, 4",
    "mr 4, 3",
    "lis 3, {iexec}@ha",
    "lwz 3, {iexec}@l(3)",
    "lwz 12, {signal}(3)",
    "mtctr 12",
    "bctr",
    ".size amigaos4_task_trampoline, . - amigaos4_task_trampoline",
    ".previous",
    iexec = sym sys::IExec,
    signal = const core::mem::offset_of!(sys::ExecIFace, Signal),
);

/// The child's work, called by the trampoline with the packet (AT_Param1).
/// Returns `(parent << 32) | done mask` for the trampoline's Signal.
extern "C" fn body<F: FnOnce() -> R, R>(param: u32) -> u64 {
    // The handshake: nothing runs before the parent has the handle.
    wait(sys::SIGBREAKF_CTRL_F);
    let pk = param as *mut Packet<F, R>;
    // SAFETY: the parent keeps the packet until it receives the done signal,
    // which the trampoline sends after this returns.
    unsafe {
        if let Some(f) = (*pk).f.take() {
            // A panic comes back here as Err: no result (join gives an
            // error), what the child had registered released.
            (*pk).result = crate::track::catch(f).ok();
        }
        crate::track::child_finished((*pk).parent);
        let parent = (*pk).parent as u32;
        let mask = (*pk).done_mask;
        (*pk).done.store(1, Ordering::Release);
        // The packet is not touched after this point.
        ((parent as u64) << 32) | mask as u64
    }
}

/// A spawned task. [`join`](Self::join) waits for its result; dropping the
/// handle joins too (and drops the result).
pub struct JoinHandle<R> {
    task: *mut Task,
    packet: *mut u8,
    done_flag: *const AtomicU32,
    finish: unsafe fn(*mut u8) -> Option<R>,
    done: Signal,
    spawner: *mut Task,
    joined: bool,
    _not_send: PhantomData<*mut ()>,
}

impl<R> JoinHandle<R> {
    /// The child's Task (valid until it has ended; for display and
    /// comparisons, not for signalling after the join).
    pub fn task(&self) -> *mut Task {
        self.task
    }

    /// Has the child finished its closure? (It may still be on its way out.)
    pub fn is_finished(&self) -> bool {
        // SAFETY: the packet is alive until finish().
        unsafe { (*self.done_flag).load(Ordering::Acquire) != 0 }
    }

    /// Wait for the child and take its result.
    pub fn join(mut self) -> Result<R> {
        self.join_inner()
    }

    fn join_inner(&mut self) -> Result<R> {
        if self.joined {
            return Err(AmigaError::NullPointer);
        }
        if current_task() != self.spawner {
            return Err(AmigaError::WrongTask);
        }
        // The child signals once, after setting `done`: wait for that
        // signal itself, so the parent never ends while a Signal() to it is
        // still to come.
        loop {
            self.done.wait();
            // SAFETY: the packet is alive until finish().
            if unsafe { (*self.done_flag).load(Ordering::Acquire) } != 0 {
                break;
            }
        }
        self.joined = true;
        // SAFETY: done: the child no longer touches the packet.
        unsafe { (self.finish)(self.packet) }.ok_or(AmigaError::NullPointer)
    }
}

impl<R> Drop for JoinHandle<R> {
    fn drop(&mut self) {
        // An error with `joined` still false is only reachable from the
        // wrong task: leak rather than free a packet the child may use.
        if !self.joined && self.join_inner().is_err() && !self.joined {
            crate::serial_println!("amigaos4::task: JoinHandle dropped on the wrong task; leaked");
        }
    }
}

// ---------------------------------------------------------------------------
// The calling task
// ---------------------------------------------------------------------------

/// The calling task's priority (tc_Node.ln_Pri, offset 9 of a Task:
/// checked against SDK 54.25).
///
/// Spawn children relative to it: a child above the parent that never
/// waits starves the parent (a Shell job may run at -2, so an
/// absolute priority 0 is above it).
pub fn priority() -> i8 {
    // SAFETY: the caller's own Task; ln_Pri is a byte at offset 9, read
    // volatile because SetTaskPri may change it.
    unsafe { core::ptr::read_volatile((current_task() as *const u8).add(9) as *const i8) }
}

/// Set the calling task's priority; returns the old one.
pub fn set_priority(pri: i8) -> i8 {
    // SAFETY: the caller's own task.
    unsafe { sys::exec_set_task_pri(current_task(), pri) }
}

/// Keep the calling task on the core it is on now (SetTaskAffinity,
/// V54.48; nothing happens on a single-core system).
pub fn pin_to_current_core() {
    let tags = [
        TagItem { ti_Tag: sys::STA_CORE, ti_Data: sys::STACORE_CLEAR as u32 },
        TagItem { ti_Tag: sys::STA_CORE, ti_Data: sys::STACORE_CURRENT as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: NULL = the calling task; a valid tag list.
    unsafe { sys::exec_set_task_affinity(null_mut(), tags.as_ptr()) }
}

/// Let the calling task run on any core again.
pub fn clear_affinity() {
    let tags = [
        TagItem { ti_Tag: sys::STA_CORE, ti_Data: sys::STACORE_CLEAR as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: as above.
    unsafe { sys::exec_set_task_affinity(null_mut(), tags.as_ptr()) }
}

/// May the caller block (Wait, lock)? False inside Forbid or Disable, or in
/// an interrupt (GetSystemInfo, V54.27).
pub fn can_block() -> bool {
    let (mut sched, mut ints, mut in_int) = (0u32, 0u32, 0u32);
    let tags = [
        TagItem { ti_Tag: sys::GSITAG_SCHEDULER_STATE, ti_Data: ptr::addr_of_mut!(sched) as u32 },
        TagItem { ti_Tag: sys::GSITAG_INTERRUPT_STATE, ti_Data: ptr::addr_of_mut!(ints) as u32 },
        TagItem { ti_Tag: sys::GSITAG_PROCESSING_INTERRUPT, ti_Data: ptr::addr_of_mut!(in_int) as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: each tag points at a u32 on this stack.
    unsafe { sys::exec_get_system_info(tags.as_ptr()) };
    sched == sys::GSISTATE_ENABLE && ints == sys::GSISTATE_ENABLE && in_int == 0
}

/// The number of CPUs (GetCPUInfo GCIT_NumberOfCPUs).
pub fn cpu_count() -> u32 {
    let mut n = 1u32;
    let tags = [
        TagItem { ti_Tag: sys::GCIT_NUMBER_OF_CPUS, ti_Data: ptr::addr_of_mut!(n) as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: the tag points at a u32 on this stack.
    unsafe { sys::exec_get_cpuinfo(tags.as_ptr()) };
    n
}
