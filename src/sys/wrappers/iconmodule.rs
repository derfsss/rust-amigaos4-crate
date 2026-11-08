//! IIconModule global(s) and convenience wrappers.
//!
//! Hand-written binding for icon-module plugins — the per-format
//! handlers icon.library calls into to load/save/scale specific
//! icon formats (the classic icon.library, PNG, info, etc.).

use crate::sys::types::*;
use crate::sys::interfaces::iconmodule::*;

// ---- IIconModule (IconModuleIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IIconModule: *mut IconModuleIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IIconModule: *mut IconModuleIFace = core::ptr::null_mut();

// ── Module control ───────────────────────────────────────────

#[inline]
pub unsafe fn iconmodule_module_control_a(arg: APTR, tag_list: *mut TagItem) -> u32 {
    ((*IIconModule).ModuleControlA)(IIconModule, arg, tag_list)
}

// ── Load / save ──────────────────────────────────────────────

#[inline]
pub unsafe fn iconmodule_load_icon_a(
    name: CONST_STRPTR, def_icon: *mut DiskObject, tag_list: *mut TagItem,
) -> *mut DiskObject {
    ((*IIconModule).LoadIconA)(IIconModule, name, def_icon, tag_list)
}

#[inline]
pub unsafe fn iconmodule_save_icon_a(
    name: CONST_STRPTR, icon: *mut DiskObject, tag_list: *mut TagItem,
) -> u32 {
    ((*IIconModule).SaveIconA)(IIconModule, name, icon, tag_list)
}

// ── Icon-data lifecycle ──────────────────────────────────────

#[inline]
pub unsafe fn iconmodule_alloc_icon_data_a(icon: *mut DiskObject, tag_list: *mut TagItem) -> u32 {
    ((*IIconModule).AllocIconDataA)(IIconModule, icon, tag_list)
}

#[inline]
pub unsafe fn iconmodule_free_icon_data_a(icon: *mut DiskObject, tag_list: *mut TagItem) {
    ((*IIconModule).FreeIconDataA)(IIconModule, icon, tag_list)
}

#[inline]
pub unsafe fn iconmodule_dup_icon_data_a(icon: *mut DiskObject, tag_list: *mut TagItem) -> APTR {
    ((*IIconModule).DupIconDataA)(IIconModule, icon, tag_list)
}

// ── Scaling ──────────────────────────────────────────────────

#[inline]
pub unsafe fn iconmodule_scale_icon_a(
    icon: *mut DiskObject, request: *mut IconScaleRequest, tag_list: *mut TagItem,
) -> u32 {
    ((*IIconModule).ScaleIconA)(IIconModule, icon, request, tag_list)
}
