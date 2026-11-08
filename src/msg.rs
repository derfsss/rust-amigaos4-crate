//! Typed messages between the tasks of a program, over exec ports.
//!
//! - [`Port<T>`]: a port of the calling task that receives `Message<T>`s.
//!   It is not `Send`: its signal bit belongs to the task that made it.
//! - [`Sender<T>`]: a handle to a port, `Send` and `Clone`, for any task.
//!   Sending to a port that has been dropped gives the message back.
//! - [`Message<T>`]: a `T` in an exec message (AllocSysObject ASOT_MESSAGE,
//!   MEMF_SHARED), optionally with a reply port.
//! - [`Received<T>`]: a message taken from a port. Replying sends it back to
//!   its reply port (ReplyMsg); dropping a received message that wants a
//!   reply replies too, so a sender always gets its message back. A
//!   message that came back as a reply, or has no reply port, is freed.
//!
//! The port's state (open or closed) sits behind an exec mutex that every
//! PutMsg and ReplyMsg holds, so nothing is sent to a port whose owner has
//! dropped it (its signal bit would be gone, its task perhaps too).
//! Dropping a port replies to the messages still queued in it.

use alloc::sync::Arc;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull};

use crate::error::{AmigaError, Result};
use crate::signal::{current_task, Signal};
use crate::sync::Mutex;
use crate::sys::{self, MsgPort, TagItem, Task, APTR, TAG_DONE};

/// exec's struct Message: 20 bytes (tests/layout/exec.c).
const MSG_HEADER: usize = 20;
/// ln_Type's offset in the header.
const LN_TYPE: usize = 8;

/// The shared part of a port: the exec port and whether it is open.
struct PortInner {
    port: *mut MsgPort,
    open: Mutex<bool>,
}

// SAFETY: the port pointer is only used for PutMsg/ReplyMsg under `open`'s
// lock while it is open, and freed once, by the last owner of the Arc.
unsafe impl Send for PortInner {}
// SAFETY: as above: every use goes through the mutex.
unsafe impl Sync for PortInner {}

impl Drop for PortInner {
    fn drop(&mut self) {
        // SAFETY: the last reference; the port was made without a signal of
        // its own (ASOPORT_AllocSig FALSE), so freeing it frees no signal
        // of another task.
        unsafe { sys::exec_free_sys_object(sys::ASOT_PORT, self.port as APTR) }
    }
}

/// A port of the calling task receiving `Message<T>`s.
pub struct Port<T: Send + 'static> {
    inner: Arc<PortInner>,
    sig: Signal,
    owner: *mut Task,
    _t: PhantomData<fn(T) -> T>,
}

/// A handle another task sends `Message<T>`s through.
pub struct Sender<T: Send + 'static> {
    inner: Arc<PortInner>,
    _t: PhantomData<fn(T) -> T>,
}

impl<T: Send + 'static> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), _t: PhantomData }
    }
}

impl<T: Send + 'static> Port<T> {
    /// A private port with a new signal bit of the calling task.
    pub fn new() -> Result<Self> {
        let sig = Signal::alloc()?;
        let me = current_task();
        let tags = [
            TagItem { ti_Tag: sys::ASOPORT_ALLOCSIG, ti_Data: 0 },
            TagItem { ti_Tag: sys::ASOPORT_SIGNAL, ti_Data: sig.bit() as u32 },
            TagItem { ti_Tag: sys::ASOPORT_TARGET, ti_Data: me as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list naming our own bit and task.
        let port = unsafe { sys::exec_alloc_sys_object(sys::ASOT_PORT, tags.as_ptr()) } as *mut MsgPort;
        if port.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        let open = match Mutex::new(true) {
            Ok(m) => m,
            Err(e) => {
                // SAFETY: the port was never shared.
                unsafe { sys::exec_free_sys_object(sys::ASOT_PORT, port as APTR) };
                return Err(e);
            }
        };
        Ok(Self { inner: Arc::new(PortInner { port, open }), sig, owner: me, _t: PhantomData })
    }

    /// A handle for sending to this port from any task.
    pub fn sender(&self) -> Sender<T> {
        Sender { inner: self.inner.clone(), _t: PhantomData }
    }

    /// The port's signal mask, to wait for it together with other signals.
    pub fn signal_mask(&self) -> u32 {
        self.sig.mask()
    }

    fn check_owner(&self) -> Result<()> {
        if current_task() == self.owner { Ok(()) } else { Err(AmigaError::WrongTask) }
    }

    /// The next message, if one is queued.
    pub fn try_recv(&self) -> Result<Option<Received<T>>> {
        self.check_owner()?;
        // SAFETY: our live port.
        let m = unsafe { sys::exec_get_msg(self.inner.port) };
        Ok(NonNull::new(m as *mut u8).map(Received::new))
    }

    /// Wait for the next message.
    pub fn recv(&self) -> Result<Received<T>> {
        loop {
            if let Some(m) = self.try_recv()? {
                return Ok(m);
            }
            self.sig.wait();
        }
    }

    /// Wait for the next message at most `ms` milliseconds: `Ok(None)` when
    /// none came, `Err(Cancelled)` on Ctrl-C (see `timer::Waiter`).
    #[cfg(feature = "timer")]
    pub fn recv_timeout(&self, waiter: &mut crate::timer::Waiter, ms: u32) -> Result<Option<Received<T>>> {
        let t0 = crate::timer::get_up_time().as_micros();
        loop {
            if let Some(m) = self.try_recv()? {
                return Ok(Some(m));
            }
            let spent = (crate::timer::get_up_time().as_micros().saturating_sub(t0) / 1000) as u32;
            if spent >= ms {
                return Ok(None);
            }
            if waiter.wait_ms(self.sig.mask(), ms - spent)? == crate::timer::Waited::TimedOut {
                return self.try_recv();
            }
        }
    }

    /// Wait for the next message or one of `signals`: `Ok(Err(signals that
    /// arrived))` when they came first.
    pub fn recv_or(&self, signals: u32) -> Result<core::result::Result<Received<T>, u32>> {
        loop {
            if let Some(m) = self.try_recv()? {
                return Ok(Ok(m));
            }
            let got = self.sig.wait_or(signals);
            if got & signals != 0 {
                return Ok(Err(got & signals));
            }
        }
    }
}

impl<T: Send + 'static> Drop for Port<T> {
    fn drop(&mut self) {
        // Close it under the lock: no PutMsg or ReplyMsg reaches it after.
        if let Ok(mut open) = self.inner.open.lock() {
            *open = false;
        }
        // The messages still queued: reply them (or free replies) through
        // Received's drop. GetMsg needs no lock: nothing is added any more.
        loop {
            // SAFETY: our port, still allocated (the Arc holds it).
            let m = unsafe { sys::exec_get_msg(self.inner.port) };
            match NonNull::new(m as *mut u8) {
                Some(raw) => drop(Received::<T>::new(raw)),
                None => break,
            }
        }
        // `sig` frees the bit now (on the owner: Port is not Send); the
        // exec port itself is freed with the last Sender.
    }
}

impl<T: Send + 'static> Sender<T> {
    /// Queue `msg` at the port (PutMsg). The message comes back as `Err`
    /// when the port's owner has dropped it.
    pub fn send(&self, msg: Message<T>) -> core::result::Result<(), Message<T>> {
        let Ok(open) = self.inner.open.lock() else { return Err(msg) };
        if !*open {
            return Err(msg);
        }
        // SAFETY: an open port; the message is ours and handed over.
        unsafe { sys::exec_put_msg(self.inner.port, msg.raw.as_ptr() as *mut sys::Message) };
        core::mem::forget(msg);
        drop(open);
        Ok(())
    }
}

/// What a message carries after the exec header.
struct Payload<T: Send + 'static> {
    value: T,
    reply: Option<Sender<T>>,
}

/// A `T` in an exec message.
pub struct Message<T: Send + 'static> {
    raw: NonNull<u8>,
    _t: PhantomData<Payload<T>>,
}

// SAFETY: a message owns its T and is handed between tasks by exec.
unsafe impl<T: Send + 'static> Send for Message<T> {}

impl<T: Send + 'static> Message<T> {
    fn payload_offset() -> usize {
        let a = align_of::<Payload<T>>();
        (MSG_HEADER + a - 1) & !(a - 1)
    }

    fn alloc(value: T, reply: Option<Sender<T>>) -> Result<Self> {
        // exec gives at least 8-byte aligned memory.
        assert!(align_of::<Payload<T>>() <= 8, "Message<T>: T is aligned to more than 8 bytes");
        let size = Self::payload_offset() + size_of::<Payload<T>>();
        let rport = reply.as_ref().map_or(ptr::null_mut(), |s| s.inner.port);
        let tags = [
            TagItem { ti_Tag: sys::ASOMSG_SIZE, ti_Data: size as u32 },
            TagItem { ti_Tag: sys::ASOMSG_REPLYPORT, ti_Data: rport as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list.
        let raw = unsafe { sys::exec_alloc_sys_object(sys::ASOT_MESSAGE, tags.as_ptr()) } as *mut u8;
        let raw = NonNull::new(raw).ok_or(AmigaError::AllocationFailed)?;
        let m = Self { raw, _t: PhantomData };
        // SAFETY: fresh memory of `size` bytes; the payload's place is
        // inside it and aligned (exec's 8 and the rounded offset).
        unsafe { m.payload_ptr().write(Payload { value, reply }) };
        Ok(m)
    }

    /// A message with no reply port: the receiver frees it.
    pub fn new(value: T) -> Result<Self> {
        Self::alloc(value, None)
    }

    /// A message that the receiver replies to `reply` (a port of this
    /// task, which then receives it back).
    pub fn with_reply(value: T, reply: &Port<T>) -> Result<Self> {
        Self::alloc(value, Some(reply.sender()))
    }

    fn payload_ptr(&self) -> *mut Payload<T> {
        // SAFETY: the offset is inside the allocation.
        unsafe { self.raw.as_ptr().add(Self::payload_offset()) as *mut Payload<T> }
    }

    /// The value, freeing the message.
    pub fn into_inner(self) -> T {
        let me = core::mem::ManuallyDrop::new(self);
        // SAFETY: the payload is initialised; it is read out once and the
        // reply handle dropped, then the memory freed and never used again.
        unsafe {
            let p = me.payload_ptr().read();
            sys::exec_free_sys_object(sys::ASOT_MESSAGE, me.raw.as_ptr() as APTR);
            p.value
        }
    }

    fn node_type(&self) -> u8 {
        // SAFETY: ln_Type of the exec header (offset 8).
        unsafe { ptr::read_volatile(self.raw.as_ptr().add(LN_TYPE)) }
    }
}

impl<T: Send + 'static> Deref for Message<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: an initialised payload, owned by `self`.
        unsafe { &(*self.payload_ptr()).value }
    }
}

impl<T: Send + 'static> DerefMut for Message<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as above, borrowed mutably through `&mut self`.
        unsafe { &mut (*self.payload_ptr()).value }
    }
}

impl<T: Send + 'static> Drop for Message<T> {
    fn drop(&mut self) {
        // SAFETY: the payload is initialised and dropped once; the memory is
        // our ASOT_MESSAGE object, freed once.
        unsafe {
            ptr::drop_in_place(self.payload_ptr());
            sys::exec_free_sys_object(sys::ASOT_MESSAGE, self.raw.as_ptr() as APTR);
        }
    }
}

/// A message taken from a [`Port`].
pub struct Received<T: Send + 'static> {
    /// Some until replied or taken out.
    msg: Option<Message<T>>,
}

impl<T: Send + 'static> Received<T> {
    fn new(raw: NonNull<u8>) -> Self {
        Self { msg: Some(Message { raw, _t: PhantomData }) }
    }

    fn m(&self) -> &Message<T> {
        self.msg.as_ref().expect("Received: message present until consumed")
    }

    /// True when this is a message of ours coming back as a reply.
    pub fn is_reply(&self) -> bool {
        self.m().node_type() == sys::NT_REPLYMSG
    }

    /// Does its sender want it back?
    pub fn wants_reply(&self) -> bool {
        wants_reply(self.m())
    }

    /// Send it back to its reply port. A message that wants no reply (or
    /// whose reply port has been dropped) is freed instead.
    pub fn reply(mut self) {
        if let Some(m) = self.msg.take() {
            reply_or_free(m);
        }
    }

    /// Keep it: the message, no longer replied when dropped.
    pub fn into_message(mut self) -> Message<T> {
        self.msg.take().expect("Received: message present until consumed")
    }
}

fn wants_reply<T: Send + 'static>(m: &Message<T>) -> bool {
    // SAFETY: the payload is initialised.
    m.node_type() != sys::NT_REPLYMSG && unsafe { (*m.payload_ptr()).reply.is_some() }
}

/// ReplyMsg `m` to its reply port if it wants a reply and the port is open;
/// otherwise free it.
fn reply_or_free<T: Send + 'static>(m: Message<T>) {
    if !wants_reply(&m) {
        return; // dropped: freed
    }
    // SAFETY: the payload is initialised; the clone keeps the reply port's
    // memory alive while it is used.
    let Some(reply) = (unsafe { (*m.payload_ptr()).reply.clone() }) else { return };
    let Ok(open) = reply.inner.open.lock() else { return };
    if !*open {
        return; // dropped (after the lock is released): freed
    }
    // SAFETY: an open reply port, the one in mn_ReplyPort; the message is
    // handed back to it.
    unsafe { sys::exec_reply_msg(m.raw.as_ptr() as *mut sys::Message) };
    core::mem::forget(m);
    drop(open);
}

impl<T: Send + 'static> Deref for Received<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.m()
    }
}

impl<T: Send + 'static> DerefMut for Received<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.msg.as_mut().expect("Received: message present until consumed")
    }
}

impl<T: Send + 'static> Drop for Received<T> {
    fn drop(&mut self) {
        if let Some(m) = self.msg.take() {
            reply_or_free(m);
        }
    }
}

