//! File and directory change notification (StartNotify / EndNotify),
//! signal method: a [`Watcher`] owns a signal bit of the calling Process
//! that DOS sets when the watched object changes.
//!
//! Not for ENV: or ENVARC: (the autodoc: use variable notification, which
//! is [`VarWatcher`]).
//! A file's change is reported when it is closed.

use alloc::vec::Vec;

use super::{last_error, process};
use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::signal::Signal;
use crate::sys::{self, TagItem, APTR, TAG_DONE};

/// End and free a registered notification (the panic path; see
/// `crate::track`).
unsafe fn release_watcher(req: usize, _: usize) {
    if let Ok(i) = process() {
        // SAFETY: the registry calls this once, in the Process that started
        // it, for a request still started.
        unsafe {
            ((*i).EndNotify)(i, req as *mut sys::NotifyRequest);
            ((*i).FreeDosObject)(i, sys::DOS_NOTIFYREQUEST, req as APTR);
        }
    }
}

/// Remove a registered variable notification: its hook is in the
/// program's code.
unsafe fn release_var_watcher(hook: usize, _: usize) {
    if let Ok(i) = process() {
        // SAFETY: as release_watcher; the hook stays allocated (leaked).
        unsafe { ((*i).NotifyVar)(i, core::ptr::null(), hook as *const sys::Hook, sys::NV_REMOVE_NOTIFY_HOOK, core::ptr::null()) };
    }
}

/// A running notification; EndNotify and FreeDosObject on drop.
pub struct Watcher {
    _t: crate::track::Tracked,
    req: APTR,
    sig: Signal,
    // The request points at the name: kept until EndNotify.
    _name: Vec<u8>,
}

impl Watcher {
    /// Watch `path` (a file or a directory; it need not exist yet).
    #[track_caller]
    pub fn new(path: &str) -> Result<Self> {
        let i = process()?;
        let name = c_copy(path.as_bytes())?;
        let sig = Signal::alloc()?;
        let tags = [
            TagItem { ti_Tag: sys::ADO_NOTIFY_NAME, ti_Data: name.as_ptr() as u32 },
            TagItem { ti_Tag: sys::ADO_NOTIFY_METHOD, ti_Data: sys::NRF_SEND_SIGNAL },
            TagItem { ti_Tag: sys::ADO_NOTIFY_TASK, ti_Data: crate::signal::current_task() as u32 },
            TagItem { ti_Tag: sys::ADO_NOTIFY_SIGNAL_NUMBER, ti_Data: sig.bit() as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: an open interface in a Process; a TAG_DONE-ended list
        // naming our NUL-terminated name (kept with the watcher), our task
        // and a signal bit we own.
        let req = unsafe { ((*i).AllocDosObject)(i, sys::DOS_NOTIFYREQUEST, tags.as_ptr()) };
        if req.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        // SAFETY: the request just made.
        if unsafe { ((*i).StartNotify)(i, req as *mut sys::NotifyRequest) } == 0 {
            let e = last_error(i);
            // SAFETY: our request, not started, freed once.
            unsafe { ((*i).FreeDosObject)(i, sys::DOS_NOTIFYREQUEST, req) };
            return Err(e);
        }
        Ok(Self { _t: crate::track::register("dos notification", req as usize, 0, release_watcher), req, sig, _name: name })
    }

    /// The signal mask to Wait() on with other signals.
    pub fn signal_mask(&self) -> u32 {
        self.sig.mask()
    }

    /// Whether a change was signalled since the last check (clears it).
    pub fn changed(&self) -> bool {
        self.sig.take()
    }

    /// Wait for the next change.
    pub fn wait(&self) {
        crate::signal::wait(self.sig.mask());
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: our started request: ended, then freed once, by the
            // Process that started it (Signal makes Watcher !Send).
            unsafe {
                ((*i).EndNotify)(i, self.req as *mut sys::NotifyRequest);
                ((*i).FreeDosObject)(i, sys::DOS_NOTIFYREQUEST, self.req);
            }
        }
    }
}

/// A watch on a global (ENV:) variable (NotifyVar, V50): its signal is set
/// when the variable is created, changed or deleted. The variable need not
/// exist yet. Ended (NV_REMOVE_NOTIFY_HOOK) on drop, by the Process that
/// started it (Signal makes it !Send).
///
/// DOS calls the notification from whichever process changes the variable,
/// so the hook here does nothing but signal this Process.
pub struct VarWatcher {
    _t: crate::track::Tracked,
    // Field order: the hook is removed from DOS in drop() before either is
    // freed.
    hook: crate::hook::Hook<alloc::boxed::Box<dyn Fn(APTR, APTR) -> u32 + Send + Sync>>,
    sig: Signal,
}

impl VarWatcher {
    /// Watch the global variable `name` ("Language", "Sys/def_disk.info";
    /// a leading "ENV:" or "ENVARC:" is ignored by DOS).
    #[track_caller]
    pub fn new(name: &str) -> Result<Self> {
        let i = process()?;
        let n = c_copy(name.as_bytes())?;
        if name.is_empty() {
            return Err(AmigaError::DosError(sys::ERROR_REQUIRED_ARG_MISSING));
        }
        let sig = Signal::alloc()?;
        let target = sig.target();
        let f: alloc::boxed::Box<dyn Fn(APTR, APTR) -> u32 + Send + Sync> = alloc::boxed::Box::new(move |_, _| {
            // SAFETY: the bit is ours until drop, which removes this hook
            // from DOS before freeing the bit.
            unsafe { target.send() };
            0
        });
        let hook = crate::hook::Hook::new(f)?;
        // SAFETY: an open interface in a Process, a NUL-terminated name (DOS
        // copies it into its own node) and a hook kept until drop.
        if unsafe { ((*i).NotifyVar)(i, n.as_ptr(), hook.as_ptr(), sys::NV_ADD_NOTIFY, core::ptr::null()) } == 0 {
            return Err(last_error(i));
        }
        Ok(Self { _t: crate::track::register("variable notification", hook.as_ptr() as usize, 0, release_var_watcher), hook, sig })
    }

    /// The signal mask to Wait() on with other signals.
    pub fn signal_mask(&self) -> u32 {
        self.sig.mask()
    }

    /// Whether a change was signalled since the last check (clears it).
    pub fn changed(&self) -> bool {
        self.sig.take()
    }

    /// Wait for the next change.
    pub fn wait(&self) {
        crate::signal::wait(self.sig.mask());
    }
}

impl Drop for VarWatcher {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: our hook, removed by the Process that added it; DOS
            // calls it no more once this returns.
            unsafe { ((*i).NotifyVar)(i, core::ptr::null(), self.hook.as_ptr(), sys::NV_REMOVE_NOTIFY_HOOK, core::ptr::null()) };
        }
    }
}
