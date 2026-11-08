//! IPASDMA global(s) and convenience wrappers.
//!
//! Hand-written binding for the PASemi DMA controller used by
//! AmigaOne X1000. Note: this IFace has no framework methods
//! (no Obtain/Release/Expunge/Clone) — every slot is a real
//! method. SDK filename is `pasemi_dma.h`.

use crate::sys::types::*;
use crate::sys::interfaces::pasdma::*;

// ---- IPASDMA (PASDMAIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IPASDMA: *mut PASDMAIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IPASDMA: *mut PASDMAIFace = core::ptr::null_mut();

// ── DMA register access ──────────────────────────────────────

#[inline]
pub unsafe fn pasdma_read_dmareg_byte(offset: u32) -> u8 {
    ((*IPASDMA).ReadDMARegByte)(IPASDMA, offset)
}

#[inline]
pub unsafe fn pasdma_read_dmareg_word(offset: u32) -> u16 {
    ((*IPASDMA).ReadDMARegWord)(IPASDMA, offset)
}

#[inline]
pub unsafe fn pasdma_read_dmareg_long(offset: u32) -> u32 {
    ((*IPASDMA).ReadDMARegLong)(IPASDMA, offset)
}

#[inline]
pub unsafe fn pasdma_write_dmareg_long(offset: u32, value: u32) {
    ((*IPASDMA).WriteDMARegLong)(IPASDMA, offset, value)
}

#[inline]
pub unsafe fn pasdma_write_iobreg_long(offset: u32, value: u32) {
    ((*IPASDMA).WriteIOBRegLong)(IPASDMA, offset, value)
}

// ── DMA channels ─────────────────────────────────────────────

#[inline]
pub unsafe fn pasdma_alloc_channel(owner: APTR, channel_type: u32) -> *mut PASDMAChannel {
    ((*IPASDMA).AllocChannel)(IPASDMA, owner, channel_type)
}

#[inline]
pub unsafe fn pasdma_free_channel(channel: *mut PASDMAChannel) {
    ((*IPASDMA).FreeChannel)(IPASDMA, channel)
}

#[inline]
pub unsafe fn pasdma_start_channel(channel: *mut PASDMAChannel, flags: u32) {
    ((*IPASDMA).StartChannel)(IPASDMA, channel, flags)
}

#[inline]
pub unsafe fn pasdma_stop_channel(channel: *mut PASDMAChannel) -> u32 {
    ((*IPASDMA).StopChannel)(IPASDMA, channel)
}

#[inline]
pub unsafe fn pasdma_alloc_ring(channel: *mut PASDMAChannel, size: u32) -> u32 {
    ((*IPASDMA).AllocRing)(IPASDMA, channel, size)
}

#[inline]
pub unsafe fn pasdma_free_ring(channel: *mut PASDMAChannel) {
    ((*IPASDMA).FreeRing)(IPASDMA, channel)
}

#[inline]
pub unsafe fn pasdma_alloc_buffer(size: u32) -> APTR {
    ((*IPASDMA).AllocBuffer)(IPASDMA, size)
}

#[inline]
pub unsafe fn pasdma_free_buffer(buffer: APTR) {
    ((*IPASDMA).FreeBuffer)(IPASDMA, buffer)
}

// ── Events ───────────────────────────────────────────────────

#[inline]
pub unsafe fn pasdma_alloc_event() -> i8 {
    ((*IPASDMA).AllocEvent)(IPASDMA)
}

#[inline]
pub unsafe fn pasdma_free_event(event: u8) {
    ((*IPASDMA).FreeEvent)(IPASDMA, event)
}

#[inline]
pub unsafe fn pasdma_set_event(event: u8) {
    ((*IPASDMA).SetEvent)(IPASDMA, event)
}

#[inline]
pub unsafe fn pasdma_clear_event(event: u8) {
    ((*IPASDMA).ClearEvent)(IPASDMA, event)
}

// ── Functions ────────────────────────────────────────────────

#[inline]
pub unsafe fn pasdma_alloc_function() -> i8 {
    ((*IPASDMA).AllocFunction)(IPASDMA)
}

#[inline]
pub unsafe fn pasdma_free_function(func: u8) {
    ((*IPASDMA).FreeFunction)(IPASDMA, func)
}
