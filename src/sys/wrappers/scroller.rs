//! IScroller global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `SCROLLER_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::scroller::*;

// ---- IScroller (ScrollerIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IScroller: *mut ScrollerIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IScroller: *mut ScrollerIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn scroller_scroller_get_class() -> *mut APTR {
    ((*IScroller).SCROLLER_GetClass)(IScroller)
}
