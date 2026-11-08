//! IDiskIO global(s) and convenience wrappers.
//!
//! Hand-written binding for diskio.library — high-throughput block
//! and byte I/O against any mounted storage device. Used by
//! filesystems and disk utilities; bypasses the filesystem and
//! talks to the underlying device directly.

use crate::sys::types::*;
use crate::sys::interfaces::diskio::*;

// ---- IDiskIO (DiskIOIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IDiskIO: *mut DiskIOIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IDiskIO: *mut DiskIOIFace = core::ptr::null_mut();

// ── Setup / teardown ─────────────────────────────────────────

#[inline]
pub unsafe fn diskio_setup(device_name: CONST_STRPTR, tag_list: *const TagItem) -> *mut DiskIO {
    ((*IDiskIO).Setup)(IDiskIO, device_name, tag_list)
}

#[inline]
pub unsafe fn diskio_cleanup(handle: *mut DiskIO) {
    ((*IDiskIO).Cleanup)(IDiskIO, handle)
}

#[inline]
pub unsafe fn diskio_update(handle: *mut DiskIO) {
    ((*IDiskIO).Update)(IDiskIO, handle)
}

// ── Status queries ───────────────────────────────────────────

#[inline]
pub unsafe fn diskio_is_disk_present(handle: *mut DiskIO) -> u32 {
    ((*IDiskIO).IsDiskPresent)(IDiskIO, handle)
}

#[inline]
pub unsafe fn diskio_is_write_protected(handle: *mut DiskIO) -> u32 {
    ((*IDiskIO).IsWriteProtected)(IDiskIO, handle)
}

#[inline]
pub unsafe fn diskio_query(handle: *mut DiskIO, tag_list: *const TagItem) {
    ((*IDiskIO).Query)(IDiskIO, handle, tag_list)
}

// ── Cache / flush ────────────────────────────────────────────

#[inline]
pub unsafe fn diskio_flush_iocache(handle: *mut DiskIO) -> i32 {
    ((*IDiskIO).FlushIOCache)(IDiskIO, handle)
}

#[inline]
pub unsafe fn diskio_flush_blocks(handle: *mut DiskIO, first_block: u64, num_blocks: u32) -> i32 {
    ((*IDiskIO).FlushBlocks)(IDiskIO, handle, first_block, num_blocks)
}

// ── Block-aligned I/O ────────────────────────────────────────

#[inline]
pub unsafe fn diskio_read_blocks(
    handle: *mut DiskIO, first_block: u64, buffer: APTR, num_blocks: u32,
) -> i32 {
    ((*IDiskIO).ReadBlocks)(IDiskIO, handle, first_block, buffer, num_blocks)
}

#[inline]
pub unsafe fn diskio_write_blocks(
    handle: *mut DiskIO, first_block: u64, buffer: CONST_APTR, num_blocks: u32,
) -> i32 {
    ((*IDiskIO).WriteBlocks)(IDiskIO, handle, first_block, buffer, num_blocks)
}

// ── Byte-granular I/O ────────────────────────────────────────

#[inline]
pub unsafe fn diskio_read_bytes(
    handle: *mut DiskIO, offset: u64, buffer: APTR, length: u32,
) -> i32 {
    ((*IDiskIO).ReadBytes)(IDiskIO, handle, offset, buffer, length)
}

#[inline]
pub unsafe fn diskio_write_bytes(
    handle: *mut DiskIO, offset: u64, buffer: CONST_APTR, length: u32,
) -> i32 {
    ((*IDiskIO).WriteBytes)(IDiskIO, handle, offset, buffer, length)
}

// ── Disk-change notifications ────────────────────────────────

#[inline]
pub unsafe fn diskio_add_disk_change_handler(
    device_name: CONST_STRPTR, handler: APTR, user_data: APTR, tag_list: *const TagItem,
) -> *mut DiskChangeHandler {
    ((*IDiskIO).AddDiskChangeHandler)(IDiskIO, device_name, handler, user_data, tag_list)
}

#[inline]
pub unsafe fn diskio_rem_disk_change_handler(handler: *mut DiskChangeHandler) {
    ((*IDiskIO).RemDiskChangeHandler)(IDiskIO, handler)
}
