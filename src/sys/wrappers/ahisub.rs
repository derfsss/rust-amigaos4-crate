//! IAHIsub global(s) and convenience wrappers.
//!
//! Hand-written binding for ahi sub-drivers (AHIsub.* interface) —
//! the per-hardware audio backend ahi.device calls into. All
//! `AHIsub_*`-prefixed per the SDK convention.

use crate::sys::types::*;
use crate::sys::interfaces::ahisub::*;

// ---- IAHIsub (AHIsubIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IAHIsub: *mut AHIsubIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IAHIsub: *mut AHIsubIFace = core::ptr::null_mut();

// ── Driver lifecycle ─────────────────────────────────────────

#[inline]
pub unsafe fn ahisub_ahisub_alloc_audio(tag_list: *mut TagItem, ctrl: *mut AHIAudioCtrlDrv) -> u32 {
    ((*IAHIsub).AHIsub_AllocAudio)(IAHIsub, tag_list, ctrl)
}

#[inline]
pub unsafe fn ahisub_ahisub_free_audio(ctrl: *mut AHIAudioCtrlDrv) {
    ((*IAHIsub).AHIsub_FreeAudio)(IAHIsub, ctrl)
}

#[inline]
pub unsafe fn ahisub_ahisub_disable(ctrl: *mut AHIAudioCtrlDrv) {
    ((*IAHIsub).AHIsub_Disable)(IAHIsub, ctrl)
}

#[inline]
pub unsafe fn ahisub_ahisub_enable(ctrl: *mut AHIAudioCtrlDrv) {
    ((*IAHIsub).AHIsub_Enable)(IAHIsub, ctrl)
}

// ── Playback control ─────────────────────────────────────────

#[inline]
pub unsafe fn ahisub_ahisub_start(flags: u32, ctrl: *mut AHIAudioCtrlDrv) -> u32 {
    ((*IAHIsub).AHIsub_Start)(IAHIsub, flags, ctrl)
}

#[inline]
pub unsafe fn ahisub_ahisub_update(flags: u32, ctrl: *mut AHIAudioCtrlDrv) -> u32 {
    ((*IAHIsub).AHIsub_Update)(IAHIsub, flags, ctrl)
}

#[inline]
pub unsafe fn ahisub_ahisub_stop(flags: u32, ctrl: *mut AHIAudioCtrlDrv) -> u32 {
    ((*IAHIsub).AHIsub_Stop)(IAHIsub, flags, ctrl)
}

// ── Voice / mixer control ────────────────────────────────────

#[inline]
pub unsafe fn ahisub_ahisub_set_vol(
    channel: u16, volume: APTR, pan: APTR, ctrl: *mut AHIAudioCtrlDrv, flags: u32,
) -> u32 {
    ((*IAHIsub).AHIsub_SetVol)(IAHIsub, channel, volume, pan, ctrl, flags)
}

#[inline]
pub unsafe fn ahisub_ahisub_set_freq(
    channel: u16, freq: u32, ctrl: *mut AHIAudioCtrlDrv, flags: u32,
) -> u32 {
    ((*IAHIsub).AHIsub_SetFreq)(IAHIsub, channel, freq, ctrl, flags)
}

#[inline]
pub unsafe fn ahisub_ahisub_set_sound(
    channel: u16, sound: u16, offset: u32, length: i32, ctrl: *mut AHIAudioCtrlDrv, flags: u32,
) -> u32 {
    ((*IAHIsub).AHIsub_SetSound)(IAHIsub, channel, sound, offset, length, ctrl, flags)
}

#[inline]
pub unsafe fn ahisub_ahisub_set_effect(effect: APTR, ctrl: *mut AHIAudioCtrlDrv) -> u32 {
    ((*IAHIsub).AHIsub_SetEffect)(IAHIsub, effect, ctrl)
}

// ── Sound bank ───────────────────────────────────────────────

#[inline]
pub unsafe fn ahisub_ahisub_load_sound(
    sound: u16, sound_type: u32, info: APTR, ctrl: *mut AHIAudioCtrlDrv,
) -> u32 {
    ((*IAHIsub).AHIsub_LoadSound)(IAHIsub, sound, sound_type, info, ctrl)
}

#[inline]
pub unsafe fn ahisub_ahisub_unload_sound(sound: u16, ctrl: *mut AHIAudioCtrlDrv) -> u32 {
    ((*IAHIsub).AHIsub_UnloadSound)(IAHIsub, sound, ctrl)
}

// ── Attribute access + hardware control ──────────────────────

#[inline]
pub unsafe fn ahisub_ahisub_get_attr(
    attribute: u32, argument: i32, default: i32, tag_list: *mut TagItem,
    ctrl: *mut AHIAudioCtrlDrv,
) -> i32 {
    ((*IAHIsub).AHIsub_GetAttr)(IAHIsub, attribute, argument, default, tag_list, ctrl)
}

#[inline]
pub unsafe fn ahisub_ahisub_hardware_control(
    attribute: u32, argument: i32, ctrl: *mut AHIAudioCtrlDrv,
) -> i32 {
    ((*IAHIsub).AHIsub_HardwareControl)(IAHIsub, attribute, argument, ctrl)
}
