//! ISlider global(s) and convenience wrappers.
//!
//! Hand-written ReAction gadget-class binding. Like the other
//! single-`GetClass` gadgets, the only user-facing method is
//! `SLIDER_GetClass`; lifecycle methods are framework-owned.

use crate::sys::types::*;
use crate::sys::interfaces::slider::*;

// ---- ISlider (SliderIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ISlider: *mut SliderIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ISlider: *mut SliderIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn slider_slider_get_class() -> *mut APTR {
    ((*ISlider).SLIDER_GetClass)(ISlider)
}
