//! IRamdrive global(s) and convenience wrappers.
//!
//! Hand-written binding for ramdrive.device — the AmigaOS recoverable
//! RAM disk (RAD:). The two methods kill RAD0: or a specified RAD
//! unit respectively, freeing its memory.

use crate::sys::types::*;
use crate::sys::interfaces::ramdrive::*;

// ---- IRamdrive (RamdriveIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IRamdrive: *mut RamdriveIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IRamdrive: *mut RamdriveIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn ramdrive_kill_rad0() -> STRPTR {
    ((*IRamdrive).KillRAD0)(IRamdrive)
}

#[inline]
pub unsafe fn ramdrive_kill_rad(unit: u32) -> STRPTR {
    ((*IRamdrive).KillRAD)(IRamdrive, unit)
}
