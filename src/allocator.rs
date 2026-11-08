//! Driver-mode allocators backed by IExec->AllocVecTagList / FreeVec.
//!
//! No clib4 dependency. Works with `-nostartfiles -nodefaultlibs -lgcc`.
//!
//! IMPORTANT: IExec must be set before any allocation occurs.
//! The C startup glue (driver_glue.c) sets IExec in _start() before
//! calling rust_handler_main(), so all Rust code sees a valid IExec.
//!
//! Two of them, differing only in the memory type:
//!
//! - [`ExecAllocator`]: MEMF_PRIVATE, "memory that is only visible to the
//!   allocator task" (exec/memory.h). For a program whose heap one task uses.
//! - [`SharedExecAllocator`]: MEMF_SHARED, visible to all tasks. For a
//!   device, library or handler: its heap is used by its own tasks and by
//!   every caller (BeginIO, Open and Close run in the caller's task, and may
//!   allocate there), so it must not be one task's private memory.
//!   virtio-scsi's Kickstart build crashed DOS with MEMF_PRIVATE: a caller's
//!   process corrupted its stack base on exit.
//!
//! For DMA buffers use `mem::DmaBuffer`, which allocates MEMF_SHARED memory.

use core::alloc::{GlobalAlloc, Layout};
use crate::sys::{IExec, TagItem, TAG_DONE, AVT_TYPE, MEMF_PRIVATE, MEMF_SHARED};

/// Global allocator backed by Exec's `AllocVecTagList`/`FreeVec`, in
/// MEMF_PRIVATE memory: for programs (one task uses the heap).
///
/// IExec must be initialized before any allocation.
pub struct ExecAllocator;

/// The same in MEMF_SHARED memory: for devices, libraries and handlers,
/// whose heap other tasks use.
pub struct SharedExecAllocator;

/// Allocate `layout` in `mem_type` memory.
///
/// # Safety
/// IExec is null (then this returns null) or a valid exec interface.
unsafe fn alloc_typed(mem_type: u32, layout: Layout) -> *mut u8 {
    // SAFETY: reading the interface global (a plain pointer read).
    let iexec = unsafe { IExec };
    if iexec.is_null() {
        return core::ptr::null_mut();
    }
    let tags = [
        TagItem { ti_Tag: AVT_TYPE, ti_Data: mem_type },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];

    // AllocVecTagList returns memory aligned to at least 16 bytes.
    // For most Rust types this is sufficient.
    if layout.align() <= 16 {
        // SAFETY: a valid interface (the caller's contract) and tag list.
        unsafe { ((*iexec).AllocVecTagList)(iexec, layout.size() as u32, tags.as_ptr()) as *mut u8 }
    } else {
        // Over-allocate for unusual alignment (>16 bytes).
        // Store the original pointer just before the aligned region.
        let overhead = layout.align() + core::mem::size_of::<usize>();
        let total = match layout.size().checked_add(overhead) {
            Some(t) => t,
            None => return core::ptr::null_mut(),
        };
        // SAFETY: as above.
        let raw = unsafe { ((*iexec).AllocVecTagList)(iexec, total as u32, tags.as_ptr()) as *mut u8 };
        if raw.is_null() {
            return raw;
        }
        // Align: skip past the space for the stored pointer.
        // SAFETY: `raw` has `total` > size_of::<usize>() bytes.
        let base = unsafe { raw.add(core::mem::size_of::<usize>()) };
        let aligned = ((base as usize + layout.align() - 1) & !(layout.align() - 1)) as *mut u8;
        // Store the original pointer just before the aligned address.
        // SAFETY: `aligned` is at least size_of::<usize>() past `raw` and
        // inside the block, and the word before it is in the block too;
        // usize alignment holds since `aligned` is aligned to > 16.
        unsafe {
            let store = aligned.sub(core::mem::size_of::<usize>()) as *mut usize;
            *store = raw as usize;
        }
        aligned
    }
}

/// Free what [`alloc_typed`] returned for `layout`.
///
/// # Safety
/// `ptr` is null or came from `alloc_typed` with this `layout`.
unsafe fn dealloc_typed(ptr: *mut u8, layout: Layout) {
    // SAFETY: reading the interface global.
    let iexec = unsafe { IExec };
    if iexec.is_null() || ptr.is_null() {
        return;
    }

    if layout.align() <= 16 {
        // SAFETY: `ptr` is an AllocVecTagList block (the caller's contract).
        unsafe {
            ((*iexec).FreeVec)(iexec, ptr as *mut core::ffi::c_void);
        }
    } else {
        // Recover the original pointer stored before the aligned address.
        // SAFETY: alloc_typed stored it in the word before `ptr`.
        let original = unsafe {
            let store = ptr.sub(core::mem::size_of::<usize>()) as *const usize;
            *store as *mut core::ffi::c_void
        };
        // SAFETY: `original` is the AllocVecTagList block.
        unsafe {
            ((*iexec).FreeVec)(iexec, original);
        }
    }
}

// SAFETY: alloc_typed returns blocks of at least layout.size() bytes at
// layout.align() (or null), and dealloc_typed frees exactly those.
unsafe impl GlobalAlloc for ExecAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: IExec is set before any allocation (see the type).
        unsafe { alloc_typed(MEMF_PRIVATE, layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: GlobalAlloc's contract: `ptr` came from alloc(layout).
        unsafe { dealloc_typed(ptr, layout) }
    }
}

// SAFETY: as for ExecAllocator.
unsafe impl GlobalAlloc for SharedExecAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as for ExecAllocator.
        unsafe { alloc_typed(MEMF_SHARED, layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: as for ExecAllocator.
        unsafe { dealloc_typed(ptr, layout) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    #[test]
    fn exec_allocators_are_zst() {
        assert_eq!(mem::size_of::<ExecAllocator>(), 0);
        assert_eq!(mem::size_of::<SharedExecAllocator>(), 0);
    }
}
