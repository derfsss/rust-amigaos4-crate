//! IFiller global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `FILLER_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::filler::*;

// ---- IFiller (FillerIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IFiller: *mut FillerIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IFiller: *mut FillerIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn filler_filler_get_class() -> *mut APTR {
    ((*IFiller).FILLER_GetClass)(IFiller)
}
