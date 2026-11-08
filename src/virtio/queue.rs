//! A virtqueue: ring memory, its DMA mapping, and the ring over it.

use super::ring::{Endian, RingLayout, SplitRing, LEGACY_ALIGN};
use crate::error::{AmigaError, Result};
use crate::mem::{DmaBuffer, DmaDirection, DmaMapping};

/// A split virtqueue in one physically contiguous, 4096-aligned block,
/// mapped for the device for as long as the queue lives (the device reads
/// and writes it at any time, so it is never unmapped while in use).
///
/// The legacy interface gives the device one page number for the whole
/// block, so it must be one physical block; the modern interface takes the
/// three areas' addresses separately and accepts the same layout.
pub struct Virtqueue {
    // Field order is drop order: the mapping ends (EndDMA) before the
    // memory under it is freed.
    _map: DmaMapping<'static>,
    _mem: DmaBuffer,
    ring: SplitRing,
    phys: u32,
}

impl Virtqueue {
    /// Allocate and map a queue of `size` entries (a power of two) with
    /// fields in `endian` order: [`Endian::Native`] for a legacy device,
    /// [`Endian::Little`] for a modern one.
    ///
    /// # Errors
    /// `AllocationFailed` for a bad size or no memory; `DmaFailed` if the
    /// block does not map to one 4096-aligned physical block.
    pub fn new(size: u16, endian: Endian) -> Result<Self> {
        let layout = RingLayout::legacy(size).ok_or(AmigaError::AllocationFailed)?;
        let mut mem = DmaBuffer::alloc_contiguous(layout.total, LEGACY_ALIGN as u32)?;
        let base = mem.as_mut_ptr();
        // SAFETY: `mem` is kept alongside the mapping and outlives it (field
        // order), and the CPU's accesses to it go through the ring, which
        // flushes and invalidates around the device's.
        let map = unsafe { DmaMapping::new(base, layout.total, DmaDirection::ToDevice)? };
        let e = map.entries();
        if e.len() != 1 || (e[0].PhysicalAddress as usize) % LEGACY_ALIGN != 0 {
            return Err(AmigaError::DmaFailed);
        }
        let phys = e[0].PhysicalAddress as u32;
        // SAFETY: `base` covers layout.total zeroed bytes, owned by `mem`.
        let ring = unsafe { SplitRing::new(base, layout, endian) };
        Ok(Self { _map: map, _mem: mem, ring, phys })
    }

    /// The ring, for filling descriptors and taking used entries.
    #[inline]
    pub fn ring(&mut self) -> &mut SplitRing {
        &mut self.ring
    }

    /// The layout.
    #[inline]
    pub fn layout(&self) -> RingLayout {
        self.ring.layout()
    }

    /// Physical address of the descriptor table (the whole block).
    #[inline]
    pub fn desc_phys(&self) -> u32 {
        self.phys
    }

    /// Physical address of the avail (driver) ring.
    #[inline]
    pub fn avail_phys(&self) -> u32 {
        self.phys + self.layout().avail_off as u32
    }

    /// Physical address of the used (device) ring.
    #[inline]
    pub fn used_phys(&self) -> u32 {
        self.phys + self.layout().used_off as u32
    }
}
