//! IfslDMA global(s) and convenience wrappers.
//!
//! Hand-written binding for fsldma.resource — Freescale (PowerPC)
//! DMA controller used by the AmigaOne X1000/X5000 hardware for
//! memory-to-memory transfers and scatter/gather chains.
//!
//! Note: the SDK struct is `fslDMAIFace` (lowercase initial 'f' —
//! atypical), so the global is `IfslDMA`.

use crate::sys::types::*;
use crate::sys::interfaces::fsldma::*;

// ---- IfslDMA (fslDMAIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IfslDMA: *mut fslDMAIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IfslDMA: *mut fslDMAIFace = core::ptr::null_mut();

// ── Memory-to-memory copy ────────────────────────────────────

#[inline]
pub unsafe fn fsldma_copy_mem_dma(source: CONST_APTR, dest: APTR, size: u32) -> u32 {
    ((*IfslDMA).CopyMemDMA)(IfslDMA, source, dest, size)
}

#[inline]
pub unsafe fn fsldma_copy_mem_dmatag_list(
    source: CONST_APTR, dest: APTR, size: u32, tag_list: *const TagItem,
) -> u32 {
    ((*IfslDMA).CopyMemDMATagList)(IfslDMA, source, dest, size, tag_list)
}

// ── Scatter/gather DMA-descriptor chains ─────────────────────

#[inline]
pub unsafe fn fsldma_start_dmachain(chain: *mut dma_link_descriptor) -> u32 {
    ((*IfslDMA).StartDMAChain)(IfslDMA, chain)
}

#[inline]
pub unsafe fn fsldma_start_dmachain_tag_list(
    chain: *mut dma_link_descriptor, tag_list: *const TagItem,
) -> u32 {
    ((*IfslDMA).StartDMAChainTagList)(IfslDMA, chain, tag_list)
}
