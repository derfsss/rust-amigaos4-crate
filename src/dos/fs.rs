//! Files, locks, examining and directory listing (dos.library, in a
//! DOS Process).
//!
//! [`File`] and [`Lock`] own their DOS handle (Close / UnLock on drop) and
//! are not `Send`: DOS tracks them per Process. [`Metadata`] is a copy of
//! an ExamineData; the ExamineData itself is freed (FreeDosObject) or left
//! to its directory context before the copy is returned, so no DOS
//! pointer escapes. [`ReadDir`] asks for every field (EX_DataFields =
//! EXF_ALL: the default leaves the V54 type bits out).

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use super::{last_error, process, DateStamp, Pattern};
use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, DOSIFace, ExamineData, TagItem, TAG_DONE};

/// Not Send, not Sync: a DOS handle belongs to the Process that made it.
type NotSend = PhantomData<*const ()>;

fn ok_or_err(i: *mut DOSIFace, ok: bool) -> Result<()> {
    if ok { Ok(()) } else { Err(last_error(i)) }
}

/// Read into `buf` from a handle (shared by File and Stream).
pub(crate) fn read_fh(fh: u32, buf: &mut [u8]) -> Result<usize> {
    let i = process()?;
    let n = buf.len().min(i32::MAX as usize) as i32;
    // SAFETY: an open interface in a Process, a handle the caller owns or
    // borrows from the Process, and `n` writable bytes.
    let r = unsafe { ((*i).Read)(i, fh, buf.as_mut_ptr() as sys::APTR, n) };
    if r < 0 { Err(last_error(i)) } else { Ok(r as usize) }
}

/// Write all of `buf` to a handle: anything short of the full length is
/// a failure (the autodoc's rule, not only -1).
pub(crate) fn write_all_fh(fh: u32, mut buf: &[u8]) -> Result<()> {
    let i = process()?;
    while !buf.is_empty() {
        let n = buf.len().min(i32::MAX as usize) as i32;
        // SAFETY: as in read_fh, `n` readable bytes.
        let r = unsafe { ((*i).Write)(i, fh, buf.as_ptr() as sys::CONST_APTR, n) };
        if r != n {
            return Err(last_error(i));
        }
        buf = &buf[n as usize..];
    }
    Ok(())
}

/// Fill `buf` from a call writing a NUL-terminated name into a buffer,
/// growing it while DOS says ERROR_LINE_TOO_LONG.
fn name_from(i: *mut DOSIFace, f: impl Fn(*mut u8, i32) -> i32) -> Result<String> {
    let mut size = 512usize;
    loop {
        let mut buf = alloc::vec![0u8; size];
        if f(buf.as_mut_ptr(), size as i32) != 0 {
            let end = buf.iter().position(|&b| b == 0).unwrap_or(size);
            buf.truncate(end);
            return Ok(String::from_utf8_lossy(&buf).into_owned());
        }
        // SAFETY: an open interface in a Process, straight after the call.
        let e = unsafe { ((*i).IoErr)(i) };
        if e != sys::ERROR_LINE_TOO_LONG || size > 64 * 1024 {
            return Err(AmigaError::DosError(e));
        }
        size *= 4;
    }
}

/// How to open a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenMode {
    /// An existing file, shared (MODE_OLDFILE).
    Read,
    /// A new, empty file; an old one is replaced (MODE_NEWFILE,
    /// exclusive).
    Create,
    /// An existing file, or a new one, for reading and writing
    /// (MODE_READWRITE, shared).
    Update,
}

/// Where a seek is counted from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    Current(i64),
    End(i64),
}

/// An open file (unbuffered DOS I/O); Close on drop.
pub struct File {
    fh: u32,
    _t: crate::track::Tracked,
    _p: NotSend,
}

/// Close a registered file (the panic path and leaks; see `crate::track`).
unsafe fn release_file(fh: usize, _: usize) {
    if let Ok(i) = super::any() {
        // SAFETY: the registry calls this once, for a file still open.
        unsafe { ((*i).Close)(i, fh as u32) };
    }
}

/// UnLock a registered lock (as release_file).
unsafe fn release_lock(l: usize, _: usize) {
    if let Ok(i) = super::any() {
        // SAFETY: the registry calls this once, for a lock still held.
        unsafe { ((*i).UnLock)(i, l as u32) };
    }
}

impl File {
    /// A File owning `fh`, registered with the calling task.
    #[track_caller]
    pub(crate) fn adopt(fh: u32) -> Self {
        Self { fh, _t: crate::track::register("dos file", fh as usize, 0, release_file), _p: PhantomData }
    }

    /// Give up the handle: unregistered, not closed.
    pub(crate) fn into_raw(self) -> u32 {
        let me = core::mem::ManuallyDrop::new(self);
        // SAFETY: `me` is never used or dropped again; its registration is
        // read out once and dropped (unregistered).
        drop(unsafe { core::ptr::read(&me._t) });
        me.fh
    }

    #[track_caller]
    pub fn open(path: &str, mode: OpenMode) -> Result<Self> {
        let i = process()?;
        let p = c_copy(path.as_bytes())?;
        let m = match mode {
            OpenMode::Read => sys::MODE_OLDFILE,
            OpenMode::Create => sys::MODE_NEWFILE,
            OpenMode::Update => sys::MODE_READWRITE,
        };
        // SAFETY: an open interface in a Process, a NUL-terminated name.
        let fh = unsafe { ((*i).Open)(i, p.as_ptr(), m) };
        if fh == 0 { Err(last_error(i)) } else { Ok(Self::adopt(fh)) }
    }

    /// A shorthand for `open(path, Create)` and `write_all(data)`.
    pub fn write_new(path: &str, data: &[u8]) -> Result<()> {
        let mut f = Self::open(path, OpenMode::Create)?;
        f.write_all(data)?;
        f.close()
    }

    /// The whole file.
    pub fn read_to_vec(path: &str) -> Result<Vec<u8>> {
        let mut f = Self::open(path, OpenMode::Read)?;
        let mut v = Vec::with_capacity(f.len().unwrap_or(0).min(64 << 20) as usize);
        let mut buf = alloc::vec![0u8; 32 * 1024];
        loop {
            match f.read(&mut buf)? {
                0 => return Ok(v),
                n => v.extend_from_slice(&buf[..n]),
            }
        }
    }

    /// Read some bytes; 0 at the end of the file.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        read_fh(self.fh, buf)
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        write_all_fh(self.fh, buf)
    }

    /// ChangeFilePosition; returns the new position.
    pub fn seek(&mut self, to: SeekFrom) -> Result<u64> {
        let i = process()?;
        let (off, mode) = match to {
            SeekFrom::Start(o) => (o as i64, sys::OFFSET_BEGINNING),
            SeekFrom::Current(o) => (o, sys::OFFSET_CURRENT),
            SeekFrom::End(o) => (o, sys::OFFSET_END),
        };
        // SAFETY: an open interface in a Process and our handle.
        let ok = unsafe { ((*i).ChangeFilePosition)(i, self.fh, off, mode) } != 0;
        ok_or_err(i, ok)?;
        self.position()
    }

    /// GetFilePosition.
    pub fn position(&self) -> Result<u64> {
        let i = process()?;
        // SAFETY: as in seek.
        let p = unsafe { ((*i).GetFilePosition)(i, self.fh) };
        if p < 0 { Err(last_error(i)) } else { Ok(p as u64) }
    }

    /// GetFileSize.
    pub fn len(&self) -> Result<u64> {
        let i = process()?;
        // SAFETY: as in seek.
        let s = unsafe { ((*i).GetFileSize)(i, self.fh) };
        if s < 0 { Err(last_error(i)) } else { Ok(s as u64) }
    }

    pub fn is_empty(&self) -> Result<bool> {
        self.len().map(|l| l == 0)
    }

    /// ChangeFileSize to `size` bytes.
    pub fn set_len(&mut self, size: u64) -> Result<()> {
        let i = process()?;
        // SAFETY: as in seek.
        let ok = unsafe { ((*i).ChangeFileSize)(i, self.fh, size as i64, sys::OFFSET_BEGINNING) } != 0;
        ok_or_err(i, ok)
    }

    /// ExamineObject of the open file.
    pub fn metadata(&self) -> Result<Metadata> {
        examine(sys::EX_FILE_HANDLE_INPUT, self.fh)
    }

    /// NameFromFH: the file's full name (for showing; volume names can
    /// repeat).
    pub fn path(&self) -> Result<String> {
        let i = process()?;
        // SAFETY: an open interface in a Process, our handle, a buffer of
        // the size given.
        name_from(i, |b, n| unsafe { ((*i).NameFromFH)(i, self.fh, b, n) })
    }

    /// DupLockFromFH: a shared lock on the file.
    #[track_caller]
    pub fn lock(&self) -> Result<Lock> {
        let i = process()?;
        // SAFETY: an open interface in a Process and our handle.
        let l = unsafe { ((*i).DupLockFromFH)(i, self.fh) };
        if l == 0 { Err(last_error(i)) } else { Ok(Lock::adopt(l)) }
    }

    /// The handle, for giving it away (a child process's stream).
    pub(crate) fn bptr(&self) -> u32 {
        self.fh
    }

    /// IsInteractive.
    pub fn is_interactive(&self) -> bool {
        // SAFETY: an open interface in a Process and our handle.
        process().map(|i| unsafe { ((*i).IsInteractive)(i, self.fh) } != 0).unwrap_or(false)
    }

    /// Close, with its result (a buffered write may fail only now).
    pub fn close(self) -> Result<()> {
        let fh = self.into_raw();
        let i = process()?;
        // SAFETY: our handle, closed once (forgotten above, so not again
        // by Drop).
        let ok = unsafe { ((*i).Close)(i, fh) } != 0;
        ok_or_err(i, ok)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: our handle, closed once, in the Process that opened
            // it (File is not Send).
            unsafe { ((*i).Close)(i, self.fh) };
        }
    }
}

impl crate::io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        File::read(self, buf)
    }
}

impl crate::io::Write for File {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_all(buf).map(|_| buf.len())
    }
}

/// How a lock holds its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    Shared,
    Exclusive,
}

/// What SameLock says about two locks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sameness {
    Same,
    SameVolume,
    Different,
}

/// A DOS lock on a file or directory; UnLock on drop.
pub struct Lock {
    l: u32,
    _t: crate::track::Tracked,
    _p: NotSend,
}

impl Lock {
    /// A Lock owning `l`, registered with the calling task.
    #[track_caller]
    pub(crate) fn adopt(l: u32) -> Self {
        Self { l, _t: crate::track::register("dos lock", l as usize, 0, release_lock), _p: PhantomData }
    }

    /// Give up the lock: unregistered, not unlocked.
    pub(crate) fn into_raw(self) -> u32 {
        let me = core::mem::ManuallyDrop::new(self);
        // SAFETY: as File::into_raw.
        drop(unsafe { core::ptr::read(&me._t) });
        me.l
    }

    #[track_caller]
    pub fn new(path: &str, mode: LockMode) -> Result<Self> {
        let i = process()?;
        let p = c_copy(path.as_bytes())?;
        let m = if mode == LockMode::Shared { sys::SHARED_LOCK } else { sys::EXCLUSIVE_LOCK };
        // SAFETY: an open interface in a Process, a NUL-terminated name.
        let l = unsafe { ((*i).Lock)(i, p.as_ptr(), m) };
        if l == 0 { Err(last_error(i)) } else { Ok(Self::adopt(l)) }
    }

    /// DupLock (shared locks only).
    #[track_caller]
    pub fn try_clone(&self) -> Result<Self> {
        let i = process()?;
        // SAFETY: an open interface in a Process and our lock.
        let l = unsafe { ((*i).DupLock)(i, self.l) };
        if l == 0 { Err(last_error(i)) } else { Ok(Self::adopt(l)) }
    }

    /// ParentDir: None at the root of a volume.
    #[track_caller]
    pub fn parent(&self) -> Result<Option<Self>> {
        let i = process()?;
        // SAFETY: an open interface in a Process and our lock; IoErr is
        // cleared first so the root (ZERO, IoErr 0) is told from a failure.
        unsafe {
            ((*i).SetIoErr)(i, 0);
            let l = ((*i).ParentDir)(i, self.l);
            if l != 0 {
                return Ok(Some(Self::adopt(l)));
            }
            match ((*i).IoErr)(i) {
                0 => Ok(None),
                e => Err(AmigaError::DosError(e)),
            }
        }
    }

    /// NameFromLock: the full name (for showing).
    pub fn path(&self) -> Result<String> {
        let i = process()?;
        // SAFETY: an open interface in a Process, our lock, a buffer of
        // the size given.
        name_from(i, |b, n| unsafe { ((*i).NameFromLock)(i, self.l, b, n) })
    }

    /// NameFromLock on a lock someone else owns (a Workbench argument).
    ///
    /// # Safety
    /// `l` is a valid lock (or ZERO) for the call.
    #[cfg_attr(not(feature = "workbench"), allow(dead_code))]
    pub(crate) unsafe fn path_of(l: u32) -> Result<String> {
        let i = process()?;
        // SAFETY: the caller's lock, a buffer of the size given.
        name_from(i, |b, n| unsafe { ((*i).NameFromLock)(i, l, b, n) })
    }

    /// The lock, for giving it away (a child's current directory).
    pub(crate) fn bptr(&self) -> u32 {
        self.l
    }

    /// SameLock.
    pub fn same(&self, other: &Lock) -> Result<Sameness> {
        let i = process()?;
        // SAFETY: an open interface in a Process and two locks we hold.
        Ok(match unsafe { ((*i).SameLock)(i, self.l, other.l) } {
            sys::LOCK_SAME => Sameness::Same,
            sys::LOCK_SAME_VOLUME => Sameness::SameVolume,
            _ => Sameness::Different,
        })
    }

    /// ExamineObject of the locked object.
    pub fn metadata(&self) -> Result<Metadata> {
        examine(sys::EX_LOCK_INPUT, self.l)
    }

    /// The entries of the locked directory.
    pub fn read_dir(&self) -> Result<ReadDir> {
        ReadDir::new(sys::EX_LOCK_INPUT, self.l, None)
    }

    /// OpenFromLock: a file from this lock (consumed on success; given
    /// back with the error otherwise).
    #[track_caller]
    pub fn open(self) -> core::result::Result<File, (AmigaError, Lock)> {
        let i = match process() {
            Ok(i) => i,
            Err(e) => return Err((e, self)),
        };
        // SAFETY: an open interface in a Process and our lock, which the
        // file takes over on success.
        let fh = unsafe { ((*i).OpenFromLock)(i, self.l) };
        if fh == 0 {
            return Err((last_error(i), self));
        }
        let _ = self.into_raw();
        Ok(File::adopt(fh))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        if let Ok(i) = super::any() {
            // SAFETY: our lock, unlocked once (UnLock is allowed in a task,
            // and Lock is not Send anyway).
            unsafe { ((*i).UnLock)(i, self.l) };
        }
    }
}

/// What kind of object an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
    SoftLink,
    Pipe,
    Socket,
    Other(u32),
}

/// A copy of what ExamineObject or ExamineDir reported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub name: String,
    pub comment: String,
    /// A link's target ("" for anything else).
    pub link: String,
    pub kind: Kind,
    /// A hard link (FSOF_LINK); `kind` is then what it links to.
    pub hard_link: bool,
    /// Bytes, for files.
    pub size: Option<u64>,
    pub date: DateStamp,
    /// The protection bits (RWED negated in bits 3..0, see dos/dos.h).
    pub protection: u32,
    pub owner: (u32, u32),
    pub object_id: u64,
    pub ref_count: u32,
    /// DOS emulated the call with an old filesystem one (54.93).
    pub emulated: bool,
}

impl Metadata {
    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Dir
    }

    pub fn is_file(&self) -> bool {
        self.kind == Kind::File
    }

    /// Copy an ExamineData's fields (all read unaligned: it is packed).
    ///
    /// # Safety
    /// `e` is a live ExamineData with all fields filled (EXF_ALL).
    unsafe fn from_raw(e: *const ExamineData) -> Self {
        let s = |p: *const u8| -> String {
            if p.is_null() {
                return String::new();
            }
            // SAFETY: a NUL-terminated string of the live ExamineData.
            String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned()
        };
        macro_rules! f {
            ($field:ident) => {
                // SAFETY: a field of the live ExamineData (the contract).
                unsafe { core::ptr::addr_of!((*e).$field).read_unaligned() }
            };
        }
        let t = f!(Type);
        let kind = match t & sys::FSO_TYPE_MASK {
            sys::FSO_TYPE_FILE => Kind::File,
            sys::FSO_TYPE_DIRECTORY => Kind::Dir,
            sys::FSO_TYPE_SOFTLINK => Kind::SoftLink,
            sys::FSO_TYPE_PIPE => Kind::Pipe,
            sys::FSO_TYPE_SOCKET => Kind::Socket,
            other => Kind::Other(other),
        };
        let size = f!(FileSize);
        Metadata {
            name: s(f!(Name) as *const u8),
            comment: s(f!(Comment) as *const u8),
            link: s(f!(Link) as *const u8),
            kind,
            hard_link: t & sys::FSOF_LINK != 0 && kind != Kind::SoftLink,
            size: (size >= 0 && kind == Kind::File).then_some(size as u64),
            date: f!(Date),
            protection: f!(Protection),
            owner: (f!(OwnerUID), f!(OwnerGID)),
            object_id: f!(ObjectID),
            ref_count: f!(RefCount),
            emulated: t & sys::FSOF_EMULATED != 0,
        }
    }
}

/// ExamineObject by one input tag; the result is copied and freed.
fn examine(tag: u32, data: u32) -> Result<Metadata> {
    let i = process()?;
    let tags = [TagItem { ti_Tag: tag, ti_Data: data }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
    // SAFETY: an open interface in a Process; the input is a live name,
    // lock or handle of the caller's.
    let e = unsafe { ((*i).ExamineObject)(i, tags.as_ptr()) };
    if e.is_null() {
        return Err(last_error(i));
    }
    // SAFETY: the ExamineData ExamineObject returned, copied, then freed
    // with FreeDosObject (DOS_EXAMINEDATA) once.
    unsafe {
        let m = Metadata::from_raw(e);
        ((*i).FreeDosObject)(i, sys::DOS_EXAMINEDATA, e as sys::APTR);
        Ok(m)
    }
}

/// ExamineObject of a name.
pub fn metadata(path: &str) -> Result<Metadata> {
    let p = c_copy(path.as_bytes())?;
    examine(sys::EX_STRING_NAME_INPUT, p.as_ptr() as u32)
}

/// Whether `path` names something (Lock succeeds).
pub fn exists(path: &str) -> Result<bool> {
    match Lock::new(path, LockMode::Shared) {
        Ok(_) => Ok(true),
        Err(AmigaError::DosError(sys::ERROR_OBJECT_NOT_FOUND)) | Err(AmigaError::DosError(sys::ERROR_DIR_NOT_FOUND)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// A directory's entries (ObtainDirContext, ExamineDir); ReleaseDirContext
/// on drop. Each item is copied before the next ExamineDir reuses the
/// context's buffer.
pub struct ReadDir {
    ctx: sys::APTR,
    done: bool,
    // Kept alive for the context: the name and the parsed pattern.
    _name: Option<Vec<u8>>,
    _pattern: Option<Vec<u8>>,
    _p: NotSend,
}

impl ReadDir {
    fn new(tag: u32, data: u32, pattern: Option<&Pattern>) -> Result<Self> {
        let i = process()?;
        let pat = match pattern {
            // EX_MatchString wants a ParsePatternNoCase result.
            Some(p) if p.case_sensitive() => return Err(AmigaError::InvalidArgument("read_dir patterns ignore case: parse with case = false")),
            Some(p) => Some(p.parsed().to_vec()),
            None => None,
        };
        let mut tags = alloc::vec![
            TagItem { ti_Tag: tag, ti_Data: data },
            TagItem { ti_Tag: sys::EX_DATA_FIELDS, ti_Data: sys::EXF_ALL },
        ];
        if let Some(p) = &pat {
            tags.push(TagItem { ti_Tag: sys::EX_MATCH_STRING, ti_Data: p.as_ptr() as u32 });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: an open interface in a Process; the input and the pattern
        // stay alive with the context (moved into it below).
        let ctx = unsafe { ((*i).ObtainDirContext)(i, tags.as_ptr()) };
        if ctx.is_null() {
            return Err(last_error(i));
        }
        Ok(Self { ctx, done: false, _name: None, _pattern: pat, _p: PhantomData })
    }
}

/// The entries of the directory `path`.
pub fn read_dir(path: &str) -> Result<ReadDir> {
    let p = c_copy(path.as_bytes())?;
    let mut r = ReadDir::new(sys::EX_STRING_NAME_INPUT, p.as_ptr() as u32, None)?;
    r._name = Some(p);
    Ok(r)
}

/// The entries of `path` whose names match `pattern` (made with
/// `Pattern::new(.., false)`).
pub fn read_dir_matching(path: &str, pattern: &Pattern) -> Result<ReadDir> {
    let p = c_copy(path.as_bytes())?;
    let mut r = ReadDir::new(sys::EX_STRING_NAME_INPUT, p.as_ptr() as u32, Some(pattern))?;
    r._name = Some(p);
    Ok(r)
}

impl Iterator for ReadDir {
    type Item = Result<Metadata>;

    fn next(&mut self) -> Option<Result<Metadata>> {
        if self.done {
            return None;
        }
        let i = match process() {
            Ok(i) => i,
            Err(e) => {
                self.done = true;
                return Some(Err(e));
            }
        };
        // SAFETY: an open interface in a Process and our context.
        let e = unsafe { ((*i).ExamineDir)(i, self.ctx) };
        if e.is_null() {
            self.done = true;
            // SAFETY: as above, straight after ExamineDir.
            return match unsafe { ((*i).IoErr)(i) } {
                sys::ERROR_NO_MORE_ENTRIES => None,
                err => Some(Err(AmigaError::DosError(err))),
            };
        }
        // SAFETY: the context's ExamineData, valid until the next
        // ExamineDir, copied now and never freed by us.
        Some(Ok(unsafe { Metadata::from_raw(e) }))
    }
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: our context, released once, by the Process that
            // obtained it (ReadDir is not Send).
            unsafe { ((*i).ReleaseDirContext)(i, self.ctx) };
        }
    }
}

/// CreateDir (the lock it returns is dropped).
pub fn create_dir(path: &str) -> Result<()> {
    let i = process()?;
    let p = c_copy(path.as_bytes())?;
    // SAFETY: an open interface in a Process, a NUL-terminated name.
    let l = unsafe { ((*i).CreateDir)(i, p.as_ptr()) };
    if l == 0 {
        return Err(last_error(i));
    }
    drop(Lock::adopt(l));
    Ok(())
}

/// Delete a file or an empty directory.
pub fn delete(path: &str) -> Result<()> {
    let i = process()?;
    let p = c_copy(path.as_bytes())?;
    // SAFETY: an open interface in a Process, a NUL-terminated name.
    let ok = unsafe { ((*i).Delete)(i, p.as_ptr()) } != 0;
    ok_or_err(i, ok)
}

/// Rename (within one volume).
pub fn rename(from: &str, to: &str) -> Result<()> {
    let i = process()?;
    let (f, t) = (c_copy(from.as_bytes())?, c_copy(to.as_bytes())?);
    // SAFETY: an open interface in a Process, two NUL-terminated names.
    let ok = unsafe { ((*i).Rename)(i, f.as_ptr(), t.as_ptr()) } != 0;
    ok_or_err(i, ok)
}

/// SetProtection.
pub fn set_protection(path: &str, bits: u32) -> Result<()> {
    let i = process()?;
    let p = c_copy(path.as_bytes())?;
    // SAFETY: an open interface in a Process, a NUL-terminated name.
    let ok = unsafe { ((*i).SetProtection)(i, p.as_ptr(), bits) } != 0;
    ok_or_err(i, ok)
}

/// SetComment.
pub fn set_comment(path: &str, comment: &str) -> Result<()> {
    let i = process()?;
    let (p, c) = (c_copy(path.as_bytes())?, c_copy(comment.as_bytes())?);
    // SAFETY: an open interface in a Process, two NUL-terminated strings.
    let ok = unsafe { ((*i).SetComment)(i, p.as_ptr(), c.as_ptr()) } != 0;
    ok_or_err(i, ok)
}

/// SetDate.
pub fn set_date(path: &str, date: &DateStamp) -> Result<()> {
    let i = process()?;
    let p = c_copy(path.as_bytes())?;
    // SAFETY: an open interface in a Process, a NUL-terminated name, a
    // DateStamp it only reads.
    let ok = unsafe { ((*i).SetDate)(i, p.as_ptr(), date) } != 0;
    ok_or_err(i, ok)
}
