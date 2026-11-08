//! IARexx global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `AREXX_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::arexx::*;

// ---- IARexx (ARexxIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IARexx: *mut ARexxIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IARexx: *mut ARexxIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn arexx_arexx_get_class() -> *mut APTR {
    ((*IARexx).AREXX_GetClass)(IARexx)
}
