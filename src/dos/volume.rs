//! Volumes, devices and assigns (a DOS_VOLUMELIST snapshot), and what a
//! filesystem says about its disk (GetDiskInfo).

use alloc::string::String;
use alloc::vec::Vec;

use super::{last_error, process};
use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, InfoData, TagItem, APTR, TAG_DONE};

/// Which names a listing holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kinds {
    pub volumes: bool,
    pub devices: bool,
    pub assigns: bool,
}

/// The names DOS knows of the given kinds, each with its colon
/// ("Workbench:", "DH0:", "LIBS:").
pub fn names(kinds: Kinds) -> Result<Vec<String>> {
    let i = process()?;
    let t = (if kinds.volumes { sys::LDF_VOLUMES } else { 0 })
        | (if kinds.devices { sys::LDF_DEVICES } else { 0 })
        | (if kinds.assigns { sys::LDF_ASSIGNS } else { 0 });
    let tags = [
        TagItem { ti_Tag: sys::ADO_TYPE, ti_Data: t },
        TagItem { ti_Tag: sys::ADO_ADD_COLON, ti_Data: 1 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: an open interface in a Process; DOS_VOLUMELIST (V53.52) with
    // its two documented tags.
    let list = unsafe { ((*i).AllocDosObject)(i, sys::DOS_VOLUMELIST, tags.as_ptr()) } as *mut sys::List;
    if list.is_null() {
        return Err(last_error(i));
    }
    let mut v = Vec::new();
    // SAFETY: the snapshot list DOS made: exec nodes with read-only names,
    // walked, copied, then freed with FreeDosObject of its type.
    unsafe {
        let mut n = sys::exec_get_head(list);
        while !n.is_null() && v.len() < 4096 {
            let name = (*n).ln_Name;
            if !name.is_null() {
                v.push(String::from_utf8_lossy(core::ffi::CStr::from_ptr(name as *const core::ffi::c_char).to_bytes()).into_owned());
            }
            n = sys::exec_get_succ(n);
        }
        ((*i).FreeDosObject)(i, sys::DOS_VOLUMELIST, list as APTR);
    }
    Ok(v)
}

/// The state of a disk, from GetDiskInfo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskState {
    WriteProtected,
    Validating,
    Validated,
    Other(i32),
}

/// What GetDiskInfo reports about the disk under a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskInfo {
    pub state: DiskState,
    pub blocks: u32,
    pub blocks_used: u32,
    pub bytes_per_block: u32,
    /// The filesystem's DOSType ("DOS\x07", "SFS\0" ...).
    pub dos_type: u32,
    /// Open locks and files on it.
    pub in_use: i32,
}

impl DiskInfo {
    pub fn total_bytes(&self) -> u64 {
        self.blocks as u64 * self.bytes_per_block as u64
    }

    pub fn free_bytes(&self) -> u64 {
        self.blocks.saturating_sub(self.blocks_used) as u64 * self.bytes_per_block as u64
    }
}

/// GetDiskInfo for the volume, device or assign `name` refers to.
pub fn disk_info(name: &str) -> Result<DiskInfo> {
    let i = process()?;
    let n = c_copy(name.as_bytes())?;
    let none = [TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
    // SAFETY: an open interface in a Process; an InfoData from
    // AllocDosObject (the autodoc wants it aligned that way).
    let id = unsafe { ((*i).AllocDosObject)(i, sys::DOS_INFODATA, none.as_ptr()) } as *mut InfoData;
    if id.is_null() {
        return Err(AmigaError::AllocationFailed);
    }
    let tags = [
        TagItem { ti_Tag: sys::GDI_STRING_NAME_INPUT, ti_Data: n.as_ptr() as u32 },
        TagItem { ti_Tag: sys::GDI_INFO_DATA, ti_Data: id as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: as above; a NUL-terminated name and our InfoData. The copy
    // is read before the InfoData is freed once.
    unsafe {
        let ok = ((*i).GetDiskInfo)(i, tags.as_ptr()) != 0;
        let r = if ok {
            let d = core::ptr::read_unaligned(id);
            Ok(DiskInfo {
                state: match d.id_DiskState {
                    sys::ID_DISKSTATE_WRITE_PROTECTED => DiskState::WriteProtected,
                    sys::ID_DISKSTATE_VALIDATING => DiskState::Validating,
                    sys::ID_DISKSTATE_VALIDATED => DiskState::Validated,
                    other => DiskState::Other(other),
                },
                blocks: d.id_NumBlocks,
                blocks_used: d.id_NumBlocksUsed,
                bytes_per_block: d.id_BytesPerBlock,
                dos_type: d.id_DOSType as u32,
                in_use: d.id_UseCount,
            })
        } else {
            Err(last_error(i))
        };
        ((*i).FreeDosObject)(i, sys::DOS_INFODATA, id as APTR);
        r
    }
}

// ---------------------------------------------------------------------------
// Assigns
// ---------------------------------------------------------------------------

/// An assign's name as DOS wants it: no colon. One trailing colon is
/// dropped; an empty name is refused.
fn assign_name(name: &str) -> Result<Vec<u8>> {
    let n = name.strip_suffix(':').unwrap_or(name);
    if n.is_empty() || n.contains(':') || n.contains('/') {
        return Err(AmigaError::DosError(sys::ERROR_INVALID_COMPONENT_NAME));
    }
    c_copy(n.as_bytes())
}

/// Run an assign call that returns a DOS boolean; IoErr on failure.
fn assign_call(f: impl FnOnce(*mut sys::DOSIFace) -> i32) -> Result<()> {
    let i = process()?;
    if f(i) != 0 { Ok(()) } else { Err(last_error(i)) }
}

/// AssignLock: `name:` refers to `dir` from now on, replacing an assign of
/// that name (and its multi-assign entries). DOS keeps the lock and unlocks
/// it when the assign goes; on failure it is handed back.
pub fn assign_lock(name: &str, dir: super::fs::Lock) -> core::result::Result<(), (AmigaError, super::fs::Lock)> {
    let n = match assign_name(name) {
        Ok(n) => n,
        Err(e) => return Err((e, dir)),
    };
    // SAFETY: an open interface in a Process, a NUL-terminated name and our
    // lock, which DOS takes over only when it returns success.
    match assign_call(|i| unsafe { ((*i).AssignLock)(i, n.as_ptr(), dir.bptr()) }) {
        Ok(()) => {
            let _ = dir.into_raw();
            Ok(())
        }
        Err(e) => Err((e, dir)),
    }
}

/// AssignPath: `name:` is expanded to `path` at each use (a non-binding
/// assign; the path need not exist yet).
pub fn assign_path(name: &str, path: &str) -> Result<()> {
    let (n, p) = (assign_name(name)?, c_copy(path.as_bytes())?);
    if path.is_empty() {
        return Err(AmigaError::DosError(sys::ERROR_REQUIRED_ARG_MISSING));
    }
    // SAFETY: an open interface in a Process and two NUL-terminated strings.
    assign_call(|i| unsafe { ((*i).AssignPath)(i, n.as_ptr(), p.as_ptr()) })
}

/// AssignLate: `name:` is resolved to `path` at its first use, then acts
/// as a lock assign.
pub fn assign_late(name: &str, path: &str) -> Result<()> {
    let (n, p) = (assign_name(name)?, c_copy(path.as_bytes())?);
    if path.is_empty() {
        return Err(AmigaError::DosError(sys::ERROR_REQUIRED_ARG_MISSING));
    }
    // SAFETY: as in assign_path.
    assign_call(|i| unsafe { ((*i).AssignLate)(i, n.as_ptr(), p.as_ptr()) })
}

/// AssignAddToList: add `dir` to the multi-assign `name:` at `position`
/// (0 = the primary directory, 1 = the first extra one; `None` appends).
/// DOS keeps the lock on success; on failure it is handed back.
pub fn assign_add(name: &str, dir: super::fs::Lock, position: Option<u32>) -> core::result::Result<(), (AmigaError, super::fs::Lock)> {
    let n = match assign_name(name) {
        Ok(n) => n,
        Err(e) => return Err((e, dir)),
    };
    let pos = position.map(|p| p.min(i32::MAX as u32) as i32).unwrap_or(-1);
    // SAFETY: as in assign_lock.
    match assign_call(|i| unsafe { ((*i).AssignAddToList)(i, n.as_ptr(), dir.bptr(), pos) }) {
        Ok(()) => {
            let _ = dir.into_raw();
            Ok(())
        }
        Err(e) => Err((e, dir)),
    }
}

/// RemAssignList: take the entry for the object `dir` refers to out of the
/// multi-assign `name:` (DOS unlocks its own lock; `dir` stays yours).
pub fn remove_assign_entry(name: &str, dir: &super::fs::Lock) -> Result<()> {
    let n = assign_name(name)?;
    // SAFETY: an open interface in a Process, a name and a lock we keep.
    assign_call(|i| unsafe { ((*i).RemAssignList)(i, n.as_ptr(), dir.bptr()) })
}

/// The kind of assign [`remove_assign`] takes away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignKind {
    /// Made by [`assign_lock`] or [`assign_add`] (or a resolved late one).
    Lock,
    /// Made by [`assign_path`].
    Path,
    /// Made by [`assign_late`] and not used yet.
    Late,
}

/// Remove the assign `name:` of the given kind, with all its entries
/// (AssignLock with no lock, AssignPath or AssignLate with no path).
pub fn remove_assign(name: &str, kind: AssignKind) -> Result<()> {
    let n = assign_name(name)?;
    let empty = [0u8];
    // SAFETY: an open interface in a Process and a name; 0 / "" mean
    // remove (the "" form works from dos 53.24).
    assign_call(|i| unsafe {
        match kind {
            AssignKind::Lock => ((*i).AssignLock)(i, n.as_ptr(), 0),
            AssignKind::Path => ((*i).AssignPath)(i, n.as_ptr(), empty.as_ptr()),
            AssignKind::Late => ((*i).AssignLate)(i, n.as_ptr(), empty.as_ptr()),
        }
    })
}
