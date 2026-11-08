//! IAHI global(s) and convenience wrappers.
//!
//! Hand-written binding for ahi.device — AmigaOS's standard audio
//! API. All methods are `AHI_*`-prefixed per the SDK convention.

use crate::sys::types::*;
use crate::sys::interfaces::ahi::*;

// ---- IAHI (AHIIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IAHI: *mut AHIIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IAHI: *mut AHIIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn ahi_ahi_alloc_audio_a(tag_list: *mut TagItem) -> *mut AHIAudioCtrl {
    ((*IAHI).AHI_AllocAudioA)(IAHI, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_free_audio(ctrl: *mut AHIAudioCtrl) {
    ((*IAHI).AHI_FreeAudio)(IAHI, ctrl)
}

#[inline]
pub unsafe fn ahi_ahi_kill_audio() {
    ((*IAHI).AHI_KillAudio)(IAHI)
}

#[inline]
pub unsafe fn ahi_ahi_control_audio_a(ctrl: *mut AHIAudioCtrl, tag_list: *mut TagItem) -> u32 {
    ((*IAHI).AHI_ControlAudioA)(IAHI, ctrl, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_set_vol(
    channel: u16, volume: APTR, pan: APTR, ctrl: *mut AHIAudioCtrl, flags: u32,
) {
    ((*IAHI).AHI_SetVol)(IAHI, channel, volume, pan, ctrl, flags)
}

#[inline]
pub unsafe fn ahi_ahi_set_freq(channel: u16, freq: u32, ctrl: *mut AHIAudioCtrl, flags: u32) {
    ((*IAHI).AHI_SetFreq)(IAHI, channel, freq, ctrl, flags)
}

#[inline]
pub unsafe fn ahi_ahi_set_sound(
    channel: u16, sound: u16, offset: u32, length: i32, ctrl: *mut AHIAudioCtrl, flags: u32,
) {
    ((*IAHI).AHI_SetSound)(IAHI, channel, sound, offset, length, ctrl, flags)
}

#[inline]
pub unsafe fn ahi_ahi_set_effect(effect: APTR, ctrl: *mut AHIAudioCtrl) -> u32 {
    ((*IAHI).AHI_SetEffect)(IAHI, effect, ctrl)
}

#[inline]
pub unsafe fn ahi_ahi_load_sound(
    sound: u16, sound_type: u32, info: APTR, ctrl: *mut AHIAudioCtrl,
) -> u32 {
    ((*IAHI).AHI_LoadSound)(IAHI, sound, sound_type, info, ctrl)
}

#[inline]
pub unsafe fn ahi_ahi_unload_sound(sound: u16, ctrl: *mut AHIAudioCtrl) {
    ((*IAHI).AHI_UnloadSound)(IAHI, sound, ctrl)
}

#[inline]
pub unsafe fn ahi_ahi_next_audio_id(id: u32) -> u32 {
    ((*IAHI).AHI_NextAudioID)(IAHI, id)
}

#[inline]
pub unsafe fn ahi_ahi_get_audio_attrs_a(
    id: u32, ctrl: *mut AHIAudioCtrl, tag_list: *mut TagItem,
) -> u32 {
    ((*IAHI).AHI_GetAudioAttrsA)(IAHI, id, ctrl, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_best_audio_ida(tag_list: *mut TagItem) -> u32 {
    ((*IAHI).AHI_BestAudioIDA)(IAHI, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_alloc_audio_request_a(tag_list: *mut TagItem) -> *mut AHIAudioModeRequester {
    ((*IAHI).AHI_AllocAudioRequestA)(IAHI, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_audio_request_a(
    req: *mut AHIAudioModeRequester, tag_list: *mut TagItem,
) -> u32 {
    ((*IAHI).AHI_AudioRequestA)(IAHI, req, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_free_audio_request(req: *mut AHIAudioModeRequester) {
    ((*IAHI).AHI_FreeAudioRequest)(IAHI, req)
}

#[inline]
pub unsafe fn ahi_ahi_play_a(ctrl: *mut AHIAudioCtrl, tag_list: *mut TagItem) {
    ((*IAHI).AHI_PlayA)(IAHI, ctrl, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_sample_frame_size(sample_type: u32) -> u32 {
    ((*IAHI).AHI_SampleFrameSize)(IAHI, sample_type)
}

#[inline]
pub unsafe fn ahi_ahi_add_audio_mode(tag_list: *mut TagItem) -> u32 {
    ((*IAHI).AHI_AddAudioMode)(IAHI, tag_list)
}

#[inline]
pub unsafe fn ahi_ahi_remove_audio_mode(id: u32) -> u32 {
    ((*IAHI).AHI_RemoveAudioMode)(IAHI, id)
}

#[inline]
pub unsafe fn ahi_ahi_load_mode_file(filename: STRPTR) -> u32 {
    ((*IAHI).AHI_LoadModeFile)(IAHI, filename)
}
