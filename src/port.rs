//! A message port of the calling task (AllocSysObject ASOT_PORT), freed on
//! drop.
//!
//! [`AmigaMsgPort`] is the untyped port: a device I/O reply port, or a
//! port another program finds by name ([`AmigaMsgPort::public`]). Its
//! signal bit belongs to the task that made it, so it is neither `Send`
//! nor `Sync`. Typed messages between tasks of one program are
//! [`crate::msg`].

use core::marker::PhantomData;
use core::ptr::null_mut;

use crate::error::{AmigaError, Result};
use crate::sys::{self, Message, MsgPort, TagItem, APTR, TAG_DONE};

/// An exec message port of the calling task.
pub struct AmigaMsgPort {
    ptr: *mut MsgPort,
    _t: crate::track::Tracked,
    _not_send: PhantomData<*mut ()>,
}

/// Free a registered port (the panic path; see `crate::track`).
unsafe fn release_port(p: usize, _: usize) {
    // SAFETY: the registry calls this once, in the task that made it. What
    // is still queued is replied first (not what came back as a reply), so
    // no sender waits for ever; then the port is freed.
    unsafe {
        loop {
            let m = sys::exec_get_msg(p as *mut MsgPort);
            if m.is_null() {
                break;
            }
            if *((m as *const u8).add(8)) != sys::NT_REPLYMSG {
                sys::exec_reply_msg(m);
            }
        }
        sys::exec_free_sys_object(sys::ASOT_PORT, p as APTR)
    }
}

impl AmigaMsgPort {
    /// A private port with a signal bit of its own.
    #[track_caller]
    pub fn new() -> Result<Self> {
        Self::with_tags(&[TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// A public port named `name` (NUL-terminated; exec copies it), found
    /// by other programs with FindPort. It is removed from the public list
    /// when dropped.
    #[track_caller]
    pub fn public(name: &[u8]) -> Result<Self> {
        if name.last() != Some(&0) {
            return Err(AmigaError::NotNulTerminated);
        }
        Self::with_tags(&[
            TagItem { ti_Tag: sys::ASOPORT_NAME, ti_Data: name.as_ptr() as u32 },
            TagItem { ti_Tag: sys::ASOPORT_COPYNAME, ti_Data: 1 },
            TagItem { ti_Tag: sys::ASOPORT_PUBLIC, ti_Data: 1 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ])
    }

    #[track_caller]
    fn with_tags(tags: &[TagItem]) -> Result<Self> {
        // SAFETY: a TAG_DONE-ended tag list; a name in it is copied.
        let ptr = unsafe { sys::exec_alloc_sys_object(sys::ASOT_PORT, tags.as_ptr()) } as *mut MsgPort;
        if ptr.is_null() {
            Err(AmigaError::AllocationFailed)
        } else {
            Ok(Self { ptr, _t: crate::track::register_bound("message port", ptr as usize, 0, release_port), _not_send: PhantomData })
        }
    }

    /// Get the raw `MsgPort` pointer.
    #[inline]
    pub fn as_ptr(&self) -> *mut MsgPort {
        self.ptr
    }

    /// The Exec signal mask of this port (`1 << mp_SigBit`).
    #[inline]
    pub fn signal_mask(&self) -> u32 {
        // SAFETY: mp_SigBit is the byte at offset 15 of a live MsgPort
        // (checked against the SDK, tests/layout/exec.c).
        unsafe { 1u32 << *((self.ptr as *const u8).add(15)) }
    }

    /// The next message, if one is queued (GetMsg). The caller owns it
    /// and must reply or free it as its sender's protocol says.
    pub fn get_raw(&self) -> Option<*mut Message> {
        // SAFETY: our live port.
        let m = unsafe { sys::exec_get_msg(self.ptr) };
        if m.is_null() { None } else { Some(m) }
    }

    /// Wait until a message is queued (WaitPort); it stays queued.
    pub fn wait(&self) {
        // SAFETY: our live port, waited for by its own task.
        unsafe { sys::exec_wait_port(self.ptr) };
    }
}

impl Drop for AmigaMsgPort {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: our port, freed once by the task that owns its signal
            // (the type is not Send); a public one is removed first by exec.
            unsafe { sys::exec_free_sys_object(sys::ASOT_PORT, self.ptr as APTR) }
            self.ptr = null_mut();
        }
    }
}
