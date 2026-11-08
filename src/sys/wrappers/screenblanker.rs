//! IScreenBlanker global(s) and convenience wrappers.
//!
//! Hand-written binding for screenblanker.library — Workbench
//! screen-blanker control. Open and close blanker modules, set the
//! blanking mode (on / off / forced), and get/set blanker attrs.
//!
//! Note: `ScreenBlankerIFace::Clone` returns `*mut IScreenBlanker`
//! (a specific cloned-interface type) rather than the generic
//! `*mut Interface` — atypical for the framework.

use crate::sys::types::*;
use crate::sys::interfaces::screenblanker::*;

// ---- IScreenBlanker (ScreenBlankerIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IScreenBlankerLib: *mut ScreenBlankerIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IScreenBlankerLib: *mut ScreenBlankerIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn screenblanker_open_blanker_module_a(name: CONST_STRPTR, tag_list: *mut TagItem) -> u32 {
    ((*IScreenBlankerLib).OpenBlankerModuleA)(IScreenBlankerLib, name, tag_list)
}

#[inline]
pub unsafe fn screenblanker_close_blanker_module() {
    ((*IScreenBlankerLib).CloseBlankerModule)(IScreenBlankerLib)
}

#[inline]
pub unsafe fn screenblanker_set_blanking_mode(mode: u32) -> u32 {
    ((*IScreenBlankerLib).SetBlankingMode)(IScreenBlankerLib, mode)
}

#[inline]
pub unsafe fn screenblanker_set_screen_blanker_attrs_a(tag_list: *mut TagItem) -> u32 {
    ((*IScreenBlankerLib).SetScreenBlankerAttrsA)(IScreenBlankerLib, tag_list)
}

#[inline]
pub unsafe fn screenblanker_get_screen_blanker_attrs_a(tag_list: *mut TagItem) -> u32 {
    ((*IScreenBlankerLib).GetScreenBlankerAttrsA)(IScreenBlankerLib, tag_list)
}
