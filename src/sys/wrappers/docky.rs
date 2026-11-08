//! IDocky global(s) and convenience wrappers.
//!
//! Hand-written binding for the AmigaOS dock-icon (AmiDock) interface
//! used by docky modules to interoperate with the Workbench dock.

use crate::sys::types::*;
use crate::sys::interfaces::docky::*;

// ---- IDocky (DockyIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IDocky: *mut DockyIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IDocky: *mut DockyIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn docky_docky_get(attr: u32, storage: *mut u32) -> u32 {
    ((*IDocky).DockyGet)(IDocky, attr, storage)
}

#[inline]
pub unsafe fn docky_docky_set(attr: u32, value: u32) -> u32 {
    ((*IDocky).DockySet)(IDocky, attr, value)
}

#[inline]
pub unsafe fn docky_docky_process(
    message: u32, arg1: *mut u32, arg2: *mut u32, arg3: *mut u32,
) -> u32 {
    ((*IDocky).DockyProcess)(IDocky, message, arg1, arg2, arg3)
}
