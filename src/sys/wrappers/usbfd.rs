//! IUSBFD global(s) and convenience wrappers.
//!
//! Hand-written binding for USB function-driver interface.

use crate::sys::types::*;
use crate::sys::interfaces::usbfd::*;

// ---- IUSBFD (USBFDIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IUSBFD: *mut USBFDIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IUSBFD: *mut USBFDIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn usbfd_usbfdget_attrs_a(tag_list: *mut TagItem) {
    ((*IUSBFD).USBFDGetAttrsA)(IUSBFD, tag_list)
}

#[inline]
pub unsafe fn usbfd_usbfdrun_function(msg: *mut USBFDStartupMsg) -> i32 {
    ((*IUSBFD).USBFDRunFunction)(IUSBFD, msg)
}

#[inline]
pub unsafe fn usbfd_usbfdrun_interface(msg: *mut USBFDStartupMsg) -> i32 {
    ((*IUSBFD).USBFDRunInterface)(IUSBFD, msg)
}
