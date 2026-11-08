//! ICIA global(s) and convenience wrappers.
//!
//! Hand-written binding for cia.library — direct access to the Amiga
//! 8520 CIA chip(s), still emulated on AmigaOS 4.1 for legacy
//! hardware support. Used for ICR (interrupt control register)
//! handling.

use crate::sys::types::*;
use crate::sys::interfaces::cia::*;

// ---- ICIA (CIAIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ICIA: *mut CIAIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ICIA: *mut CIAIFace = core::ptr::null_mut();

// ── ICR vector management ────────────────────────────────────

#[inline]
pub unsafe fn cia_add_icrvector(resource: *mut Library, bit: i16, interrupt: *mut Interrupt) -> *mut Interrupt {
    ((*ICIA).AddICRVector)(ICIA, resource, bit, interrupt)
}

#[inline]
pub unsafe fn cia_rem_icrvector(resource: *mut Library, bit: i16, interrupt: *mut Interrupt) {
    ((*ICIA).RemICRVector)(ICIA, resource, bit, interrupt)
}

// ── ICR bit control ──────────────────────────────────────────

#[inline]
pub unsafe fn cia_able_icr(resource: *mut Library, mask: i16) -> i16 {
    ((*ICIA).AbleICR)(ICIA, resource, mask)
}

#[inline]
pub unsafe fn cia_set_icr(resource: *mut Library, mask: i16) -> i16 {
    ((*ICIA).SetICR)(ICIA, resource, mask)
}
