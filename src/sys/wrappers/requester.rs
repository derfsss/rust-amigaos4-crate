//! IRequester global(s) and convenience wrappers.
//!
//! Hand-written ReAction requester-class binding. The only user-facing
//! method is `REQUESTER_GetClass`; lifecycle methods are
//! framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::requester::*;

// ---- IRequester (RequesterIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IRequester: *mut RequesterIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IRequester: *mut RequesterIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn requester_requester_get_class() -> *mut APTR {
    ((*IRequester).REQUESTER_GetClass)(IRequester)
}
