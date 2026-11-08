//! RAII memory allocation via AmigaOS `AllocVecTagList`/`FreeVec`.
//!
//! [`AmigaVec`] provides a safe, auto-freeing wrapper around OS memory
//! allocation. Use this for buffers that need specific memory type flags
//! (e.g. `MEMF_SHARED` for DMA) rather than the global allocator.

use core::marker::PhantomData;
use crate::error::{AmigaError, Result};
use crate::tag::TagListBuilder;
use crate::sys::{APTR, AVT_TYPE, AVT_CLEAR_WITH_VALUE};

/// RAII wrapper around AllocVecTagList / FreeVec.
///
/// Automatically frees the allocation on drop.
pub struct AmigaVec {
    ptr: *mut u8,
    len: usize,
}

impl AmigaVec {
    /// Allocate `size` bytes with the given memory type flags.
    pub fn alloc(size: usize, mem_type: u32) -> Result<Self> {
        if size > u32::MAX as usize {
            return Err(AmigaError::AllocationFailed);
        }
        let tags = TagListBuilder::new()
            .tag(AVT_TYPE, mem_type)
            .build();
        // SAFETY: a TAG_DONE-ended tag list; null is handled below.
        let ptr = unsafe {
            crate::sys::exec_alloc_vec_tag_list(size as u32, tags.as_ptr())
        };
        if ptr.is_null() {
            Err(AmigaError::AllocationFailed)
        } else {
            Ok(Self { ptr: ptr as *mut u8, len: size })
        }
    }

    /// Allocate `size` bytes, cleared to `fill` value.
    pub fn alloc_cleared(size: usize, mem_type: u32, fill: u8) -> Result<Self> {
        if size > u32::MAX as usize {
            return Err(AmigaError::AllocationFailed);
        }
        let tags = TagListBuilder::new()
            .tag(AVT_TYPE, mem_type)
            .tag(AVT_CLEAR_WITH_VALUE, fill as u32)
            .build();
        // SAFETY: a TAG_DONE-ended tag list; null is handled below.
        let ptr = unsafe {
            crate::sys::exec_alloc_vec_tag_list(size as u32, tags.as_ptr())
        };
        if ptr.is_null() {
            Err(AmigaError::AllocationFailed)
        } else {
            Ok(Self { ptr: ptr as *mut u8, len: size })
        }
    }

    /// Get a raw pointer to the allocated memory.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get a mutable raw pointer to the allocated memory.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Return the size of the allocation in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return `true` if the allocation has zero length.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// View the allocation as a byte slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: our block of `len` bytes, alive while `self` is.
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// View the allocation as a mutable byte slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: as above, borrowed mutably through `&mut self`.
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for AmigaVec {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: our AllocVecTagList block, freed once.
            unsafe { crate::sys::exec_free_vec(self.ptr as APTR) }
        }
    }
}

/// Which way a DMA transfer moves data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// The device reads the buffer (RAM to device): `DMAF_ReadFromRAM`.
    ToDevice,
    /// The device writes the buffer (device to RAM).
    FromDevice,
}

impl DmaDirection {
    #[inline]
    fn flags(self) -> u32 {
        match self {
            DmaDirection::ToDevice => crate::sys::DMAF_READ_FROM_RAM,
            DmaDirection::FromDevice => 0,
        }
    }
}

/// A `MEMF_SHARED` buffer for device DMA.
///
/// The CPU fills or reads the buffer through [`as_slice`](Self::as_slice) and
/// [`as_mut_slice`](Self::as_mut_slice). To hand it to a device, call
/// [`map`](Self::map), which runs exec's `StartDMA`/`GetDMAList` and returns a
/// [`DmaMapping`] holding the buffer's physical blocks. The mapping borrows the
/// buffer mutably, so the CPU cannot touch it while the device owns it, and
/// dropping the mapping runs `EndDMA`.
///
/// This is the route the exec autodocs require (SDK 54.25, `exec.doc`
/// StartDMA): it flushes or invalidates the caches correctly on cache
/// non-coherent machines such as the Sam460, and it gives the device physical
/// addresses, which can differ from the CPU's virtual ones and can be split
/// into several blocks.
///
/// ```ignore
/// let mut buf = DmaBuffer::alloc(4096)?;
/// buf.as_mut_slice()[..len].copy_from_slice(data);
/// {
///     let map = buf.map(DmaDirection::ToDevice)?;
///     for e in map.entries() {
///         // program one descriptor per block:
///         // e.PhysicalAddress, e.BlockLength
///     }
///     // ... start the transfer and wait for it to finish ...
/// } // EndDMA here
/// ```
pub struct DmaBuffer {
    mem: AmigaVec,
}

impl DmaBuffer {
    /// Allocate `size` bytes of zeroed `MEMF_SHARED` memory.
    pub fn alloc(size: usize) -> Result<Self> {
        Ok(Self {
            mem: AmigaVec::alloc_cleared(size, crate::sys::MEMF_SHARED, 0)?,
        })
    }

    /// Allocate `size` bytes of zeroed `MEMF_SHARED` memory that is one
    /// physically contiguous block, aligned to `align` bytes both virtually
    /// and physically (`AVT_Contiguous`, `AVT_Alignment`,
    /// `AVT_PhysicalAlignment`). For structures a device addresses by one
    /// base address, such as a virtio queue. `align` must be a power of two.
    pub fn alloc_contiguous(size: usize, align: u32) -> Result<Self> {
        use crate::sys::{
            AVT_ALIGNMENT, AVT_CONTIGUOUS, AVT_PHYSICALALIGNMENT, MEMF_SHARED, TAG_DONE, TagItem,
        };
        if size == 0 || size > u32::MAX as usize || !align.is_power_of_two() {
            return Err(AmigaError::AllocationFailed);
        }
        let tags = [
            TagItem { ti_Tag: AVT_TYPE, ti_Data: MEMF_SHARED },
            TagItem { ti_Tag: AVT_CONTIGUOUS, ti_Data: 1 },
            TagItem { ti_Tag: AVT_ALIGNMENT, ti_Data: align },
            TagItem { ti_Tag: AVT_PHYSICALALIGNMENT, ti_Data: align },
            TagItem { ti_Tag: AVT_CLEAR_WITH_VALUE, ti_Data: 0 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a valid TAG_DONE-terminated list on the stack.
        let ptr = unsafe { crate::sys::exec_alloc_vec_tag_list(size as u32, tags.as_ptr()) };
        if ptr.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { mem: AmigaVec { ptr: ptr as *mut u8, len: size } })
    }

    /// The buffer's CPU address for code that keeps a long-lived mapping
    /// with [`DmaMapping::new`] and reads or writes the buffer while the
    /// device owns parts of it (a virtio queue). Prefer [`map`](Self::map)
    /// for one-shot transfers.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.mem.as_mut_ptr()
    }

    /// Prepare the whole buffer for a DMA transfer in direction `dir`.
    ///
    /// The returned mapping lists the physical blocks to give the device.
    /// Keep it alive until the transfer has finished; dropping it ends the DMA.
    pub fn map(&mut self, dir: DmaDirection) -> Result<DmaMapping<'_>> {
        let len = self.mem.len();
        // SAFETY: the pointer and length describe this allocation, and the
        // mapping's lifetime is tied to the &mut borrow of `self`.
        unsafe { DmaMapping::new(self.mem.as_mut_ptr(), len, dir) }
    }

    /// The buffer as a byte slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.mem.as_slice()
    }

    /// The buffer as a mutable byte slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.mem.as_mut_slice()
    }

    /// The CPU's (virtual) address of the buffer. Never give this to a
    /// device; use the physical blocks from [`map`](Self::map).
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.mem.as_ptr()
    }

    /// Buffer length in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.mem.len()
    }

    /// True if the buffer has zero length.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.mem.is_empty()
    }
}

/// A memory region prepared for DMA with `StartDMA`, and its physical layout
/// from `GetDMAList`.
///
/// Dropping it frees the entry list and calls `EndDMA` with the same address,
/// size and flags, as the autodoc requires for every successful `StartDMA`.
pub struct DmaMapping<'a> {
    addr: *mut u8,
    len: u32,
    flags: u32,
    end_flags: u32,
    list: *mut crate::sys::DMAEntry,
    count: u32,
    _region: core::marker::PhantomData<&'a mut [u8]>,
}

impl<'a> DmaMapping<'a> {
    /// Prepare any memory region for DMA, for example a caller's `io_Data`.
    ///
    /// # Safety
    /// `addr` must point to `len` bytes that stay allocated, and that the CPU
    /// does not access, until the mapping is dropped.
    pub unsafe fn new(addr: *mut u8, len: usize, dir: DmaDirection) -> Result<Self> {
        use crate::sys::{
            TagItem, ASODMAE_NUM_ENTRIES, ASOT_DMAENTRY, DMAF_NO_MODIFY, TAG_DONE,
        };

        if len == 0 || len > u32::MAX as usize {
            return Err(AmigaError::DmaFailed);
        }
        let len = len as u32;
        let flags = dir.flags();

        // SAFETY: `addr` is `len` bytes that stay allocated (the caller's
        // contract).
        let count = unsafe { crate::sys::exec_start_dma(addr as _, len, flags) };
        if count == 0 {
            return Err(AmigaError::DmaFailed);
        }

        let tags = [
            TagItem { ti_Tag: ASODMAE_NUM_ENTRIES, ti_Data: count },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list.
        let list = unsafe { crate::sys::exec_alloc_sys_object(ASOT_DMAENTRY, tags.as_ptr()) }
            as *mut crate::sys::DMAEntry;
        if list.is_null() {
            // The transfer will not happen: still pair StartDMA with EndDMA.
            // SAFETY: the region StartDMA accepted, with its flags.
            unsafe { crate::sys::exec_end_dma(addr as _, len, flags | DMAF_NO_MODIFY) };
            return Err(AmigaError::AllocationFailed);
        }
        // SAFETY: the region StartDMA accepted, and a list of `count` entries.
        unsafe { crate::sys::exec_get_dmalist(addr as _, len, flags, list) };

        Ok(Self {
            addr,
            len,
            flags,
            end_flags: 0,
            list,
            count,
            _region: core::marker::PhantomData,
        })
    }

    /// The physical blocks, in ascending virtual address order. Their
    /// lengths add up to the mapped size.
    #[inline]
    pub fn entries(&self) -> &[crate::sys::DMAEntry] {
        // SAFETY: GetDMAList filled `count` entries into `list`.
        unsafe { core::slice::from_raw_parts(self.list, self.count as usize) }
    }

    /// End the mapping for a transfer that was never started or was aborted
    /// before the device wrote anything (`DMAF_NoModify`).
    pub fn abort(mut self) {
        self.end_flags |= crate::sys::DMAF_NO_MODIFY;
    }
}

impl Drop for DmaMapping<'_> {
    fn drop(&mut self) {
        // SAFETY: `list` came from AllocSysObject(ASOT_DMAENTRY), and the
        // address, size and flags are the ones given to StartDMA.
        unsafe {
            crate::sys::exec_free_sys_object(crate::sys::ASOT_DMAENTRY, self.list as APTR);
            crate::sys::exec_end_dma(self.addr as _, self.len, self.flags | self.end_flags);
        }
    }
}

/// An exec memory pool (`CreatePool`/`AllocPooled`). Sub-allocations are freed
/// ATOMICALLY when the pool is dropped (`DeletePool` frees the whole pool at
/// once, even allocations you never freed individually) - useful for a unit of
/// work whose memory should all go away together (e.g. a contained subsystem).
///
/// Automatically deletes the pool on drop.
pub struct AmigaPool {
    pool: APTR,
}

impl AmigaPool {
    /// Create a pool. `mem_type` is the `MEMF_*` requirements, `puddle` the bytes
    /// allocated per puddle, and `thresh` the largest single allocation served
    /// from puddles (larger allocations get their own dedicated block). `thresh`
    /// must be <= `puddle`.
    pub fn new(mem_type: u32, puddle: u32, thresh: u32) -> Result<Self> {
        // SAFETY: plain values; null is handled below.
        let pool = unsafe { crate::sys::exec_create_pool(mem_type, puddle, thresh) };
        if pool.is_null() {
            Err(AmigaError::AllocationFailed)
        } else {
            Ok(Self { pool })
        }
    }

    /// Allocate `size` bytes from the pool. The memory is freed en masse when the
    /// pool is dropped (or individually via [`AmigaPool::free`]). Returns a null
    /// pointer on failure.
    pub fn alloc(&self, size: usize) -> *mut u8 {
        if size > u32::MAX as usize {
            return core::ptr::null_mut();
        }
        // SAFETY: our pool, alive while `self` is.
        unsafe { crate::sys::exec_alloc_pooled(self.pool, size as u32) as *mut u8 }
    }

    /// Free a single allocation back to the pool (optional - the pool frees
    /// everything on drop). `size` must match the original `alloc`.
    ///
    /// # Safety
    /// `ptr` must have come from this pool's `alloc` with the same `size`.
    pub unsafe fn free(&self, ptr: *mut u8, size: usize) {
        if !ptr.is_null() && size <= u32::MAX as usize {
            // SAFETY: the caller's contract: this pool's block of `size`.
            unsafe { crate::sys::exec_free_pooled(self.pool, ptr as APTR, size as u32) };
        }
    }

    /// The raw pool header.
    pub fn as_ptr(&self) -> APTR {
        self.pool
    }
}

impl Drop for AmigaPool {
    fn drop(&mut self) {
        if !self.pool.is_null() {
            // SAFETY: our pool, deleted once; no allocation outlives `self`
            // through safe code (alloc hands out raw pointers).
            unsafe { crate::sys::exec_delete_pool(self.pool) }
        }
    }
}

// ---------------------------------------------------------------------------
// Queries, copies and locked memory
// ---------------------------------------------------------------------------

/// Free memory of the kind `flags` describes (AvailMem: MEMF_ANY,
/// MEMF_SHARED, MEMF_LARGEST, MEMF_TOTAL, ...).
pub fn avail(flags: u32) -> u32 {
    // SAFETY: AvailMem only reads exec's lists.
    unsafe { crate::sys::exec_avail_mem(flags) }
}

/// The MEMF_ attributes of the memory `value` lives in (TypeOfMem); 0 for
/// memory exec does not manage (a stack may be such memory).
pub fn type_of<T: ?Sized>(value: &T) -> u32 {
    // SAFETY: TypeOfMem only looks the address up.
    unsafe { crate::sys::exec_type_of_mem(value as *const T as *const u8 as _) }
}

/// Copy `src` into `dst` with exec's CopyMem. Panics when the lengths
/// differ (as `copy_from_slice` does).
pub fn copy(src: &[u8], dst: &mut [u8]) {
    assert_eq!(src.len(), dst.len(), "mem::copy: source and destination lengths differ");
    if src.is_empty() {
        return;
    }
    // SAFETY: two valid, non-overlapping slices (one is &mut) of equal length.
    unsafe { crate::sys::exec_copy_mem(src.as_ptr() as _, dst.as_mut_ptr() as APTR, src.len() as u32) }
}

/// Copy `src` into `dst` with CopyMemQuick (longword aligned, a whole
/// number of longwords: what `[u32]` guarantees). Panics when the lengths
/// differ.
pub fn copy_quick(src: &[u32], dst: &mut [u32]) {
    assert_eq!(src.len(), dst.len(), "mem::copy_quick: source and destination lengths differ");
    if src.is_empty() {
        return;
    }
    // SAFETY: as for copy, and both are u32-aligned, size a multiple of 4.
    unsafe { crate::sys::exec_copy_mem_quick(src.as_ptr() as _, dst.as_mut_ptr() as APTR, (src.len() * 4) as u32) }
}

/// Memory locked in physical RAM (LockMem) while this lives: it cannot be
/// paged out. Locks nest; each one is undone on drop (UnlockMem).
pub struct MemLock<'a> {
    addr: *mut u8,
    len: u32,
    _mem: PhantomData<&'a [u8]>,
}

impl<'a> MemLock<'a> {
    /// Lock `mem`; None when exec refuses.
    pub fn new(mem: &'a [u8]) -> Option<Self> {
        if mem.is_empty() || mem.len() > u32::MAX as usize {
            return None;
        }
        let (addr, len) = (mem.as_ptr() as *mut u8, mem.len() as u32);
        // SAFETY: a valid range, borrowed for the lock's life.
        (unsafe { crate::sys::exec_lock_mem(addr as APTR, len) } != 0).then_some(Self { addr, len, _mem: PhantomData })
    }
}

impl Drop for MemLock<'_> {
    fn drop(&mut self) {
        // SAFETY: the range this value locked, unlocked once.
        unsafe { crate::sys::exec_unlock_mem(self.addr as APTR, self.len) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    #[test]
    fn amiga_vec_struct_size() {
        // AmigaVec contains a *mut u8 and a usize
        assert_eq!(
            mem::size_of::<AmigaVec>(),
            mem::size_of::<*mut u8>() + mem::size_of::<usize>()
        );
    }
}
