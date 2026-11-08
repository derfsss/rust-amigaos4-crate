//! A utility.library Hook that calls a Rust closure (AllocSysObject
//! ASOT_HOOK, MEMF_SHARED).
//!
//! The closure gets the hook's object and message pointers as the caller
//! passes them (`h_Entry(hook, object, message)`, the native calling
//! convention) and returns the hook's 32-bit result. It must be `Send` and
//! `Sync`: the OS may call a hook from any task (an item pool's
//! constructor, a scan that another task runs). The hook lives as long as
//! the [`Hook`]: pass it only to calls that use it while it is borrowed.

use alloc::boxed::Box;
use core::marker::PhantomData;

use crate::error::{AmigaError, Result};
use crate::sys::{self, TagItem, APTR, TAG_DONE};

/// struct Hook's h_Data offset (tests/layout/exec.c).
const H_DATA: usize = 16;

/// A hook calling `F`.
pub struct Hook<F: Fn(APTR, APTR) -> u32 + Send + Sync> {
    raw: *mut sys::Hook,
    _f: PhantomData<Box<F>>,
}

// SAFETY: the closure is Send + Sync; the hook object is MEMF_SHARED and
// only freed by drop.
unsafe impl<F: Fn(APTR, APTR) -> u32 + Send + Sync> Send for Hook<F> {}
// SAFETY: as above: calls only read the closure through a shared reference.
unsafe impl<F: Fn(APTR, APTR) -> u32 + Send + Sync> Sync for Hook<F> {}

extern "C" fn trampoline<F: Fn(APTR, APTR) -> u32 + Send + Sync>(hook: *mut sys::Hook, object: APTR, message: APTR) -> u32 {
    // SAFETY: h_Data holds the Box<F> this hook was made with, alive while
    // the Hook is (the caller only calls a hook it was lent).
    let f = unsafe { &*(core::ptr::read((hook as *const u8).add(H_DATA) as *const *const F)) };
    f(object, message)
}

impl<F: Fn(APTR, APTR) -> u32 + Send + Sync> Hook<F> {
    pub fn new(f: F) -> Result<Self> {
        let data = Box::into_raw(Box::new(f));
        let entry: extern "C" fn(*mut sys::Hook, APTR, APTR) -> u32 = trampoline::<F>;
        let tags = [
            TagItem { ti_Tag: sys::ASOHOOK_ENTRY, ti_Data: entry as usize as u32 },
            TagItem { ti_Tag: sys::ASOHOOK_DATA, ti_Data: data as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list.
        let raw = unsafe { sys::exec_alloc_sys_object(sys::ASOT_HOOK, tags.as_ptr()) } as *mut sys::Hook;
        if raw.is_null() {
            // SAFETY: from Box::into_raw just above, never shared.
            drop(unsafe { Box::from_raw(data) });
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { raw, _f: PhantomData })
    }

    /// The struct Hook to pass to an OS call.
    pub fn as_ptr(&self) -> *mut sys::Hook {
        self.raw
    }
}

impl<F: Fn(APTR, APTR) -> u32 + Send + Sync> Drop for Hook<F> {
    fn drop(&mut self) {
        // SAFETY: our hook: its h_Data is our Box<F>, both freed once; no
        // call can be running (a call borrows the Hook).
        unsafe {
            let data = core::ptr::read((self.raw as *const u8).add(H_DATA) as *const *mut F);
            sys::exec_free_sys_object(sys::ASOT_HOOK, self.raw as APTR);
            drop(Box::from_raw(data));
        }
    }
}
