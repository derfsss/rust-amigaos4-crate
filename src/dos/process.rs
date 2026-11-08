//! DOS processes: run a closure as a child Process (CreateNewProc) and
//! wait for it (WaitForChildExit, dos.library 54.107).
//!
//! Unlike an exec task (`crate::task`), a child Process may use
//! dos.library: files, locks, variables, its own streams.
//!
//! How it is made safe:
//! - Start-up handshake: the child first waits for CTRL-F, which the
//!   parent sends once CreateNewProc has returned. dos.library 54.136
//!   links a new process into its list only after it has been started; a
//!   child that ran and ended in that gap crashed DOS. With the handshake
//!   no child runs before the parent holds it.
//! - NP_Child: the child is a dependant of the parent, so DOS never
//!   unloads the parent's code while the child runs in it, and
//!   WaitForChildExit(pid) returns only once the child has fully ended.
//!   [`Child::wait`] (or dropping the `Child`) does that, so the result
//!   is read from the MEMF_SHARED control block after the child is gone.
//! - What the child is given (input and output files, a current
//!   directory lock) becomes the child's: DOS closes and unlocks them when
//!   it ends. If CreateNewProc fails they are still the caller's, and are
//!   dropped here.
//!
//! A child's default input and output are NIL:. A child that panics gives
//! no result: `wait` returns `Err(NullPointer)`.
//!
//! The control block is MEMF_SHARED memory of its own. What the closure
//! captures lives where it was allocated: with `ExecAllocator`
//! (MEMF_PRIVATE) the child should not use it; programs whose processes
//! share heap memory use `SharedExecAllocator`.

use alloc::vec::Vec;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};

use super::fs::{File, Lock};
use super::{last_error, process};
use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, TagItem, APTR, TAG_DONE};

/// Default stack for a child process.
pub const DEFAULT_STACK: u32 = 65536;

/// struct Task's tc_UserData offset (tests/layout/dos.c).
const TC_USERDATA: usize = 88;

/// The child's control block, in MEMF_SHARED memory.
struct Packet<F, R> {
    f: Option<F>,
    result: Option<R>,
    /// The spawning task: what the child leaves registered passes to it.
    parent: *mut crate::sys::Task,
}

/// How to make a child process.
pub struct ProcessBuilder {
    name: Vec<u8>,
    priority: Option<i8>,
    stack: u32,
    input: Option<File>,
    output: Option<File>,
    dir: Option<Lock>,
}

impl ProcessBuilder {
    pub fn new(name: &str) -> Result<Self> {
        Ok(Self { name: c_copy(name.as_bytes())?, priority: None, stack: DEFAULT_STACK, input: None, output: None, dir: None })
    }

    /// The child's priority (default: the parent's).
    pub fn priority(mut self, p: i8) -> Self {
        self.priority = Some(p);
        self
    }

    pub fn stack(mut self, bytes: u32) -> Self {
        self.stack = bytes.max(8192);
        self
    }

    /// The child's input stream (closed by DOS when the child ends).
    pub fn input(mut self, f: File) -> Self {
        self.input = Some(f);
        self
    }

    /// The child's output stream (closed by DOS when the child ends).
    pub fn output(mut self, f: File) -> Self {
        self.output = Some(f);
        self
    }

    /// The child's current directory (unlocked by DOS when the child
    /// ends); default: a copy of the parent's.
    pub fn current_dir(mut self, l: Lock) -> Self {
        self.dir = Some(l);
        self
    }

    /// Start `f` in a new child Process.
    pub fn spawn<F, R>(self, f: F) -> Result<Child<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let i = process()?;
        if super::version() < (54, 107) {
            return Err(AmigaError::Unsupported("WaitForChildExit by PID needs dos.library 54.107"));
        }
        let packet = alloc_packet::<F, R>(f)?;
        let entry: extern "C" fn(*const u8, i32, APTR) -> i32 = entry::<F, R>;
        let mut tags = alloc::vec![
            TagItem { ti_Tag: sys::NP_START, ti_Data: entry as usize as u32 },
            TagItem { ti_Tag: sys::NP_NAME, ti_Data: self.name.as_ptr() as u32 },
            TagItem { ti_Tag: sys::NP_STACK_SIZE, ti_Data: self.stack },
            TagItem { ti_Tag: sys::NP_USER_DATA, ti_Data: packet as u32 },
            TagItem { ti_Tag: sys::NP_CHILD, ti_Data: 1 },
        ];
        if let Some(p) = self.priority {
            tags.push(TagItem { ti_Tag: sys::NP_PRIORITY, ti_Data: p as i32 as u32 });
        }
        if let Some(f) = &self.input {
            tags.push(TagItem { ti_Tag: sys::NP_INPUT, ti_Data: f.bptr() });
            tags.push(TagItem { ti_Tag: sys::NP_CLOSE_INPUT, ti_Data: 1 });
        }
        if let Some(f) = &self.output {
            tags.push(TagItem { ti_Tag: sys::NP_OUTPUT, ti_Data: f.bptr() });
            tags.push(TagItem { ti_Tag: sys::NP_CLOSE_OUTPUT, ti_Data: 1 });
        }
        if let Some(l) = &self.dir {
            tags.push(TagItem { ti_Tag: sys::NP_CURRENT_DIR, ti_Data: l.bptr() });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: an open interface in a Process; a TAG_DONE-ended list:
        // our entry, a NUL-terminated name, the packet, and handles we own
        // (given to the child only if this succeeds).
        let proc = unsafe { ((*i).CreateNewProc)(i, tags.as_ptr()) };
        if proc.is_null() {
            let e = last_error(i);
            // SAFETY: no child was made: the packet is ours again.
            unsafe { free_packet::<F, R>(packet) };
            return Err(e);
        }
        // SAFETY: straight after CreateNewProc: its PID (V51.94).
        let pid = unsafe { ((*i).IoErr)(i) } as u32;
        // The child owns these now; DOS closes and unlocks them.
        if let Some(f) = self.input {
            let _ = f.into_raw();
        }
        if let Some(f) = self.output {
            let _ = f.into_raw();
        }
        if let Some(l) = self.dir {
            let _ = l.into_raw();
        }
        crate::track::child_started();
        // SAFETY: the child waits for CTRL-F before anything else, so it is
        // alive and `proc` valid here.
        unsafe { sys::exec_signal(proc as *mut sys::Task, sys::SIGBREAKF_CTRL_F) };
        Ok(Child { pid, packet: packet as *mut u8, finish: finish_packet::<F, R>, done: false, _name: self.name, _r: PhantomData, _p: PhantomData })
    }
}

impl ProcessBuilder {
    /// Start `f` in a Process that belongs to no one: not NP_Child, never
    /// waited for. For a process that outlives its starter (a library's or
    /// device's server, started by its first opener and stopped by its
    /// last). The same start-up handshake; a panic in `f` ends it through
    /// its catch point; whatever it still has registered when `f` returns
    /// is released by the process itself, which then frees its own start-up
    /// block. Returns the PID.
    ///
    /// The code `f` runs in must stay loaded until the process has gone:
    /// a library or device signals it to stop and then waits until
    /// FindTask(name) no longer finds it (give it a name of its own) before
    /// its segment may be unloaded.
    pub fn spawn_detached<F>(self, f: F) -> Result<u32>
    where
        F: FnOnce() + Send + 'static,
    {
        let i = process()?;
        let packet = alloc_packet::<F, ()>(f)?;
        let entry: extern "C" fn(*const u8, i32, APTR) -> i32 = entry_detached::<F>;
        let mut tags = alloc::vec![
            TagItem { ti_Tag: sys::NP_START, ti_Data: entry as usize as u32 },
            TagItem { ti_Tag: sys::NP_NAME, ti_Data: self.name.as_ptr() as u32 },
            TagItem { ti_Tag: sys::NP_STACK_SIZE, ti_Data: self.stack },
            TagItem { ti_Tag: sys::NP_USER_DATA, ti_Data: packet as u32 },
            TagItem { ti_Tag: sys::NP_CHILD, ti_Data: 0 },
        ];
        if let Some(p) = self.priority {
            tags.push(TagItem { ti_Tag: sys::NP_PRIORITY, ti_Data: p as i32 as u32 });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: as in spawn: an open interface in a Process and a
        // TAG_DONE-ended list naming our entry, a NUL-terminated name (DOS
        // copies NP_Name) and the packet.
        let proc = unsafe { ((*i).CreateNewProc)(i, tags.as_ptr()) };
        if proc.is_null() {
            let e = last_error(i);
            // SAFETY: no process was made: the packet is ours again.
            unsafe { free_packet::<F, ()>(packet) };
            return Err(e);
        }
        // SAFETY: straight after CreateNewProc: its PID.
        let pid = unsafe { ((*i).IoErr)(i) } as u32;
        // SAFETY: the process waits for CTRL-F before anything else.
        unsafe { sys::exec_signal(proc as *mut sys::Task, sys::SIGBREAKF_CTRL_F) };
        Ok(pid)
    }
}

/// A detached process's entry: the handshake, `f` through a catch point,
/// its leftovers released, its start-up block freed.
extern "C" fn entry_detached<F: FnOnce()>(_args: *const u8, _len: i32, _sysbase: APTR) -> i32 {
    crate::signal::wait(sys::SIGBREAKF_CTRL_F);
    let me = crate::signal::current_task();
    // SAFETY: our own Task; tc_UserData holds the packet, which nobody else
    // touches once the starter has signalled us; freed here, once.
    unsafe {
        let p = core::ptr::read((me as *const u8).add(TC_USERDATA) as *const *mut Packet<F, ()>);
        let f = (*p).f.take();
        free_packet::<F, ()>(p);
        if let Some(f) = f {
            let _ = crate::track::catch(f);
        }
    }
    crate::track::release_leftovers();
    0
}

/// Start `f` in a child Process named `name` with the defaults.
pub fn spawn<F, R>(name: &str, f: F) -> Result<Child<R>>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    ProcessBuilder::new(name)?.spawn(f)
}

fn alloc_packet<F, R>(f: F) -> Result<*mut Packet<F, R>> {
    let tags = [
        TagItem { ti_Tag: sys::AVT_TYPE, ti_Data: sys::MEMF_SHARED },
        TagItem { ti_Tag: sys::AVT_ALIGNMENT, ti_Data: align_of::<Packet<F, R>>().max(8) as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: a valid tag list.
    let p = unsafe { sys::exec_alloc_vec_tag_list(size_of::<Packet<F, R>>().max(1) as u32, tags.as_ptr()) } as *mut Packet<F, R>;
    if p.is_null() {
        return Err(AmigaError::AllocationFailed);
    }
    // SAFETY: fresh memory of the right size and alignment.
    unsafe { p.write(Packet { f: Some(f), result: None, parent: crate::signal::current_task() }) };
    Ok(p)
}

/// # Safety
/// `p` came from `alloc_packet::<F, R>` and no child uses it any more.
unsafe fn free_packet<F, R>(p: *mut Packet<F, R>) {
    // SAFETY: the caller's contract.
    unsafe {
        core::ptr::drop_in_place(p);
        sys::exec_free_vec(p as APTR);
    }
}

/// Take the result and free the packet.
///
/// # Safety
/// `p` is a `Packet<F, R>` whose child has fully ended.
unsafe fn finish_packet<F, R>(p: *mut u8) -> Option<R> {
    let p = p as *mut Packet<F, R>;
    // SAFETY: the caller's contract.
    unsafe {
        let r = (*p).result.take();
        free_packet(p);
        r
    }
}

/// The child's entry (NP_Start): wait for the parent's go, run the
/// closure, keep its result, return RETURN_OK to DOS.
extern "C" fn entry<F: FnOnce() -> R, R>(_args: *const u8, _len: i32, _sysbase: APTR) -> i32 {
    crate::signal::wait(sys::SIGBREAKF_CTRL_F);
    let me = crate::signal::current_task();
    // SAFETY: our own Task; tc_UserData holds the packet (NP_UserData),
    // which the parent frees only after WaitForChildExit says we ended.
    unsafe {
        let p = core::ptr::read((me as *const u8).add(TC_USERDATA) as *const *mut Packet<F, R>);
        if let Some(f) = (*p).f.take() {
            // A panic comes back here as Err: no result (wait gives an
            // error), what the child had registered released.
            (*p).result = crate::track::catch(f).ok();
        }
        crate::track::child_finished((*p).parent);
    }
    // The result is written before DOS tells the parent we ended.
    core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
    0
}

/// A running (or ended) child Process; dropping it waits for it.
pub struct Child<R> {
    pid: u32,
    packet: *mut u8,
    finish: unsafe fn(*mut u8) -> Option<R>,
    done: bool,
    // NP_Name: whether DOS copies it is not documented; kept until the
    // child has ended.
    _name: Vec<u8>,
    _r: PhantomData<R>,
    // WaitForChildExit belongs to the parent Process: not Send.
    _p: PhantomData<*const ()>,
}

impl<R> Child<R> {
    /// The child's process ID (GetPID).
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// CheckForChildExit: whether the child is still running.
    pub fn is_running(&self) -> bool {
        if self.done {
            return false;
        }
        // SAFETY: an open interface in the parent Process (Child is not
        // Send) and our child's PID.
        process().map(|i| unsafe { ((*i).CheckForChildExit)(i, self.pid) } != 0).unwrap_or(false)
    }

    fn wait_inner(&mut self) -> Result<Option<R>> {
        if self.done {
            return Ok(None);
        }
        let i = process()?;
        // SAFETY: as in is_running; returns once the child has fully ended
        // (or was never found: it ended already).
        unsafe { ((*i).WaitForChildExit)(i, self.pid) };
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
        self.done = true;
        // SAFETY: the child has ended and no longer touches the packet.
        Ok(unsafe { (self.finish)(self.packet) })
    }

    /// WaitForChildExit, then the closure's result (`Err(NullPointer)`
    /// when the child ended without one).
    pub fn wait(mut self) -> Result<R> {
        self.wait_inner()?.ok_or(AmigaError::NullPointer)
    }
}

impl<R> Drop for Child<R> {
    fn drop(&mut self) {
        let _ = self.wait_inner();
    }
}

/// GetPID: the calling Process's ID (0 in an exec task).
pub fn pid() -> u32 {
    // SAFETY: an open interface; GetPID(NULL, ...) asks about the caller
    // and may be called from a task.
    super::any().map(|i| unsafe { ((*i).GetPID)(i, core::ptr::null(), sys::GPID_PROCESS) }).unwrap_or(0)
}

/// GetPID: the calling Process's parent's ID.
pub fn parent_pid() -> u32 {
    // SAFETY: as in pid.
    super::any().map(|i| unsafe { ((*i).GetPID)(i, core::ptr::null(), sys::GPID_PARENT) }).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// The process list (ProcessScan)
// ---------------------------------------------------------------------------

/// A DOS process as ProcessScan saw it (copied out; it may have changed or
/// ended since).
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: u32,
    /// Its task name (a Shell's is "Shell Process" whatever it runs).
    pub name: alloc::string::String,
    pub priority: i8,
}

/// Every DOS process, the newest first (ProcessScan, V51.75; exec tasks
/// are not listed). The order is only roughly by PID: dos 54 on the QEMU
/// AmigaOne listed 82 before 84 and 83 (libtest). DOS holds its process list shared while it is copied, so
/// no process starts or ends during the scan. Callable from a task.
pub fn processes() -> Result<Vec<ProcessInfo>> {
    let i = super::any()?;
    let found = crate::sync::Mutex::new(Vec::new())?;
    {
        let f = &found;
        // The interface as a number: the hook must be Send + Sync, and the
        // scan calls it in this task, synchronously.
        let iu = i as usize;
        let hook = crate::hook::Hook::new(move |_userdata: APTR, p: APTR| -> u32 {
            if p.is_null() {
                return 0;
            }
            let i = iu as *mut sys::DOSIFace;
            // SAFETY: a live Process, held in DOS's list for the scan; its
            // Node's ln_Pri and ln_Name are at 9 and 10 (tests/layout/exec.c).
            // GetPID reads the given process's IDs.
            let info = unsafe {
                let b = p as *const u8;
                let name = core::ptr::read_unaligned(b.add(10) as *const *const core::ffi::c_char);
                ProcessInfo {
                    pid: ((*i).GetPID)(i, p as *const sys::Process, sys::GPID_PROCESS),
                    parent_pid: ((*i).GetPID)(i, p as *const sys::Process, sys::GPID_PARENT),
                    name: if name.is_null() {
                        alloc::string::String::new()
                    } else {
                        alloc::string::String::from_utf8_lossy(core::ffi::CStr::from_ptr(name).to_bytes()).into_owned()
                    },
                    priority: *(b.add(9) as *const i8),
                }
            };
            // Allocating here is allowed: the scan holds a semaphore, not
            // Forbid. Starting a process would deadlock; nothing here does.
            if let Ok(mut v) = f.lock() {
                v.push(info);
            }
            0
        })?;
        // SAFETY: an open interface; a hook alive across the synchronous
        // scan; reserved must be 0.
        unsafe { ((*i).ProcessScan)(i, hook.as_ptr(), core::ptr::null(), 0) };
    }
    Ok(found.into_inner())
}

// ---------------------------------------------------------------------------
// Loaded programs (LoadSeg, RunCommand, UnLoadSeg)
// ---------------------------------------------------------------------------

/// Unload a registered seglist (the panic path; see `crate::track`).
unsafe fn release_seglist(seg: usize, _: usize) {
    if let Ok(i) = process() {
        // SAFETY: the registry calls this once, in the Process that loaded
        // it, for a seglist not running (a panic in it would be in it).
        unsafe { ((*i).UnLoadSeg)(i, seg as u32) };
    }
}

/// An executable loaded into memory (LoadSeg); UnLoadSeg on drop. Not
/// `Send`: DOS tracks a seglist with the Process that loaded it, and only a
/// Process can unload a native one.
pub struct SegList {
    _t: crate::track::Tracked,
    seg: u32,
    _p: PhantomData<*mut ()>,
}

impl SegList {
    /// Load the program at `path` (ELF or hunk); the file is closed after.
    #[track_caller]
    pub fn load(path: &str) -> Result<Self> {
        let i = process()?;
        let p = c_copy(path.as_bytes())?;
        // SAFETY: an open interface in a Process and a NUL-terminated name.
        let seg = unsafe { ((*i).LoadSeg)(i, p.as_ptr()) };
        if seg == 0 { Err(last_error(i)) } else { Ok(Self { seg, _t: crate::track::register("seglist", seg as usize, 0, release_seglist), _p: PhantomData }) }
    }

    /// Run it in the calling Process (RunCommand) with `args` as its
    /// command line, on a stack of at least `stack` bytes (0: DOS's
    /// default), and return its exit code (RETURN_OK 0 .. RETURN_FAIL 20).
    /// It shares this Process's input, output and current directory, and
    /// can be run again.
    ///
    /// # Errors
    ///
    /// `DosError` when RunCommand cannot run it (it returns -1: no stack,
    /// called from a task).
    pub fn run(&self, args: &str, stack: u32) -> Result<i32> {
        let i = process()?;
        let a = c_copy(args.as_bytes())?;
        // SAFETY: an open interface in a Process, our seglist and a
        // NUL-terminated argument string (-1: its length is strlen).
        let rc = unsafe { ((*i).RunCommand)(i, self.seg, stack, a.as_ptr(), -1) };
        if rc == -1 { Err(last_error(i)) } else { Ok(rc) }
    }
}

impl Drop for SegList {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: our seglist, not running (run borrows it and returns
            // only when the program has ended), unloaded once by the
            // Process that loaded it.
            unsafe { ((*i).UnLoadSeg)(i, self.seg) };
        }
    }
}
