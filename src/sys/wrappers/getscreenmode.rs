//! IGetScreenMode global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `GETSCREENMODE_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::getscreenmode::*;

// ---- IGetScreenMode (GetScreenModeIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IGetScreenMode: *mut GetScreenModeIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IGetScreenMode: *mut GetScreenModeIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn getscreenmode_getscreenmode_get_class() -> *mut APTR {
    ((*IGetScreenMode).GETSCREENMODE_GetClass)(IGetScreenMode)
}
