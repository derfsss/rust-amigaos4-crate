//! IMounterPrivate global(s) and convenience wrappers.
//!
//! Hand-written binding for mounter.library's private sub-interface
//! (lives in `mounter.h` alongside MounterIFace). One method that
//! the system uses to signal end-of-boot to the mounter.

use crate::sys::types::*;
use crate::sys::interfaces::mounter_private::*;

// ---- IMounterPrivate (MounterPrivateIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IMounterPrivate: *mut MounterPrivateIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IMounterPrivate: *mut MounterPrivateIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn mounter_private_full_booted() {
    ((*IMounterPrivate).FullBooted)(IMounterPrivate)
}
