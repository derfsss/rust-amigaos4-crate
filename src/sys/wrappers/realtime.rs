//! IRealTime global(s) and convenience wrappers.
//!
//! Hand-written to match the amigaos4-bindgen convention used by the
//! other wrapper modules.

use crate::sys::types::*;
use crate::sys::interfaces::realtime::*;

// ---- IRealTime (RealTimeIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IRealTime: *mut RealTimeIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IRealTime: *mut RealTimeIFace = core::ptr::null_mut();

// ── Locking ──────────────────────────────────────────────────

#[inline]
pub unsafe fn realtime_lock_real_time(mode: u32) -> APTR {
    ((*IRealTime).LockRealTime)(IRealTime, mode)
}

#[inline]
pub unsafe fn realtime_unlock_real_time(lock: APTR) {
    ((*IRealTime).UnlockRealTime)(IRealTime, lock)
}

// ── Player lifecycle ─────────────────────────────────────────

#[inline]
pub unsafe fn realtime_create_player_a(tag_list: *const TagItem) -> *mut Player {
    ((*IRealTime).CreatePlayerA)(IRealTime, tag_list)
}

#[inline]
pub unsafe fn realtime_delete_player(player: *mut Player) {
    ((*IRealTime).DeletePlayer)(IRealTime, player)
}

#[inline]
pub unsafe fn realtime_set_player_attrs_a(player: *mut Player, tag_list: *const TagItem) -> u32 {
    ((*IRealTime).SetPlayerAttrsA)(IRealTime, player, tag_list)
}

#[inline]
pub unsafe fn realtime_get_player_attrs_a(player: *const Player, tag_list: *const TagItem) -> u32 {
    ((*IRealTime).GetPlayerAttrsA)(IRealTime, player, tag_list)
}

// ── Conductor control ────────────────────────────────────────

#[inline]
pub unsafe fn realtime_set_conductor_state(player: *mut Player, state: u32, time: i32) -> i32 {
    ((*IRealTime).SetConductorState)(IRealTime, player, state, time)
}

#[inline]
pub unsafe fn realtime_external_sync(player: *mut Player, min_time: i32, max_time: i32) -> u32 {
    ((*IRealTime).ExternalSync)(IRealTime, player, min_time, max_time)
}

#[inline]
pub unsafe fn realtime_next_conductor(previous: *const Conductor) -> *mut Conductor {
    ((*IRealTime).NextConductor)(IRealTime, previous)
}

#[inline]
pub unsafe fn realtime_find_conductor(name: CONST_STRPTR) -> *mut Conductor {
    ((*IRealTime).FindConductor)(IRealTime, name)
}
