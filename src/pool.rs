//! exec's allocators beyond AllocVec: memory pools, item pools and
//! resource maps (AllocSysObject ASOT_MEMPOOL, ASOT_ITEMPOOL, ASOT_RMAP).
//!
//! - [`MemPool`]: variable-size blocks from puddles (AllocVecPooled). Made
//!   with ASOPOOL_Protected, so tasks can share it; a [`PoolBlock`] is
//!   zeroed when handed out and freed on drop.
//! - [`ItemPool<T>`]: fixed-size items for one type, made protected, a
//!   [`PoolItem`] owning a `T` (dropped and returned to the pool on drop).
//!   Good for a driver's per-request state.
//! - [`ResourceMap`]: hands out ranges of an abstract number space (I/O
//!   addresses, IDs); a [`ResourceRange`] is returned on drop. Not `Sync`:
//!   exec does not arbitrate a map.
//!
//! Blocks and items borrow their pool, so a pool outlives them.

use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::error::{AmigaError, Result};
use crate::sys::{self, TagItem, APTR, TAG_DONE};

fn alloc_object(kind: u32, tags: &[TagItem]) -> Result<APTR> {
    // SAFETY: a TAG_DONE-ended tag list.
    let p = unsafe { sys::exec_alloc_sys_object(kind, tags.as_ptr()) };
    if p.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(p) }
}

// ---------------------------------------------------------------------------
// MemPool
// ---------------------------------------------------------------------------

/// A protected exec memory pool.
pub struct MemPool {
    pool: APTR,
}

// SAFETY: the pool is made with ASOPOOL_Protected: exec locks it on every
// allocation and free.
unsafe impl Send for MemPool {}
// SAFETY: as above.
unsafe impl Sync for MemPool {}

impl MemPool {
    /// `mem_flags` MEMF_* (MEMF_SHARED for memory other tasks touch),
    /// `puddle` and `threshold` in bytes (0: exec's defaults).
    pub fn new(mem_flags: u32, puddle: u32, threshold: u32) -> Result<Self> {
        let pool = alloc_object(
            sys::ASOT_MEMPOOL,
            &[
                TagItem { ti_Tag: sys::ASOPOOL_MFLAGS, ti_Data: mem_flags },
                TagItem { ti_Tag: sys::ASOPOOL_PUDDLE, ti_Data: puddle },
                TagItem { ti_Tag: sys::ASOPOOL_THRESHOLD, ti_Data: threshold },
                TagItem { ti_Tag: sys::ASOPOOL_PROTECTED, ti_Data: 1 },
                TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
            ],
        )?;
        Ok(Self { pool })
    }

    /// A zeroed block of `size` bytes, or None when the pool cannot grow.
    pub fn alloc(&self, size: usize) -> Option<PoolBlock<'_>> {
        if size == 0 || size > u32::MAX as usize {
            return None;
        }
        // SAFETY: our pool.
        let p = unsafe { sys::exec_alloc_vec_pooled(self.pool, size as u32) } as *mut u8;
        let p = NonNull::new(p)?;
        // SAFETY: a fresh block of `size` bytes, zeroed before it is shown
        // as a slice.
        unsafe { core::ptr::write_bytes(p.as_ptr(), 0, size) };
        Some(PoolBlock { p, len: size, pool: self })
    }
}

impl Drop for MemPool {
    fn drop(&mut self) {
        // SAFETY: our pool; no block outlives it (blocks borrow it).
        unsafe { sys::exec_free_sys_object(sys::ASOT_MEMPOOL, self.pool) }
    }
}

/// A block from a [`MemPool`].
pub struct PoolBlock<'a> {
    p: NonNull<u8>,
    len: usize,
    pool: &'a MemPool,
}

// SAFETY: the block is plain memory owned by this value; the pool is Sync.
unsafe impl Send for PoolBlock<'_> {}
// SAFETY: as above.
unsafe impl Sync for PoolBlock<'_> {}

impl Deref for PoolBlock<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        // SAFETY: our initialised block of `len` bytes.
        unsafe { core::slice::from_raw_parts(self.p.as_ptr(), self.len) }
    }
}

impl DerefMut for PoolBlock<'_> {
    fn deref_mut(&mut self) -> &mut [u8] {
        // SAFETY: as above, borrowed mutably.
        unsafe { core::slice::from_raw_parts_mut(self.p.as_ptr(), self.len) }
    }
}

impl Drop for PoolBlock<'_> {
    fn drop(&mut self) {
        // SAFETY: a block of this pool from AllocVecPooled, freed once.
        unsafe { sys::exec_free_vec_pooled(self.pool.pool, self.p.as_ptr() as APTR) }
    }
}

// ---------------------------------------------------------------------------
// ItemPool
// ---------------------------------------------------------------------------

/// A protected pool of `T`-sized items.
pub struct ItemPool<T> {
    pool: APTR,
    _t: PhantomData<T>,
}

// SAFETY: protected (ASOITEM_Protected): exec locks it; items carry T, so
// sharing the pool between tasks moves T between them (T: Send).
unsafe impl<T: Send> Send for ItemPool<T> {}
// SAFETY: as above.
unsafe impl<T: Send> Sync for ItemPool<T> {}

impl<T> ItemPool<T> {
    /// A pool in MEMF_SHARED memory with no upper limit.
    pub fn new() -> Result<Self> {
        Self::with_limit(0, 0)
    }

    /// At most `max_items` items (0: no limit), allocated `batch` at a time
    /// (0: exec's choice; `max_items` must be a multiple of it).
    pub fn with_limit(max_items: u32, batch: u32) -> Result<Self> {
        assert!(align_of::<T>() <= 8, "ItemPool<T>: T is aligned to more than 8 bytes");
        let size = size_of::<T>().max(8) as u32;
        let mut tags = alloc::vec![
            TagItem { ti_Tag: sys::ASOITEM_MFLAGS, ti_Data: sys::MEMF_SHARED },
            TagItem { ti_Tag: sys::ASOITEM_ITEMSIZE, ti_Data: size },
            TagItem { ti_Tag: sys::ASOITEM_PROTECTED, ti_Data: 1 },
        ];
        if batch != 0 {
            tags.push(TagItem { ti_Tag: sys::ASOITEM_BATCHITEMS, ti_Data: batch });
        }
        if max_items != 0 {
            tags.push(TagItem { ti_Tag: sys::ASOITEM_MAXITEMS, ti_Data: max_items });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        let pool = alloc_object(sys::ASOT_ITEMPOOL, &tags)?;
        Ok(Self { pool, _t: PhantomData })
    }

    /// An item holding `value`; `Err(value)` when the pool is full or out
    /// of memory.
    pub fn alloc(&self, value: T) -> core::result::Result<PoolItem<'_, T>, T> {
        // SAFETY: our pool.
        let p = unsafe { sys::exec_item_pool_alloc(self.pool) } as *mut T;
        let Some(p) = NonNull::new(p) else { return Err(value) };
        if (p.as_ptr() as usize) % align_of::<T>() != 0 {
            // SAFETY: the item is ours and unused.
            unsafe { sys::exec_item_pool_free(self.pool, p.as_ptr() as APTR) };
            return Err(value);
        }
        // SAFETY: a fresh item of at least size_of::<T>() bytes, aligned.
        unsafe { p.as_ptr().write(value) };
        Ok(PoolItem { p, pool: self })
    }

    /// Give unused memory back (ItemPoolGC).
    pub fn gc(&self) {
        // SAFETY: our pool.
        unsafe { sys::exec_item_pool_gc(self.pool) }
    }
}

impl<T> Drop for ItemPool<T> {
    fn drop(&mut self) {
        // SAFETY: our pool; no item outlives it (items borrow it).
        unsafe { sys::exec_free_sys_object(sys::ASOT_ITEMPOOL, self.pool) }
    }
}

/// A `T` in an [`ItemPool`] item.
pub struct PoolItem<'a, T> {
    p: NonNull<T>,
    pool: &'a ItemPool<T>,
}

// SAFETY: the item owns its T; the pool is protected.
unsafe impl<T: Send> Send for PoolItem<'_, T> {}
// SAFETY: shared access gives &T only.
unsafe impl<T: Sync> Sync for PoolItem<'_, T> {}

impl<T> Deref for PoolItem<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: an initialised T owned by this item.
        unsafe { self.p.as_ref() }
    }
}

impl<T> DerefMut for PoolItem<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as above, borrowed mutably.
        unsafe { self.p.as_mut() }
    }
}

impl<T> Drop for PoolItem<'_, T> {
    fn drop(&mut self) {
        // SAFETY: drop the T once, then return the item to its pool.
        unsafe {
            core::ptr::drop_in_place(self.p.as_ptr());
            sys::exec_item_pool_free(self.pool.pool, self.p.as_ptr() as APTR);
        }
    }
}

// ---------------------------------------------------------------------------
// ResourceMap
// ---------------------------------------------------------------------------

/// Ranges of `base..base + size`, handed out in multiples of a quantum.
pub struct ResourceMap {
    map: APTR,
    _not_sync: PhantomData<core::cell::Cell<()>>,
}

// SAFETY: a map may move between tasks; it is just not shared (not Sync).
unsafe impl Send for ResourceMap {}

impl ResourceMap {
    /// `base` must not be 0 (0 is how exec reports a failed allocation).
    /// `quantum` 0 is exec's default (MEM_BLOCKSIZE).
    pub fn new(base: u32, size: u32, quantum: u32) -> Result<Self> {
        if base == 0 || size == 0 || base.checked_add(size).is_none() {
            return Err(AmigaError::Device("ResourceMap: base must be non-zero and base + size fit 32 bits"));
        }
        let mut tags = alloc::vec![
            TagItem { ti_Tag: sys::ASORMAP_BASE, ti_Data: base },
            TagItem { ti_Tag: sys::ASORMAP_SIZE, ti_Data: size },
        ];
        if quantum != 0 {
            tags.push(TagItem { ti_Tag: sys::ASORMAP_QUANTUM, ti_Data: quantum });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        Ok(Self { map: alloc_object(sys::ASOT_RMAP, &tags)?, _not_sync: PhantomData })
    }

    /// A range of at least `size`, or None.
    pub fn alloc(&self, size: u32) -> Option<ResourceRange<'_>> {
        // SAFETY: our map.
        let start = unsafe { sys::exec_rmap_alloc(self.map, size, 0) } as usize as u32;
        (start != 0).then_some(ResourceRange { start, size, aligned: false, map: self })
    }

    /// A range of at least `size` starting at a multiple of `align`.
    pub fn alloc_aligned(&self, size: u32, align: u32) -> Option<ResourceRange<'_>> {
        // SAFETY: our map.
        let start = unsafe { sys::exec_rmap_ext_alloc(self.map, size, align, 0) } as usize as u32;
        (start != 0).then_some(ResourceRange { start, size, aligned: true, map: self })
    }
}

impl Drop for ResourceMap {
    fn drop(&mut self) {
        // SAFETY: our map; no range outlives it.
        unsafe { sys::exec_free_sys_object(sys::ASOT_RMAP, self.map) }
    }
}

/// A range from a [`ResourceMap`], returned on drop.
pub struct ResourceRange<'a> {
    start: u32,
    size: u32,
    aligned: bool,
    map: &'a ResourceMap,
}

impl ResourceRange<'_> {
    pub fn start(&self) -> u32 {
        self.start
    }

    pub fn size(&self) -> u32 {
        self.size
    }
}

impl Drop for ResourceRange<'_> {
    fn drop(&mut self) {
        let a = self.start as usize as APTR;
        // SAFETY: the whole range as allocated, with the matching free.
        unsafe {
            if self.aligned {
                sys::exec_rmap_ext_free(self.map.map, a, self.size)
            } else {
                sys::exec_rmap_free(self.map.map, a, self.size)
            }
        }
    }
}
