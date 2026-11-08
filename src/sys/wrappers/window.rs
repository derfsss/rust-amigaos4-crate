//! IWindow global(s) and convenience wrappers.
//!
//! Hand-written ReAction window-class binding. Beyond the standard
//! `WINDOW_GetClass` accessor, exposes prefs management methods used
//! by the Workbench window-prefs editor.

use crate::sys::types::*;
use crate::sys::interfaces::window::*;

// ---- IWindow (WindowIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IWindow: *mut WindowIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IWindow: *mut WindowIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn window_window_get_class() -> *mut APTR {
    ((*IWindow).WINDOW_GetClass)(IWindow)
}

#[inline]
pub unsafe fn window_new_window_prefs() {
    ((*IWindow).NewWindowPrefs)(IWindow)
}

/// SDK-internal slot — declared as a regular vtable entry but body
/// is unspecified. Mirrored from the SDK header.
#[inline]
pub unsafe fn window_window_private1() -> u32 {
    ((*IWindow).WindowPrivate1)(IWindow)
}

#[inline]
pub unsafe fn window_update_window_prefs(screen: *mut Screen) {
    ((*IWindow).UpdateWindowPrefs)(IWindow, screen)
}
