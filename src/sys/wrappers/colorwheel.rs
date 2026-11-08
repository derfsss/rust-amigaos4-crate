//! IColorWheel global(s) and convenience wrappers.
//!
//! Hand-written ReAction colorwheel-class binding. Note that
//! `colorwheel.gadget` exposes pure conversion utilities rather than
//! a `<NAME>_GetClass` method; the gadget itself is fetched via
//! `OpenLibrary("colorwheel.gadget", 0)`.

use crate::sys::types::*;
use crate::sys::interfaces::colorwheel::*;

// ---- IColorWheel (ColorWheelIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IColorWheel: *mut ColorWheelIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IColorWheel: *mut ColorWheelIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn colorwheel_convert_hsbto_rgb(hsb: *mut ColorWheelHSB, rgb: *mut ColorWheelRGB) {
    ((*IColorWheel).ConvertHSBToRGB)(IColorWheel, hsb, rgb)
}

#[inline]
pub unsafe fn colorwheel_convert_rgbto_hsb(rgb: *mut ColorWheelRGB, hsb: *mut ColorWheelHSB) {
    ((*IColorWheel).ConvertRGBToHSB)(IColorWheel, rgb, hsb)
}
