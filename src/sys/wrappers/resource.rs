//! IResource global(s) and convenience wrappers.
//!
//! Hand-written binding for resource.library — the ReAction resource
//! loader. Lets a program describe a window/UI tree in an external
//! resource file and instantiate objects/groups from it at runtime
//! rather than building everything via LayoutBuilder in code.

use crate::sys::types::*;
use crate::sys::interfaces::resource::*;

// ---- IResource (ResourceIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IResource: *mut ResourceIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IResource: *mut ResourceIFace = core::ptr::null_mut();

// ── Resource-file lifecycle ──────────────────────────────────

#[inline]
pub unsafe fn resource_rl_open_resource(filename: APTR, screen: *mut Screen, catalog: *mut Catalog) -> APTR {
    ((*IResource).RL_OpenResource)(IResource, filename, screen, catalog)
}

#[inline]
pub unsafe fn resource_rl_close_resource(handle: APTR) {
    ((*IResource).RL_CloseResource)(IResource, handle)
}

// ── Object instantiation ─────────────────────────────────────

#[inline]
pub unsafe fn resource_rl_new_object_a(handle: APTR, id: APTR, tag_list: *mut TagItem) -> *mut APTR {
    ((*IResource).RL_NewObjectA)(IResource, handle, id, tag_list)
}

#[inline]
pub unsafe fn resource_rl_dispose_object(handle: APTR, object: *mut APTR) {
    ((*IResource).RL_DisposeObject)(IResource, handle, object)
}

// ── Group instantiation ──────────────────────────────────────

#[inline]
pub unsafe fn resource_rl_new_group_a(handle: APTR, id: APTR, tag_list: *mut TagItem) -> *mut *mut APTR {
    ((*IResource).RL_NewGroupA)(IResource, handle, id, tag_list)
}

#[inline]
pub unsafe fn resource_rl_dispose_group(handle: APTR, group: *mut *mut APTR) {
    ((*IResource).RL_DisposeGroup)(IResource, handle, group)
}

#[inline]
pub unsafe fn resource_rl_get_object_array(handle: APTR, ids: *mut APTR, sentinel: APTR) -> *mut *mut APTR {
    ((*IResource).RL_GetObjectArray)(IResource, handle, ids, sentinel)
}

#[inline]
pub unsafe fn resource_rl_set_resource_screen(handle: APTR, screen: *mut Screen) -> u32 {
    ((*IResource).RL_SetResourceScreen)(IResource, handle, screen)
}
