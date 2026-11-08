//! Using an exec device: open it, run requests, close it.
//!
//! [`Device::open`] makes a reply port and an I/O request (AllocSysObject
//! ASOT_IOREQUEST, of the size the device's request type needs), and opens
//! the device with it; dropping the device closes it. The device's own
//! request is [`Device::request`]; more requests on the same open device
//! come from [`Device::new_request`] (ASOIOR_Duplicate), for I/O in
//! parallel.
//!
//! A request runs synchronously ([`Request::do_io`], DoIO) or is sent
//! ([`Request::send`], SendIO) and returns an [`InFlight`], which borrows
//! the request and any data buffer until it completes: the CPU cannot touch
//! the buffer while the device owns it. Dropping an `InFlight` aborts the
//! request and waits for it (AbortIO + WaitIO), so a request is never left
//! with the device.
//!
//! A request with no data buffer (a timer's) can also be started without
//! a borrow ([`Request::start`]): the request then keeps the pending state
//! itself, and anything that touches it first cancels it (AbortIO +
//! WaitIO), as dropping the request or the device does. A device whose
//! users wait on several things at once gives each kind of request its
//! own reply port ([`Device::new_request_on`]): a WaitIO on one port
//! never takes another request's signal.
//!
//! The reply port's signal belongs to the task that opened the device: a
//! `Device` and its requests are not `Send`.

use core::marker::PhantomData;
use core::ptr::null_mut;

use crate::device::IoReq;
use crate::error::{AmigaError, Result};
use crate::port::AmigaMsgPort;
use crate::sys::{self, IORequest, TagItem, APTR, TAG_DONE};

/// struct IORequest is 32 bytes, IOStdReq 48 (tests/layout/exec.c).
pub const IOREQUEST_SIZE: u32 = 32;
pub const IOSTDREQ_SIZE: u32 = 48;

/// An open device.
pub struct Device {
    _t: crate::track::Tracked,
    port: AmigaMsgPort,
    req: Request<'static>,
    size: u32,
}

impl Device {
    /// Open unit `unit` of `name` (NUL-terminated, e.g. `b"timer.device\0"`)
    /// with a request of `size` bytes (at least [`IOREQUEST_SIZE`]; the
    /// device's own request type, e.g. 40 for a timerequest).
    /// `Err(IoErr(e))` when OpenDevice refuses with error `e`.
    pub fn open(name: &[u8], unit: u32, flags: u32, size: u32) -> Result<Self> {
        // SAFETY: no data is handed to OpenDevice.
        unsafe { Self::open_with_data(name, unit, flags, size, null_mut(), 0) }
    }

    /// Open with io_Data and io_Length set first (console.device reads a
    /// window taglist there); io_Actual holds what OpenDevice returned.
    ///
    /// # Safety
    /// `data` is NULL or valid for what the device does with it during
    /// OpenDevice, `len` bytes; `size` is an IOStdReq or larger when
    /// `data` is not NULL.
    #[track_caller]
    pub(crate) unsafe fn open_with_data(name: &[u8], unit: u32, flags: u32, size: u32, data: *mut u8, len: u32) -> Result<Self> {
        if name.last() != Some(&0) {
            return Err(AmigaError::NotNulTerminated);
        }
        if size < IOREQUEST_SIZE {
            return Err(AmigaError::Device("request smaller than an IORequest"));
        }
        let port = AmigaMsgPort::new()?;
        if !data.is_null() && size < IOSTDREQ_SIZE {
            return Err(AmigaError::Device("data needs an IOStdReq-sized request"));
        }
        let raw = alloc_request(&[
            TagItem { ti_Tag: sys::ASOIOR_SIZE, ti_Data: size },
            TagItem { ti_Tag: sys::ASOIOR_REPLYPORT, ti_Data: port.as_ptr() as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ])?;
        if !data.is_null() {
            // SAFETY: our fresh request, at least an IOStdReq (checked).
            if let Some(io) = unsafe { IoReq::from_raw(raw as *mut IORequest) } {
                io.set_data(data);
                io.set_length(len);
            }
        }
        // SAFETY: a NUL-terminated name and our fresh request (its data the
        // caller's).
        let err = unsafe { sys::exec_open_device(name.as_ptr(), unit, raw as *mut IORequest, flags) };
        if err != 0 {
            // SAFETY: the request was never opened or shared.
            unsafe { sys::exec_free_sys_object(sys::ASOT_IOREQUEST, raw as APTR) };
            return Err(AmigaError::IoErr(err as i8));
        }
        Ok(Self { _t: crate::track::register("device", raw as usize, 0, release_device), port, req: Request { raw, size, owned: false, pending: false, _dev: PhantomData, _not_send: PhantomData }, size })
    }

    /// The device's base (io_Device), for GetInterface on a device that
    /// has one (timer.device's ITimer). Valid while the device is open.
    pub fn base(&self) -> *mut sys::Library {
        // SAFETY: our open request: io_Device at offset 20
        // (tests/layout/exec.c).
        unsafe { core::ptr::read(self.req.raw.add(20) as *const *mut sys::Library) }
    }

    /// The request the device was opened with.
    pub fn request(&mut self) -> &mut Request<'static> {
        &mut self.req
    }

    /// The request the device was opened with, to read.
    pub fn request_ref(&self) -> &Request<'static> {
        &self.req
    }

    /// Another request on this open device (a copy of the device's own,
    /// with the same reply port), freed on drop.
    pub fn new_request(&self) -> Result<Request<'_>> {
        let raw = alloc_request(&[
            TagItem { ti_Tag: sys::ASOIOR_DUPLICATE, ti_Data: self.req.raw as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ])?;
        Ok(Request { raw, size: self.size, owned: true, pending: false, _dev: PhantomData, _not_send: PhantomData })
    }

    /// Another request on this open device that replies to `port`
    /// instead of the device's own port.
    pub fn new_request_on<'a>(&'a self, port: &'a AmigaMsgPort) -> Result<Request<'a>> {
        let raw = alloc_request(&[
            TagItem { ti_Tag: sys::ASOIOR_DUPLICATE, ti_Data: self.req.raw as u32 },
            TagItem { ti_Tag: sys::ASOIOR_REPLYPORT, ti_Data: port.as_ptr() as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ])?;
        Ok(Request { raw, size: self.size, owned: true, pending: false, _dev: PhantomData, _not_send: PhantomData })
    }

    /// The reply port's signal mask (to wait for several requests at once).
    pub fn signal_mask(&self) -> u32 {
        self.port.signal_mask()
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.req.cancel_pending();
        // SAFETY: no request is in flight (an InFlight borrows its request,
        // which borrows the device), so the device can be closed; then the
        // request and port are freed (the port by its own drop).
        unsafe {
            sys::exec_close_device(self.req.raw as *mut IORequest);
            sys::exec_free_sys_object(sys::ASOT_IOREQUEST, self.req.raw as APTR);
        }
        self.req.raw = null_mut();
    }
}

/// Close a registered device: its request aborted if it is in flight, the
/// device closed, the request freed (the panic path; see `crate::track`).
/// Duplicate requests of the device are not tracked: one in flight when its
/// task panics is left to the device.
unsafe fn release_device(req: usize, _: usize) {
    let r = req as *mut IORequest;
    // SAFETY: the registry calls this once, for the device's own request,
    // opened and not freed; CheckIO is NULL only while it is in flight.
    unsafe {
        if sys::exec_check_io(r).is_null() {
            sys::exec_abort_io(r);
            sys::exec_wait_io(r);
        }
        sys::exec_close_device(r);
        sys::exec_free_sys_object(sys::ASOT_IOREQUEST, r as APTR);
    }
}

fn alloc_request(tags: &[TagItem]) -> Result<*mut u8> {
    // SAFETY: a TAG_DONE-ended tag list (a duplicated request is live).
    let raw = unsafe { sys::exec_alloc_sys_object(sys::ASOT_IOREQUEST, tags.as_ptr()) } as *mut u8;
    if raw.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(raw) }
}

/// An I/O request on an open [`Device`]; `'d` borrows the device (the
/// device's own request has `'static` and lives in the device).
pub struct Request<'d> {
    raw: *mut u8,
    size: u32,
    owned: bool,
    /// Started with `start` and not yet collected.
    pending: bool,
    _dev: PhantomData<&'d Device>,
    _not_send: PhantomData<*mut ()>,
}

impl<'d> Request<'d> {
    fn io(&self) -> IoReq {
        // SAFETY: a live request of at least IORequest size; IoReq's
        // accessors past 32 bytes are only used below when size >= 48.
        unsafe { IoReq::from_raw(self.raw as *mut IORequest) }.expect("Request: non-null")
    }

    /// The request's size in bytes.
    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn command(&self) -> u16 {
        self.io().command()
    }

    pub fn set_command(&mut self, cmd: u16) {
        self.cancel_pending();
        self.io().set_command(cmd)
    }

    pub fn flags(&self) -> u8 {
        self.io().flags()
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.cancel_pending();
        self.io().set_flags(flags)
    }

    /// io_Error of the last completed I/O.
    pub fn error(&self) -> i8 {
        self.io().error()
    }

    /// io_Actual (IOStdReq-sized requests only, else 0).
    pub fn actual(&self) -> u32 {
        if self.size >= IOSTDREQ_SIZE { self.io().actual() } else { 0 }
    }

    /// The request's bytes after the IORequest header (32..size): the
    /// device-specific fields, such as a timerequest's time value.
    pub fn extra(&self) -> &[u8] {
        // SAFETY: our request of `size` bytes; not in flight (an InFlight
        // would hold `&mut self`).
        unsafe { core::slice::from_raw_parts(self.raw.add(32), (self.size - 32) as usize) }
    }

    /// As [`extra`](Self::extra), to write (a started request is
    /// cancelled first).
    pub fn extra_mut(&mut self) -> &mut [u8] {
        self.cancel_pending();
        // SAFETY: as above, borrowed mutably; not started.
        unsafe { core::slice::from_raw_parts_mut(self.raw.add(32), (self.size - 32) as usize) }
    }

    fn result(&self) -> Result<()> {
        match self.error() {
            0 => Ok(()),
            e => Err(AmigaError::IoErr(e)),
        }
    }

    /// Run the request as it is set up (DoIO): `Err(IoErr(io_Error))` when
    /// it fails.
    pub fn do_io(&mut self) -> Result<()> {
        self.cancel_pending();
        // SAFETY: an initialised request of the open device, not in flight.
        unsafe { sys::exec_do_io(self.raw as *mut IORequest) };
        self.result()
    }

    fn set_data(&mut self, cmd: u16, data: *mut u8, len: usize, offset: u32) -> Result<()> {
        self.cancel_pending();
        if self.size < IOSTDREQ_SIZE {
            return Err(AmigaError::Device("data commands need an IOStdReq-sized request"));
        }
        let io = self.io();
        io.set_command(cmd);
        io.set_data(data);
        io.set_length(len as u32);
        io.set_offset(offset);
        Ok(())
    }

    /// Run command `cmd` on `data` at `offset` (io_Data, io_Length,
    /// io_Offset of an IOStdReq) and return io_Actual. io_Data is cleared
    /// afterwards.
    pub fn do_io_data(&mut self, cmd: u16, data: &mut [u8], offset: u32) -> Result<u32> {
        self.set_data(cmd, data.as_mut_ptr(), data.len(), offset)?;
        let r = self.do_io();
        let actual = self.actual();
        self.io().set_data(null_mut());
        r.map(|_| actual)
    }

    /// Run command `cmd` on `data` that the device only reads (CMD_WRITE,
    /// a taglist of addresses to fill) and return io_Actual.
    ///
    /// # Safety
    /// The command never writes into `data` itself.
    #[cfg_attr(not(feature = "console"), allow(dead_code))]
    pub(crate) unsafe fn do_io_const(&mut self, cmd: u16, data: &[u8], offset: u32) -> Result<u32> {
        self.set_data(cmd, data.as_ptr() as *mut u8, data.len(), offset)?;
        let r = self.do_io();
        let actual = self.actual();
        self.io().set_data(null_mut());
        r.map(|_| actual)
    }

    /// Send the request as it is set up (SendIO).
    pub fn send(&mut self) -> InFlight<'_, 'd> {
        self.cancel_pending();
        // SAFETY: an initialised request, not in flight.
        unsafe { sys::exec_send_io(self.raw as *mut IORequest) };
        InFlight { req: self, _data: PhantomData, done: false }
    }

    /// Send command `cmd` on `data` at `offset`: the buffer stays borrowed
    /// until the request completes.
    pub fn send_data<'r>(&'r mut self, cmd: u16, data: &'r mut [u8], offset: u32) -> Result<InFlight<'r, 'd>> {
        self.set_data(cmd, data.as_mut_ptr(), data.len(), offset)?;
        // SAFETY: as in send; `data` is borrowed by the InFlight.
        unsafe { sys::exec_send_io(self.raw as *mut IORequest) };
        Ok(InFlight { req: self, _data: PhantomData, done: false })
    }
}

impl Request<'_> {
    /// Send the request as it is set up (SendIO) without borrowing it: for
    /// commands with no data buffer. A request already started is
    /// cancelled first.
    pub fn start(&mut self) {
        self.cancel_pending();
        // SAFETY: an initialised request, not in flight.
        unsafe { sys::exec_send_io(self.raw as *mut IORequest) };
        self.pending = true;
    }

    /// Started and not yet collected.
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// Whether a started request has completed (CheckIO); true when none
    /// is pending.
    pub fn check(&self) -> bool {
        // SAFETY: a request of ours that was started.
        !self.pending || unsafe { !sys::exec_check_io(self.raw as *mut IORequest).is_null() }
    }

    /// Collect a started request (WaitIO), with its result.
    pub fn finish(&mut self) -> Result<()> {
        if self.pending {
            // SAFETY: a request of ours that was started.
            unsafe { sys::exec_wait_io(self.raw as *mut IORequest) };
            self.pending = false;
        }
        self.result()
    }

    /// Abort a started request and collect it (AbortIO + WaitIO).
    pub fn cancel_pending(&mut self) {
        if self.pending {
            // SAFETY: a request of ours that was started: abort, then wait,
            // so the device lets go of it.
            unsafe {
                sys::exec_abort_io(self.raw as *mut IORequest);
                sys::exec_wait_io(self.raw as *mut IORequest);
            }
            self.pending = false;
        }
    }
}

impl Drop for Request<'_> {
    fn drop(&mut self) {
        self.cancel_pending();
        if self.owned && !self.raw.is_null() {
            // SAFETY: a duplicate we allocated, not in flight (InFlight
            // borrows it).
            unsafe { sys::exec_free_sys_object(sys::ASOT_IOREQUEST, self.raw as APTR) };
        }
    }
}

/// A request the device owns until it completes; `'r` also covers the data
/// buffer of [`Request::send_data`].
pub struct InFlight<'r, 'd> {
    req: &'r mut Request<'d>,
    _data: PhantomData<&'r mut [u8]>,
    done: bool,
}

impl InFlight<'_, '_> {
    /// Has it completed (CheckIO)?
    pub fn is_done(&self) -> bool {
        // SAFETY: a request of ours that was sent.
        unsafe { !sys::exec_check_io(self.req.raw as *mut IORequest).is_null() }
    }

    /// Wait for it (WaitIO): `Err(IoErr(io_Error))` when it failed.
    pub fn wait(mut self) -> Result<()> {
        self.finish();
        self.req.result()
    }

    /// Ask the device to abort it (AbortIO), then wait for it. A request
    /// that had already completed keeps its result; an aborted one has
    /// IOERR_ABORTED (-2).
    pub fn abort(mut self) -> Result<()> {
        // SAFETY: a request of ours that was sent.
        unsafe { sys::exec_abort_io(self.req.raw as *mut IORequest) };
        self.finish();
        self.req.result()
    }

    /// io_Actual once done (waits).
    pub fn wait_actual(mut self) -> Result<u32> {
        self.finish();
        self.req.result().map(|_| self.req.actual())
    }

    fn finish(&mut self) {
        if !self.done {
            // SAFETY: a request of ours that was sent; WaitIO removes the
            // reply from the port.
            unsafe { sys::exec_wait_io(self.req.raw as *mut IORequest) };
            self.done = true;
            if self.req.size >= IOSTDREQ_SIZE {
                self.req.io().set_data(null_mut());
            }
        }
    }
}

impl Drop for InFlight<'_, '_> {
    fn drop(&mut self) {
        if !self.done {
            // SAFETY: a request of ours that was sent: abort, then wait, so
            // the device lets go of it and its buffer.
            unsafe { sys::exec_abort_io(self.req.raw as *mut IORequest) };
            self.finish();
        }
    }
}
