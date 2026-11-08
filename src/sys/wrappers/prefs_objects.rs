//! IPrefsObjects global(s) and convenience wrappers.
//!
//! Hand-written binding for prefs-objects.library — typed-prefs
//! object container (similar to NSDictionary/NSArray-style trees).

use crate::sys::types::*;
use crate::sys::interfaces::prefs_objects::*;

// ---- IPrefsObjects (PrefsObjectsIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IPrefsObjects: *mut PrefsObjectsIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IPrefsObjects: *mut PrefsObjectsIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn prefs_objects_begin_deserialization(info: *mut ALPOGetProcInfo) -> u32 {
    ((*IPrefsObjects).BeginDeserialization)(IPrefsObjects, info)
}

// ── Typed-prefs constructors (A-suffix taglist variants) ─────

#[inline]
pub unsafe fn prefs_objects_prefs_base_object_a(parent: *mut APTR, length: *mut u32, tag_list: *mut TagItem) -> *mut APTR {
    ((*IPrefsObjects).PrefsBaseObjectA)(IPrefsObjects, parent, length, tag_list)
}

#[inline]
pub unsafe fn prefs_objects_prefs_string_a(parent: *mut APTR, length: *mut u32, tag_list: *mut TagItem) -> *mut APTR {
    ((*IPrefsObjects).PrefsStringA)(IPrefsObjects, parent, length, tag_list)
}

#[inline]
pub unsafe fn prefs_objects_prefs_number_a(parent: *mut APTR, length: *mut u32, tag_list: *mut TagItem) -> *mut APTR {
    ((*IPrefsObjects).PrefsNumberA)(IPrefsObjects, parent, length, tag_list)
}

#[inline]
pub unsafe fn prefs_objects_prefs_date_a(parent: *mut APTR, length: *mut u32, tag_list: *mut TagItem) -> *mut APTR {
    ((*IPrefsObjects).PrefsDateA)(IPrefsObjects, parent, length, tag_list)
}

#[inline]
pub unsafe fn prefs_objects_prefs_binary_a(parent: *mut APTR, length: *mut u32, tag_list: *mut TagItem) -> *mut APTR {
    ((*IPrefsObjects).PrefsBinaryA)(IPrefsObjects, parent, length, tag_list)
}

#[inline]
pub unsafe fn prefs_objects_prefs_dictionary_a(parent: *mut APTR, length: *mut u32, tag_list: *mut TagItem) -> *mut APTR {
    ((*IPrefsObjects).PrefsDictionaryA)(IPrefsObjects, parent, length, tag_list)
}

#[inline]
pub unsafe fn prefs_objects_prefs_array_a(parent: *mut APTR, length: *mut u32, tag_list: *mut TagItem) -> *mut APTR {
    ((*IPrefsObjects).PrefsArrayA)(IPrefsObjects, parent, length, tag_list)
}

// ── Dictionary access ───────────────────────────────────────

#[inline]
pub unsafe fn prefs_objects_dict_set_object_for_key(dict: *mut APTR, obj: *mut APTR, key: CONST_STRPTR) -> u32 {
    ((*IPrefsObjects).DictSetObjectForKey)(IPrefsObjects, dict, obj, key)
}

#[inline]
pub unsafe fn prefs_objects_dict_get_object_for_key(dict: *mut APTR, key: CONST_STRPTR) -> *mut APTR {
    ((*IPrefsObjects).DictGetObjectForKey)(IPrefsObjects, dict, key)
}

#[inline]
pub unsafe fn prefs_objects_dict_get_string_for_key(
    dict: *mut APTR, key: CONST_STRPTR, default_value: CONST_STRPTR,
) -> CONST_STRPTR {
    ((*IPrefsObjects).DictGetStringForKey)(IPrefsObjects, dict, key, default_value)
}

#[inline]
pub unsafe fn prefs_objects_dict_get_integer_for_key(
    dict: *mut APTR, key: CONST_STRPTR, default_value: i32,
) -> i32 {
    ((*IPrefsObjects).DictGetIntegerForKey)(IPrefsObjects, dict, key, default_value)
}

#[inline]
pub unsafe fn prefs_objects_dict_get_bool_for_key(
    dict: *mut APTR, key: CONST_STRPTR, default_value: u32,
) -> u32 {
    ((*IPrefsObjects).DictGetBoolForKey)(IPrefsObjects, dict, key, default_value)
}

#[inline]
pub unsafe fn prefs_objects_dict_get_option_for_key(
    dict: *mut APTR, key: CONST_STRPTR, options: *mut CONST_STRPTR, default_index: i32,
) -> i32 {
    ((*IPrefsObjects).DictGetOptionForKey)(IPrefsObjects, dict, key, options, default_index)
}

// ── Read / write ────────────────────────────────────────────

#[inline]
pub unsafe fn prefs_objects_read_prefs_a(root: *mut APTR, tag_list: *mut TagItem) -> u32 {
    ((*IPrefsObjects).ReadPrefsA)(IPrefsObjects, root, tag_list)
}

#[inline]
pub unsafe fn prefs_objects_write_prefs_a(root: *mut APTR, tag_list: *mut TagItem) -> u32 {
    ((*IPrefsObjects).WritePrefsA)(IPrefsObjects, root, tag_list)
}
