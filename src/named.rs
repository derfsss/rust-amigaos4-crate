//! Named memory (exec V51): blocks found by a namespace and a name instead
//! of a pointer, shared between programs. The "resident" namespace
//! survives a reboot (checksummed by default).
//!
//! A block's contents are only reachable through a [`NamedLock`]
//! (LockNamedMemory / AttemptNamedMemory): exclusive access, released on
//! drop. Freeing goes through the lock too ([`NamedLock::free`]), so this
//! program never frees a block it is using.
//!
//! exec has no call that gives a block's size without a scan, and
//! ScanNamedMemory waits while another task holds a named block. So a
//! block made by [`alloc`] starts with an 8-byte header, `RSNM` and the
//! size the caller asked for, written under the block's lock; [`lock`]
//! reads it and refuses a block without it.
//!
//! There is no `try_lock`: exec 54.57's AttemptNamedMemory, documented as
//! never blocking, spins in the kernel while another task holds the block
//! and stalls the whole machine (QEMU AmigaOne).
//! LockNamedMemory waits properly. The header is trusted: named memory is shared
//! with other programs, and one that overwrites it can make this one read
//! past the block, as it could corrupt any shared memory.

use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::error::{AmigaError, Result};
use crate::sys::{self, TagItem, CONST_STRPTR, TAG_DONE};

/// "RSNM": a block made by [`alloc`].
const MAGIC: u32 = 0x5253_4E4D;
const HEADER: usize = 8;

/// A namespace (None: the global one) and a name, NUL-terminated.
struct Key {
    space: Option<Vec<u8>>,
    name: Vec<u8>,
}

fn cstring(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s.as_bytes());
    v.push(0);
    v
}

impl Key {
    fn new(space: Option<&str>, name: &str) -> Self {
        Self { space: space.map(cstring), name: cstring(name) }
    }

    fn space(&self) -> CONST_STRPTR {
        self.space.as_ref().map_or(core::ptr::null(), |s| s.as_ptr())
    }

    fn name(&self) -> CONST_STRPTR {
        self.name.as_ptr()
    }
}

/// Make a zeroed block of `size` bytes named `name` in `space` (None:
/// global), with a checksum when `checksum` (exec's default for "resident"
/// is true). `Err(Exists)` when the name is taken.
pub fn alloc(space: Option<&str>, name: &str, size: u32) -> Result<()> {
    alloc_with(space, name, size, false)
}

/// As [`alloc`], choosing whether exec keeps a checksum (resident blocks:
/// a block whose checksum fails after a reboot is discarded).
pub fn alloc_with(space: Option<&str>, name: &str, size: u32, checksum: bool) -> Result<()> {
    let total = size.checked_add(HEADER as u32).ok_or(AmigaError::AllocationFailed)?;
    let key = Key::new(space, name);
    let mut err = 0u32;
    let mut tags = [
        TagItem { ti_Tag: sys::ANMT_CHECKSUM, ti_Data: checksum as u32 },
        TagItem { ti_Tag: sys::ANMT_ERROR, ti_Data: &mut err as *mut u32 as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: NUL-terminated strings (copied by exec) and a tag list whose
    // ANMT_Error points at a u32 on this stack.
    let p = unsafe { sys::exec_alloc_named_memory(total, key.space(), key.name(), tags.as_mut_ptr()) };
    if p.is_null() {
        return Err(match err {
            sys::ANMERROR_DUPLICATENAME => AmigaError::Exists,
            sys::ANMERROR_PARAMETER => AmigaError::Device("AllocNamedMemory: bad parameter"),
            _ => AmigaError::AllocationFailed,
        });
    }
    // Write the header (and zero the contents) under the block's lock.
    // SAFETY: NUL-terminated strings; the lock is released below.
    let q = unsafe { sys::exec_lock_named_memory(key.space(), key.name()) } as *mut u8;
    if q.is_null() {
        return Err(AmigaError::Device("named block gone right after it was made"));
    }
    // SAFETY: we hold the block of `total` bytes at `q`.
    unsafe {
        core::ptr::write_bytes(q, 0, total as usize);
        core::ptr::write_unaligned(q as *mut u32, MAGIC);
        core::ptr::write_unaligned(q.add(4) as *mut u32, size);
        if checksum {
            sys::exec_update_named_memory(key.space(), key.name());
        }
        sys::exec_unlock_named_memory(key.space(), key.name());
    }
    Ok(())
}

/// Is there a block named `name` in `space`?
pub fn exists(space: Option<&str>, name: &str) -> bool {
    let key = Key::new(space, name);
    // SAFETY: NUL-terminated strings; the pointer is only compared.
    !unsafe { sys::exec_find_named_memory(key.space(), key.name()) }.is_null()
}

/// Exclusive access to a named block, waiting for another holder. None:
/// no such block, or not one made by [`alloc`].
pub fn lock(space: Option<&str>, name: &str) -> Option<NamedLock> {
    let key = Key::new(space, name);
    // SAFETY: NUL-terminated strings.
    let p = unsafe { sys::exec_lock_named_memory(key.space(), key.name()) } as *mut u8;
    NamedLock::from_locked(key, p)
}

/// A locked named block; unlocked on drop.
pub struct NamedLock {
    key: Key,
    p: *mut u8,
    len: usize,
    _not_send: PhantomData<*mut ()>,
}

impl NamedLock {
    fn from_locked(key: Key, p: *mut u8) -> Option<Self> {
        if p.is_null() {
            return None;
        }
        // SAFETY: we hold the block; a block made by alloc has at least the
        // 8-byte header. One that does not start with MAGIC is refused
        // before its size word is used.
        let (magic, len) = unsafe {
            (core::ptr::read_unaligned(p as *const u32), core::ptr::read_unaligned(p.add(4) as *const u32))
        };
        if magic != MAGIC {
            // SAFETY: we hold the lock just taken.
            unsafe { sys::exec_unlock_named_memory(key.space(), key.name()) };
            return None;
        }
        Some(Self { key, p, len: len as usize, _not_send: PhantomData })
    }

    /// Recompute the block's checksum after changing it (UpdateNamedMemory).
    pub fn update_checksum(&self) {
        // SAFETY: NUL-terminated strings of a block we hold.
        unsafe { sys::exec_update_named_memory(self.key.space(), self.key.name()) }
    }

    /// Unlock and free the block. False: exec refused to free it.
    pub fn free(self) -> bool {
        let me = core::mem::ManuallyDrop::new(self);
        // SAFETY: the key is read out once; `me` is not used or dropped again.
        let key = unsafe { core::ptr::read(&me.key) };
        // SAFETY: NUL-terminated strings of the block we held.
        unsafe {
            sys::exec_unlock_named_memory(key.space(), key.name());
            sys::exec_free_named_memory(key.space(), key.name()) != 0
        }
    }
}

impl Deref for NamedLock {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        // SAFETY: the block after its header, held exclusively, `len` bytes
        // as alloc recorded.
        unsafe { core::slice::from_raw_parts(self.p.add(HEADER), self.len) }
    }
}

impl DerefMut for NamedLock {
    fn deref_mut(&mut self) -> &mut [u8] {
        // SAFETY: as above, borrowed mutably.
        unsafe { core::slice::from_raw_parts_mut(self.p.add(HEADER), self.len) }
    }
}

impl Drop for NamedLock {
    fn drop(&mut self) {
        // SAFETY: NUL-terminated strings of a block we hold.
        unsafe { sys::exec_unlock_named_memory(self.key.space(), self.key.name()) }
    }
}
