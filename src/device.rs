//! Building blocks for an exec device driver.
//!
//! - [`IoReq`]: an `IOStdReq` from a caller, with its fields read and
//!   written by name, and the two ways to end it ([`IoReq::finish`] for
//!   BeginIO, honouring `IOF_QUICK`; [`IoReq::reply`] for a request that was
//!   queued).
//! - [`RequestQueue`]: a unit's queue of requests waiting for its task,
//!   plus the one in progress, under an exec mutex (no Forbid), with
//!   AbortIO's two cases (remove a queued request; flag the current one).
//! - [`Worker`]: the unit task's work signal, published once the task is
//!   ready, so BeginIO can wake it or refuse while it is not there.
//! - [`answer_device_query`]: `NSCMD_DEVICEQUERY`.
//!
//! Layouts and values are those of the SDK headers (exec/io.h,
//! exec/errors.h, devices/newstyle.h, exec/nodes.h); `tests/device_consts.rs`
//! checks them against the SDK.
//!
//! The queue and the worker are `const`-constructible, so they can live in
//! a static (a kickstart module's per-unit state) and be reset there by
//! overwriting them.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use crate::error::{AmigaError, Result};
use crate::sys::{self, IORequest, Message, Task, TagItem};

// ---------------------------------------------------------------------------
// exec/io.h, exec/errors.h, devices/newstyle.h, exec/nodes.h
// ---------------------------------------------------------------------------

/// Standard device commands (exec/io.h).
pub mod cmd {
    pub const CMD_INVALID: u16 = 0;
    pub const CMD_RESET: u16 = 1;
    pub const CMD_READ: u16 = 2;
    pub const CMD_WRITE: u16 = 3;
    pub const CMD_UPDATE: u16 = 4;
    pub const CMD_CLEAR: u16 = 5;
    pub const CMD_STOP: u16 = 6;
    pub const CMD_START: u16 = 7;
    pub const CMD_FLUSH: u16 = 8;
    /// The first device-specific command.
    pub const CMD_NONSTD: u16 = 9;
    /// devices/newstyle.h.
    pub const NSCMD_DEVICEQUERY: u16 = 0x4000;
}

/// trackdisk-style commands (devices/trackdisk.h, devices/newstyle.h,
/// libraries/mounter.h) for block devices. The 64-bit forms take the byte
/// offset as io_Actual (high 32 bits) : io_Offset (low 32 bits).
pub mod td {
    use super::cmd::{CMD_CLEAR, CMD_NONSTD, CMD_READ, CMD_UPDATE, CMD_WRITE};
    /// ETD_ commands: a plain command with this bit set.
    pub const TDF_EXTCOM: u16 = 1 << 15;
    pub const TD_MOTOR: u16 = CMD_NONSTD;
    pub const TD_SEEK: u16 = CMD_NONSTD + 1;
    pub const TD_FORMAT: u16 = CMD_NONSTD + 2;
    pub const TD_REMOVE: u16 = CMD_NONSTD + 3;
    pub const TD_CHANGENUM: u16 = CMD_NONSTD + 4;
    pub const TD_CHANGESTATE: u16 = CMD_NONSTD + 5;
    pub const TD_PROTSTATUS: u16 = CMD_NONSTD + 6;
    pub const TD_RAWREAD: u16 = CMD_NONSTD + 7;
    pub const TD_RAWWRITE: u16 = CMD_NONSTD + 8;
    pub const TD_GETDRIVETYPE: u16 = CMD_NONSTD + 9;
    pub const TD_GETNUMTRACKS: u16 = CMD_NONSTD + 10;
    pub const TD_ADDCHANGEINT: u16 = CMD_NONSTD + 11;
    pub const TD_REMCHANGEINT: u16 = CMD_NONSTD + 12;
    pub const TD_GETGEOMETRY: u16 = CMD_NONSTD + 13;
    pub const TD_EJECT: u16 = CMD_NONSTD + 14;
    pub const TD_READ64: u16 = CMD_NONSTD + 15;
    pub const TD_WRITE64: u16 = CMD_NONSTD + 16;
    pub const TD_SEEK64: u16 = CMD_NONSTD + 17;
    pub const TD_FORMAT64: u16 = CMD_NONSTD + 18;
    pub const ETD_WRITE: u16 = CMD_WRITE | TDF_EXTCOM;
    pub const ETD_READ: u16 = CMD_READ | TDF_EXTCOM;
    pub const ETD_MOTOR: u16 = TD_MOTOR | TDF_EXTCOM;
    pub const ETD_SEEK: u16 = TD_SEEK | TDF_EXTCOM;
    pub const ETD_FORMAT: u16 = TD_FORMAT | TDF_EXTCOM;
    pub const ETD_UPDATE: u16 = CMD_UPDATE | TDF_EXTCOM;
    pub const ETD_CLEAR: u16 = CMD_CLEAR | TDF_EXTCOM;
    pub const NSCMD_TD_READ64: u16 = 0xC000;
    pub const NSCMD_TD_WRITE64: u16 = 0xC001;
    pub const NSCMD_TD_SEEK64: u16 = 0xC002;
    pub const NSCMD_TD_FORMAT64: u16 = 0xC003;
    pub const NSCMD_ETD_READ64: u16 = 0xE000;
    pub const NSCMD_ETD_WRITE64: u16 = 0xE001;
    pub const NSCMD_ETD_SEEK64: u16 = 0xE002;
    pub const NSCMD_ETD_FORMAT64: u16 = 0xE003;
    /// libraries/mounter.h: TD_GETGEOMETRY with a DriveGeometry64.
    pub const NSCMD_TD_GETGEOMETRY64: u16 = 0xA004;
    /// devices/scsidisk.h.
    pub const HD_SCSICMD: u16 = 28;
    /// TD_GETDRIVETYPE's answer for a 3.5" drive.
    pub const DRIVE3_5: u32 = 1;
}

/// trackdisk.device and scsidisk io_Error values (devices/trackdisk.h,
/// devices/scsidisk.h).
pub mod tderr {
    pub const TDERR_NOT_SPECIFIED: i8 = 20;
    pub const TDERR_NO_SEC_HDR: i8 = 21;
    pub const TDERR_BAD_SEC_HDR: i8 = 27;
    pub const TDERR_WRITE_PROT: i8 = 28;
    pub const TDERR_DISK_CHANGED: i8 = 29;
    pub const TDERR_SEEK_ERROR: i8 = 30;
    pub const TDERR_NO_MEM: i8 = 31;
    pub const TDERR_BAD_UNIT_NUM: i8 = 32;
    pub const TDERR_BAD_DRIVE_TYPE: i8 = 33;
    pub const HFERR_SELF_UNIT: i8 = 40;
    pub const HFERR_DMA: i8 = 41;
    pub const HFERR_PHASE: i8 = 42;
    pub const HFERR_SEL_TIMEOUT: i8 = 44;
    pub const HFERR_BAD_STATUS: i8 = 45;
}

/// NSDeviceQueryResult.DeviceType: a trackdisk-like device.
pub const NSDEVTYPE_TRACKDISK: u16 = 5;

/// io_Error values (exec/errors.h).
pub mod ioerr {
    pub const IOERR_SUCCESS: i8 = 0;
    pub const IOERR_OPENFAIL: i8 = -1;
    pub const IOERR_ABORTED: i8 = -2;
    pub const IOERR_NOCMD: i8 = -3;
    pub const IOERR_BADLENGTH: i8 = -4;
    pub const IOERR_BADADDRESS: i8 = -5;
    pub const IOERR_UNITBUSY: i8 = -6;
    pub const IOERR_SELFTEST: i8 = -7;
}

/// io_Flags: the caller asked for quick I/O (exec/io.h IOF_QUICK).
pub const IOF_QUICK: u8 = 1 << 0;
/// NSDeviceQueryResult.DeviceType: no suitable category.
pub const NSDEVTYPE_UNKNOWN: u16 = 0;
/// sizeof(struct NSDeviceQueryResult).
pub const NSDQR_SIZE: u32 = 16;
/// sizeof(struct IOStdReq).
pub const IOSTDREQ_SIZE: u32 = 48;

/// exec/nodes.h node types of a message.
const NT_MESSAGE: u8 = 5;

// struct IOStdReq on PPC (2-byte packed): Message 0..20 (Node: ln_Type at
// 8), io_Device 20, io_Unit 24, io_Command 28, io_Flags 30, io_Error 31,
// io_Actual 32, io_Length 36, io_Data 40, io_Offset 44.
const LN_TYPE: usize = 8;
const IO_DEVICE: usize = 20;
const IO_UNIT: usize = 24;
const IO_COMMAND: usize = 28;
const IO_FLAGS: usize = 30;
const IO_ERROR: usize = 31;
const IO_ACTUAL: usize = 32;
const IO_LENGTH: usize = 36;
const IO_DATA: usize = 40;
const IO_OFFSET: usize = 44;

// ---------------------------------------------------------------------------
// IoReq
// ---------------------------------------------------------------------------

/// A caller's `IOStdReq`, by pointer. Copy: it names the request, it does
/// not own it. Every accessor is unsafe in the one way the constructor is:
/// the pointer must be a live request of at least `IOSTDREQ_SIZE` bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IoReq(*mut IORequest);

// SAFETY: a request is handed between tasks by exec's own message passing;
// IoReq is just its address.
unsafe impl Send for IoReq {}

macro_rules! field {
    ($get:ident, $set:ident, $off:expr, $t:ty, $doc:literal) => {
        #[doc = concat!("Read ", $doc, ".")]
        #[inline]
        pub fn $get(self) -> $t {
            // SAFETY: from_raw's contract.
            unsafe { core::ptr::read_unaligned((self.0 as *const u8).add($off) as *const $t) }
        }
        #[doc = concat!("Write ", $doc, ".")]
        #[inline]
        // `v` is stored, never dereferenced (it may be a pointer field).
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn $set(self, v: $t) {
            // SAFETY: from_raw's contract.
            unsafe { core::ptr::write_unaligned((self.0 as *mut u8).add($off) as *mut $t, v) }
        }
    };
}

impl IoReq {
    /// # Safety
    /// `ior` is a live `IOStdReq` (at least `IOSTDREQ_SIZE` bytes) for as
    /// long as this value or a copy is used.
    #[inline]
    pub unsafe fn from_raw(ior: *mut IORequest) -> Option<Self> {
        if ior.is_null() { None } else { Some(Self(ior)) }
    }

    #[inline]
    pub fn as_ptr(self) -> *mut IORequest {
        self.0
    }

    field!(device, set_device, IO_DEVICE, *mut c_void, "io_Device");
    field!(unit, set_unit, IO_UNIT, *mut c_void, "io_Unit (driver private)");
    field!(command, set_command, IO_COMMAND, u16, "io_Command");
    field!(flags, set_flags, IO_FLAGS, u8, "io_Flags");
    field!(error, set_error, IO_ERROR, i8, "io_Error");
    field!(actual, set_actual, IO_ACTUAL, u32, "io_Actual");
    field!(length, set_length, IO_LENGTH, u32, "io_Length");
    field!(data, set_data, IO_DATA, *mut u8, "io_Data");
    field!(offset, set_offset, IO_OFFSET, u32, "io_Offset");

    /// The caller asked for quick I/O.
    #[inline]
    pub fn is_quick(self) -> bool {
        self.flags() & IOF_QUICK != 0
    }

    /// Make the request pending, for a driver that will reply later from
    /// another task: clear IOF_QUICK and io_Error, and mark the message
    /// NT_MESSAGE. WaitIO and CheckIO read the node type, and a request
    /// reused after a DoIO, or straight from OpenDevice, still says
    /// NT_REPLYMSG, so without this it would look finished at once.
    #[inline]
    pub fn mark_pending(self) {
        self.set_flags(self.flags() & !IOF_QUICK);
        self.set_error(0);
        // SAFETY: from_raw's contract; ln_Type is in the Message header.
        unsafe { *(self.0 as *mut u8).add(LN_TYPE) = NT_MESSAGE }
    }

    /// End a request in BeginIO: set io_Error and io_Actual, and reply
    /// unless the caller asked for quick I/O (then it is done in place).
    ///
    /// # Safety
    /// The request is the driver's to end: no other task will end it.
    #[inline]
    pub unsafe fn finish(self, error: i8, actual: u32) {
        self.set_error(error);
        self.set_actual(actual);
        if !self.is_quick() {
            // SAFETY: a live request (from_raw) that is ours to end.
            unsafe { sys::exec_reply_msg(self.0 as *mut Message) };
        }
    }

    /// End a request that was queued (marked pending, so never quick):
    /// set io_Error and io_Actual and reply.
    ///
    /// # Safety
    /// As for `finish`.
    #[inline]
    pub unsafe fn reply(self, error: i8, actual: u32) {
        self.set_error(error);
        self.set_actual(actual);
        // SAFETY: a live request (from_raw) that is ours to end.
        unsafe { sys::exec_reply_msg(self.0 as *mut Message) };
    }
}

/// Answer `NSCMD_DEVICEQUERY` into the request's io_Data: format 0, the
/// type and subtype, and the 0-terminated command list (which must stay
/// valid: a static). Returns the (io_Error, io_Actual) to end it with.
///
/// # Safety
/// io_Data, if it is long enough, points to writable memory of io_Length bytes.
pub unsafe fn answer_device_query(ior: IoReq, device_type: u16, subtype: u16, commands: &'static [u16]) -> (i8, u32) {
    let data = ior.data();
    if data.is_null() || ior.length() < NSDQR_SIZE || commands.last() != Some(&0) {
        return (ioerr::IOERR_BADLENGTH, 0);
    }
    // SAFETY: io_Data is writable for io_Length >= NSDQR_SIZE (16) bytes
    // (checked above; the caller's contract), and these writes end at 16.
    unsafe {
        core::ptr::write_unaligned(data as *mut u32, 0); // DevQueryFormat
        core::ptr::write_unaligned(data.add(4) as *mut u32, NSDQR_SIZE); // SizeAvailable
        core::ptr::write_unaligned(data.add(8) as *mut u16, device_type);
        core::ptr::write_unaligned(data.add(10) as *mut u16, subtype);
        core::ptr::write_unaligned(data.add(12) as *mut *const u16, commands.as_ptr());
    }
    (0, NSDQR_SIZE)
}

// ---------------------------------------------------------------------------
// RequestQueue
// ---------------------------------------------------------------------------

/// The queue proper, without its lock: the logic the tests run on the host.
struct QueueState {
    pending: VecDeque<IoReq>,
    current: Option<IoReq>,
    abort_current: bool,
}

/// What AbortIO found.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Abort {
    /// It was queued and is removed: the caller replies it IOERR_ABORTED.
    Removed,
    /// It is in progress: flagged, and the unit task ends it.
    Flagged,
    /// Not here (already replied, or never queued): nothing to do.
    NotFound,
}

impl QueueState {
    const fn new() -> Self {
        Self { pending: VecDeque::new(), current: None, abort_current: false }
    }
    fn take_next(&mut self) -> Option<IoReq> {
        let next = self.pending.pop_front();
        self.current = next;
        self.abort_current = false;
        next
    }
    fn done(&mut self) {
        self.current = None;
        self.abort_current = false;
    }
    fn abort(&mut self, ior: IoReq) -> Abort {
        if let Some(pos) = self.pending.iter().position(|&p| p == ior) {
            self.pending.remove(pos);
            Abort::Removed
        } else if self.current == Some(ior) {
            self.abort_current = true;
            Abort::Flagged
        } else {
            Abort::NotFound
        }
    }
    fn abort_all(&mut self) -> Vec<IoReq> {
        if self.current.is_some() {
            self.abort_current = true;
        }
        self.pending.drain(..).collect()
    }
}

/// A unit's requests: those waiting for the unit task, and the one in
/// progress, under an exec mutex. BeginIO pushes and the unit task takes;
/// AbortIO and CMD_FLUSH reach both. Every method takes the mutex, so they
/// are for tasks, never interrupts.
///
/// `const fn new()` for a static; [`init`](Self::init) creates the mutex
/// (at init time, where the caller is provably the only user, never lazily
/// on the path it protects) and [`destroy`](Self::destroy) frees it.
pub struct RequestQueue {
    state: UnsafeCell<QueueState>,
    mutex: AtomicPtr<c_void>,
}

// SAFETY: `state` is only touched with `mutex` held (or before init/after
// destroy, when there is one user).
unsafe impl Sync for RequestQueue {}

impl RequestQueue {
    pub const fn new() -> Self {
        Self { state: UnsafeCell::new(QueueState::new()), mutex: AtomicPtr::new(null_mut()) }
    }

    /// Create the mutex.
    ///
    /// # Safety
    /// No other task uses the queue yet.
    pub unsafe fn init(&self) -> Result<()> {
        // Not recursive (exec's default is): the queue never locks twice.
        let tags = [
            TagItem { ti_Tag: sys::ASOMUTEX_RECURSIVE, ti_Data: 0 },
            TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list.
        let m = unsafe { sys::exec_alloc_sys_object(sys::ASOT_MUTEX, tags.as_ptr()) };
        if m.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        self.mutex.store(m, Ordering::Release);
        Ok(())
    }

    /// Free the mutex and the queue's own memory, leaving it as `new()`
    /// left it, so `init` may be called again (a unit that restarts).
    /// Requests still queued are forgotten, not replied: end them first.
    ///
    /// # Safety
    /// No task uses the queue any more.
    pub unsafe fn destroy(&self) {
        let m = self.mutex.swap(null_mut(), Ordering::AcqRel);
        if !m.is_null() {
            // SAFETY: our mutex, unused (the caller's contract).
            unsafe { sys::exec_free_sys_object(sys::ASOT_MUTEX, m as sys::APTR) };
        }
        // SAFETY: no task uses the queue (the caller's contract).
        unsafe { *self.state.get() = QueueState::new() };
    }

    fn with<R>(&self, f: impl FnOnce(&mut QueueState) -> R) -> R {
        let m = self.mutex.load(Ordering::Acquire);
        // SAFETY: the mutex guards `state`; without one (before init) the
        // caller is the only user, as init's contract says.
        unsafe {
            if !m.is_null() {
                sys::exec_mutex_obtain(m as sys::APTR);
            }
            let r = f(&mut *self.state.get());
            if !m.is_null() {
                sys::exec_mutex_release(m as sys::APTR);
            }
            r
        }
    }

    /// Queue a request (marked pending first, see [`IoReq::mark_pending`]).
    pub fn push(&self, ior: IoReq) {
        ior.mark_pending();
        self.with(|q| q.pending.push_back(ior));
    }

    /// The unit task: take the next request and make it current.
    pub fn take_next(&self) -> Option<IoReq> {
        self.with(|q| q.take_next())
    }

    /// The unit task: the current request is finished. Call it BEFORE the
    /// reply, so an AbortIO after this point finds nothing and leaves the
    /// reply alone.
    pub fn done(&self) {
        self.with(|q| q.done())
    }

    /// The unit task, between pieces of the current request: has AbortIO
    /// or a flush asked for it to stop?
    pub fn abort_requested(&self) -> bool {
        self.with(|q| q.abort_current)
    }

    /// AbortIO. On `Removed`, reply the request IOERR_ABORTED.
    pub fn abort(&self, ior: IoReq) -> Abort {
        self.with(|q| q.abort(ior))
    }

    /// CMD_RESET / CMD_FLUSH: flag the current request and take every
    /// queued one, for the caller to reply IOERR_ABORTED.
    pub fn abort_all(&self) -> Vec<IoReq> {
        self.with(|q| q.abort_all())
    }
}

impl Default for RequestQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

/// A unit task's work signal. The task publishes itself once it is ready;
/// until then (and after it has stopped) [`wake`](Self::wake) refuses, so
/// BeginIO can fail a request instead of queueing it for nobody.
pub struct Worker {
    task: AtomicPtr<Task>,
    mask: AtomicU32,
    /// Set to ask the task to stop (see [`Worker::stop_requested`]).
    stop: AtomicBool,
}

impl Worker {
    pub const fn new() -> Self {
        Self { task: AtomicPtr::new(null_mut()), mask: AtomicU32::new(0), stop: AtomicBool::new(false) }
    }

    /// The task, once ready: wake it with `mask`.
    pub fn publish(&self, task: *mut Task, mask: u32) {
        self.task.store(task, Ordering::Release);
        self.mask.store(mask, Ordering::Release);
    }

    /// The task, as it stops: nothing more is to be sent to it.
    pub fn withdraw(&self) {
        self.mask.store(0, Ordering::Release);
        self.task.store(null_mut(), Ordering::Release);
    }

    /// Is the task ready?
    pub fn is_ready(&self) -> bool {
        self.mask.load(Ordering::Acquire) != 0 && !self.task.load(Ordering::Acquire).is_null()
    }

    /// Signal the task. False: it is not ready (or stopping).
    pub fn wake(&self) -> bool {
        let mask = self.mask.load(Ordering::Acquire);
        let task = self.task.load(Ordering::Acquire);
        if mask == 0 || task.is_null() || self.stop.load(Ordering::Acquire) {
            return false;
        }
        // SAFETY: a published task is alive until it withdraws.
        unsafe { sys::exec_signal(task, mask) };
        true
    }

    /// Ask the task to stop and wake it. The task sees it with
    /// [`stop_requested`](Self::stop_requested), ends its work, withdraws
    /// and tells the asker it is gone (the asker's own handshake).
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
        let mask = self.mask.load(Ordering::Acquire);
        let task = self.task.load(Ordering::Acquire);
        if mask != 0 && !task.is_null() {
            // SAFETY: as in wake.
            unsafe { sys::exec_signal(task, mask) };
        }
    }

    /// The task: has someone asked it to stop?
    pub fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(buf: &mut [u8; 48]) -> IoReq {
        // SAFETY: a 48-byte buffer (an IOStdReq) that outlives the test.
        unsafe { IoReq::from_raw(buf.as_mut_ptr() as *mut IORequest) }.unwrap()
    }

    #[test]
    fn fields_land_at_the_sdk_offsets() {
        let mut b = [0u8; 48];
        let r = req(&mut b);
        r.set_command(0x1234);
        r.set_flags(0x56);
        r.set_error(-2);
        r.set_actual(0x0a0b0c0d);
        r.set_length(7);
        r.set_offset(9);
        assert_eq!(u16::from_ne_bytes([b[28], b[29]]), 0x1234);
        assert_eq!(b[30], 0x56);
        assert_eq!(b[31] as i8, -2);
        assert_eq!(u32::from_ne_bytes([b[32], b[33], b[34], b[35]]), 0x0a0b0c0d);
        assert_eq!(u32::from_ne_bytes([b[36], b[37], b[38], b[39]]), 7);
        assert_eq!(u32::from_ne_bytes([b[44], b[45], b[46], b[47]]), 9);
        let r2 = req(&mut b);
        assert_eq!(r2.command(), 0x1234);
        assert_eq!(r2.error(), -2);
    }

    #[test]
    fn mark_pending_clears_quick_and_error_and_sets_nt_message() {
        let mut b = [0u8; 48];
        b[30] = IOF_QUICK | 0x80;
        b[31] = 0xfe;
        b[8] = 7; // NT_REPLYMSG
        let r = req(&mut b);
        r.mark_pending();
        assert_eq!(b[30], 0x80);
        assert_eq!(b[31], 0);
        assert_eq!(b[8], NT_MESSAGE);
    }

    #[test]
    fn device_query_fills_the_result() {
        static CMDS: [u16; 3] = [cmd::CMD_READ, cmd::NSCMD_DEVICEQUERY, 0];
        let mut out = [0u8; 16];
        let mut b = [0u8; 48];
        let r = req(&mut b);
        r.set_data(out.as_mut_ptr());
        r.set_length(16);
        // SAFETY: io_Data is `out`, writable for the io_Length given.
        let (e, n) = unsafe { answer_device_query(r, 5, 1, &CMDS) };
        assert_eq!((e, n), (0, 16));
        assert_eq!(u32::from_ne_bytes([out[4], out[5], out[6], out[7]]), 16);
        assert_eq!(u16::from_ne_bytes([out[8], out[9]]), 5);
        assert_eq!(u16::from_ne_bytes([out[10], out[11]]), 1);
        r.set_length(15);
        // SAFETY: as above.
        assert_eq!(unsafe { answer_device_query(r, 5, 1, &CMDS) }.0, ioerr::IOERR_BADLENGTH);
    }

    #[test]
    fn queue_order_abort_and_flush() {
        let mut bufs = [[0u8; 48]; 4];
        let [a, b, c, d] = {
            let p: Vec<IoReq> = bufs.iter_mut().map(|x| req(x)).collect();
            [p[0], p[1], p[2], p[3]]
        };
        let mut q = QueueState::new();
        q.pending.extend([a, b, c]);
        assert_eq!(q.take_next(), Some(a));
        assert_eq!(q.abort(b), Abort::Removed);
        assert_eq!(q.abort(a), Abort::Flagged);
        assert!(q.abort_current);
        assert_eq!(q.abort(d), Abort::NotFound);
        q.done();
        assert!(!q.abort_current && q.current.is_none());
        assert_eq!(q.abort(a), Abort::NotFound); // replied: AbortIO leaves it alone
        assert_eq!(q.take_next(), Some(c));
        q.pending.push_back(d);
        assert_eq!(q.abort_all(), alloc::vec![d]);
        assert!(q.abort_current);
        assert!(q.pending.is_empty());
        assert_eq!(q.take_next(), None);
        assert!(!q.abort_current);
    }

    #[test]
    fn worker_refuses_until_published_and_after_stop() {
        let w = Worker::new();
        assert!(!w.is_ready());
        assert!(!w.wake());
        assert!(!w.stop_requested());
        w.withdraw();
        assert!(!w.is_ready());
    }
}
