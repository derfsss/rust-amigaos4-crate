//! IBootSD global(s) and convenience wrappers.
//!
//! Hand-written binding for bootsd.resource — early-boot SD-card
//! access used by the kickstart loader.

use crate::sys::types::*;
use crate::sys::interfaces::bootsd::*;

// ---- IBootSD (BootSDIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IBootSD: *mut BootSDIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IBootSD: *mut BootSDIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn bootsd_alloc_resource(io_err: *mut i32) -> APTR {
    ((*IBootSD).AllocResource)(IBootSD, io_err)
}

#[inline]
pub unsafe fn bootsd_free_resource(resource: APTR) -> u32 {
    ((*IBootSD).FreeResource)(IBootSD, resource)
}

#[inline]
pub unsafe fn bootsd_get_geometry(resource: APTR, io_err: *mut i32, envec: *mut DosEnvec) -> APTR {
    ((*IBootSD).GetGeometry)(IBootSD, resource, io_err, envec)
}

#[inline]
pub unsafe fn bootsd_read_sdcard(
    resource: APTR, io_err: *mut i32, buffer: APTR, start_block: u32, count: u32,
) -> u32 {
    ((*IBootSD).ReadSDCard)(IBootSD, resource, io_err, buffer, start_block, count)
}

#[inline]
pub unsafe fn bootsd_write_sdcard(
    resource: APTR, io_err: *mut i32, buffer: APTR, start_block: u32, count: u32,
) -> u32 {
    ((*IBootSD).WriteSDCard)(IBootSD, resource, io_err, buffer, start_block, count)
}
