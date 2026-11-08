//! IMounter global(s) and convenience wrappers.
//!
//! Hand-written binding for mounter.library — announces/denounces
//! storage devices to the system so DOSDriver mounts can fire.

use crate::sys::types::*;
use crate::sys::interfaces::mounter::*;

// ---- IMounter (MounterIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IMounter: *mut MounterIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IMounter: *mut MounterIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn mounter_announce_device(device_name: CONST_STRPTR, unit: u32, tag_list: *mut TagItem) -> u32 {
    ((*IMounter).AnnounceDevice)(IMounter, device_name, unit, tag_list)
}

#[inline]
pub unsafe fn mounter_denounce_device(device_name: CONST_STRPTR, unit: u32) {
    ((*IMounter).DenounceDevice)(IMounter, device_name, unit)
}
