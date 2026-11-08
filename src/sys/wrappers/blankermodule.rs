//! IBlankerModule global(s) and convenience wrappers.
//!
//! Hand-written binding for blanker-module plugins — per-effect
//! handlers screenblanker.library invokes to render the actual
//! blanking animation. Bound from `blankermodule_lib.xml` in the SDK.

use crate::sys::types::*;
use crate::sys::interfaces::blankermodule::*;

// ---- IBlankerModule (BlankerModuleIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IBlankerModule: *mut BlankerModuleIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IBlankerModule: *mut BlankerModuleIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn blankermodule_blanker_module_get(attr: u32, storage: *mut u32) -> u32 {
    ((*IBlankerModule).BlankerModuleGet)(IBlankerModule, attr, storage)
}

#[inline]
pub unsafe fn blankermodule_blanker_module_set(attr: u32, value: u32) -> u32 {
    ((*IBlankerModule).BlankerModuleSet)(IBlankerModule, attr, value)
}

#[inline]
pub unsafe fn blankermodule_blanker_module_blank_task() {
    ((*IBlankerModule).BlankerModuleBlankTask)(IBlankerModule)
}
