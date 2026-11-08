//! ILowLevel global(s) and convenience wrappers.
//!
//! Hand-written to match the amigaos4-bindgen convention used by the
//! other 19 wrapper modules.

use crate::sys::types::*;
use crate::sys::interfaces::lowlevel::*;

// ---- ILowLevel (LowLevelIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ILowLevel: *mut LowLevelIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ILowLevel: *mut LowLevelIFace = core::ptr::null_mut();

// ── Input polling ────────────────────────────────────────────

#[inline]
pub unsafe fn lowlevel_read_joy_port(port: u32) -> u32 {
    ((*ILowLevel).ReadJoyPort)(ILowLevel, port)
}

#[inline]
pub unsafe fn lowlevel_get_language_selection() -> u8 {
    ((*ILowLevel).GetLanguageSelection)(ILowLevel)
}

#[inline]
pub unsafe fn lowlevel_get_key() -> u32 {
    ((*ILowLevel).GetKey)(ILowLevel)
}

#[inline]
pub unsafe fn lowlevel_query_keys(keys: *mut KeyQuery, length: u8) {
    ((*ILowLevel).QueryKeys)(ILowLevel, keys, length)
}

#[inline]
pub unsafe fn lowlevel_set_joy_port_attrs_a(port_num: u32, tag_list: *const TagItem) -> u32 {
    ((*ILowLevel).SetJoyPortAttrsA)(ILowLevel, port_num, tag_list)
}

// ── Keyboard interrupt hook ──────────────────────────────────

#[inline]
pub unsafe fn lowlevel_add_kbint(int_func: APTR, user_data: APTR) -> APTR {
    ((*ILowLevel).AddKBInt)(ILowLevel, int_func, user_data)
}

#[inline]
pub unsafe fn lowlevel_rem_kbint(handle: APTR) {
    ((*ILowLevel).RemKBInt)(ILowLevel, handle)
}

// ── Timer interrupt hook ─────────────────────────────────────

#[inline]
pub unsafe fn lowlevel_add_timer_int(int_func: APTR, user_data: APTR) -> APTR {
    ((*ILowLevel).AddTimerInt)(ILowLevel, int_func, user_data)
}

#[inline]
pub unsafe fn lowlevel_rem_timer_int(handle: APTR) {
    ((*ILowLevel).RemTimerInt)(ILowLevel, handle)
}

#[inline]
pub unsafe fn lowlevel_stop_timer_int(handle: APTR) {
    ((*ILowLevel).StopTimerInt)(ILowLevel, handle)
}

#[inline]
pub unsafe fn lowlevel_start_timer_int(handle: APTR, time_interval: u32, continuous: u32) {
    ((*ILowLevel).StartTimerInt)(ILowLevel, handle, time_interval, continuous)
}

#[inline]
pub unsafe fn lowlevel_elapsed_time(context: *mut EClockVal) -> u32 {
    ((*ILowLevel).ElapsedTime)(ILowLevel, context)
}

// ── VBlank interrupt hook ────────────────────────────────────

#[inline]
pub unsafe fn lowlevel_add_vblank_int(int_func: APTR, user_data: APTR) -> APTR {
    ((*ILowLevel).AddVBlankInt)(ILowLevel, int_func, user_data)
}

#[inline]
pub unsafe fn lowlevel_rem_vblank_int(handle: APTR) {
    ((*ILowLevel).RemVBlankInt)(ILowLevel, handle)
}

// ── System control ───────────────────────────────────────────

#[inline]
pub unsafe fn lowlevel_system_control_a(tags: *const TagItem) -> u32 {
    ((*ILowLevel).SystemControlA)(ILowLevel, tags)
}
