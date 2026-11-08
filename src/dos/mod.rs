//! dos.library, safe (feature `dos`).
//!
//! - [`fs`]: [`File`](fs::File) (Open, Read, Write, seek, size; Close on
//!   drop), [`Lock`](fs::Lock) (UnLock on drop), [`Metadata`](fs::Metadata)
//!   (ExamineObject, copied out), [`ReadDir`](fs::ReadDir)
//!   (ObtainDirContext / ExamineDir), and the operations on names:
//!   create_dir, delete, rename, protection, comment, date.
//! - here: the error text ([`fault`]), [`DateStamp`], variables
//!   ([`get_var`], [`set_var`], [`delete_var`]), [`Pattern`], path parts
//!   ([`add_part`], [`file_part`], [`path_part`]), the Shell streams
//!   ([`stdout`], [`stderr`], [`stdin`]).
//!
//! Most of dos.library works only in a DOS Process, not in a plain exec
//! task (it uses the Process's current directory, I/O error, streams and
//! variables). The calls here check that first and return
//! `Err(WrongTask)` from a task; the ones the autodoc allows in a task
//! ([`DateStamp::now`], the date conversions, [`add_part`] and friends)
//! do not.
//!
//! Errors are `AmigaError::DosError(code)`: IoErr() read straight after
//! the call that failed. [`fault`] gives the text.

pub mod args;
pub mod fs;
pub mod misc;
pub mod notify;
pub mod process;
pub mod volume;

use alloc::string::String;
use alloc::vec::Vec;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, DOSIFace};

crate::iface::iface!(iface, DOSIFace, IDOS, b"dos.library\0", b"main\0", 52);

pub use crate::sys::DateStamp;

/// dos.library's version and revision.
pub fn version() -> (u16, u16) {
    let Ok(i) = any() else { return (0, 0) };
    // SAFETY: an open interface: its InterfaceData (offset 0) names the
    // library base, whose lib_Version and lib_Revision are at 20 and 22
    // (tests/layout/exec.c).
    unsafe {
        let base = (*(i as *const sys::InterfaceData)).lib_base as *const u8;
        (core::ptr::read(base.add(20) as *const u16), core::ptr::read(base.add(22) as *const u16))
    }
}

/// The interface, for a call allowed in a task.
pub(crate) fn any() -> Result<*mut DOSIFace> {
    iface()
}

/// The interface, if the caller is a DOS Process.
pub(crate) fn process() -> Result<*mut DOSIFace> {
    if !is_process() {
        return Err(AmigaError::WrongTask);
    }
    iface()
}

/// Whether the calling task is a DOS Process.
pub fn is_process() -> bool {
    let t = crate::signal::current_task();
    // SAFETY: the calling task, alive while it runs this; its Node's
    // ln_Type is at offset 8 (tests/layout/exec.c).
    !t.is_null() && unsafe { *((t as *const u8).add(8)) } == sys::NT_PROCESS
}

/// IoErr() of the calling Process as an error.
pub(crate) fn last_error(i: *mut DOSIFace) -> AmigaError {
    // SAFETY: an open interface, called from a Process (callers check).
    AmigaError::DosError(unsafe { ((*i).IoErr)(i) })
}

/// IoErr() of the calling Process (0 in a task).
pub fn io_err() -> i32 {
    match process() {
        // SAFETY: an open interface, in a Process.
        Ok(i) => unsafe { ((*i).IoErr)(i) },
        Err(_) => 0,
    }
}

/// The text DOS has for error `code` (Fault), e.g. "object not found".
pub fn fault(code: i32) -> String {
    let Ok(i) = any() else { return String::new() };
    let mut buf = [0u8; 128];
    // SAFETY: an open interface, no header, a buffer of its size.
    let n = unsafe { ((*i).Fault)(i, code, core::ptr::null(), buf.as_mut_ptr(), buf.len() as i32) };
    let n = (n.max(0) as usize).min(buf.len());
    let end = buf[..n].iter().position(|&b| b == 0).unwrap_or(n);
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// The DOS text for an error: Fault for `DosError`, the crate's own
/// otherwise.
pub fn describe(e: &AmigaError) -> String {
    match e {
        AmigaError::DosError(c) => fault(*c),
        other => alloc::format!("{}", other),
    }
}

impl DateStamp {
    /// DateStamp(): the system time (allowed in a task).
    pub fn now() -> Result<Self> {
        let i = any()?;
        let mut d = DateStamp::default();
        // SAFETY: an open interface and a DateStamp to fill.
        unsafe { ((*i).DateStamp)(i, &mut d) };
        Ok(d)
    }

    /// SecondsToDateStamp: seconds since 1978-01-01.
    pub fn from_seconds(s: u32) -> Result<Self> {
        let i = any()?;
        let mut d = DateStamp::default();
        // SAFETY: an open interface and a DateStamp to fill.
        unsafe { ((*i).SecondsToDateStamp)(i, s, &mut d) };
        Ok(d)
    }

    /// DateStampToSeconds (whole seconds; ticks dropped).
    pub fn to_seconds(&self) -> Result<u32> {
        let i = any()?;
        // SAFETY: an open interface and a DateStamp it only reads.
        Ok(unsafe { ((*i).DateStampToSeconds)(i, self) })
    }

    /// Seconds since 1970-01-01 (the Amiga epoch is 252460800 s later).
    pub fn to_unix(&self) -> i64 {
        self.ds_Days as i64 * 86400 + self.ds_Minute as i64 * 60 + self.ds_Tick as i64 / 50 + 252_460_800
    }
}

impl PartialOrd for DateStamp {
    fn partial_cmp(&self, o: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for DateStamp {
    /// Earlier is less (CompareDates' sign is the other way round).
    fn cmp(&self, o: &Self) -> core::cmp::Ordering {
        (self.ds_Days, self.ds_Minute, self.ds_Tick).cmp(&(o.ds_Days, o.ds_Minute, o.ds_Tick))
    }
}

/// Which variables a call looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarScope {
    /// The Process's local variable, else the global one (get only).
    Any,
    /// The Process's own (Shell) variables.
    Local,
    /// Global variables (ENV:).
    Global,
}

impl VarScope {
    fn flags(self) -> u32 {
        sys::LV_VAR
            | match self {
                VarScope::Any => 0,
                VarScope::Local => sys::GVF_LOCAL_ONLY,
                VarScope::Global => sys::GVF_GLOBAL_ONLY,
            }
    }
}

/// GetVar: the variable's bytes, or None when it does not exist.
pub fn get_var_bytes(name: &str, scope: VarScope) -> Result<Option<Vec<u8>>> {
    let i = process()?;
    let n = c_copy(name.as_bytes())?;
    // GVF_BINARY_VAR keeps newlines; GetVar NUL-terminates and counts
    // without the NUL. (With GVF_DONT_NULL_TERM a local variable came back
    // with DOS's own terminator counted as data.)
    let flags = scope.flags() | sys::GVF_BINARY_VAR;
    let mut buf: Vec<u8> = alloc::vec![0; 256];
    loop {
        // SAFETY: an open interface in a Process, a NUL-terminated name, a
        // buffer of the size given.
        let got = unsafe { ((*i).GetVar)(i, n.as_ptr(), buf.as_mut_ptr(), buf.len() as i32, flags) };
        if got < 0 {
            return Ok(None);
        }
        // IoErr holds the variable's full size.
        // SAFETY: as above, straight after GetVar.
        let full = unsafe { ((*i).IoErr)(i) }.max(got) as usize;
        if full < buf.len() {
            buf.truncate(got as usize);
            return Ok(Some(buf));
        }
        buf.resize(full + 1, 0);
    }
}

/// GetVar as text (invalid UTF-8 replaced).
pub fn get_var(name: &str, scope: VarScope) -> Result<Option<String>> {
    Ok(get_var_bytes(name, scope)?.map(|b| String::from_utf8_lossy(&b).into_owned()))
}

/// SetVar: a local variable (`Local`) or a global one in ENV: (`Global`;
/// `Any` is taken as `Local`).
pub fn set_var(name: &str, value: &[u8], scope: VarScope) -> Result<()> {
    let i = process()?;
    let n = c_copy(name.as_bytes())?;
    let scope = if scope == VarScope::Any { VarScope::Local } else { scope };
    // SAFETY: an open interface in a Process, a NUL-terminated name and
    // `value.len()` bytes given with their size.
    let ok = unsafe { ((*i).SetVar)(i, n.as_ptr(), value.as_ptr(), value.len() as i32, scope.flags()) } != 0;
    if ok { Ok(()) } else { Err(last_error(i)) }
}

/// DeleteVar.
pub fn delete_var(name: &str, scope: VarScope) -> Result<()> {
    let i = process()?;
    let n = c_copy(name.as_bytes())?;
    let scope = if scope == VarScope::Any { VarScope::Local } else { scope };
    // SAFETY: an open interface in a Process, a NUL-terminated name.
    let ok = unsafe { ((*i).DeleteVar)(i, n.as_ptr(), scope.flags()) } != 0;
    if ok { Ok(()) } else { Err(last_error(i)) }
}

/// A parsed AmigaDOS pattern (`#?`, `?`, `(a|b)`, `~`, `[a-z]`...).
pub struct Pattern {
    parsed: Vec<u8>,
    case: bool,
    wild: bool,
}

impl Pattern {
    /// ParsePattern (`case`) or ParsePatternNoCase.
    pub fn new(pattern: &str, case: bool) -> Result<Self> {
        let i = process()?;
        let src = c_copy(pattern.as_bytes())?;
        let mut parsed = alloc::vec![0u8; pattern.len() * 2 + 2];
        // SAFETY: an open interface in a Process, a NUL-terminated source
        // and a destination of the documented 2 * len + 2 bytes.
        let r = unsafe {
            if case {
                ((*i).ParsePattern)(i, src.as_ptr(), parsed.as_mut_ptr(), parsed.len() as i32)
            } else {
                ((*i).ParsePatternNoCase)(i, src.as_ptr(), parsed.as_mut_ptr(), parsed.len() as i32)
            }
        };
        if r < 0 {
            return Err(last_error(i));
        }
        Ok(Self { parsed, case, wild: r == 1 })
    }

    /// Whether the pattern has wildcards (ParsePattern returned 1).
    pub fn is_wild(&self) -> bool {
        self.wild
    }

    /// MatchPattern / MatchPatternNoCase.
    pub fn matches(&self, s: &str) -> Result<bool> {
        let i = process()?;
        let s = c_copy(s.as_bytes())?;
        // A no-match leaves IoErr alone and an error sets it: clear it
        // first so a stale code is not taken for an error.
        // SAFETY: an open interface in a Process.
        unsafe { ((*i).SetIoErr)(i, 0) };
        // SAFETY: an open interface in a Process, the parsed pattern and a
        // NUL-terminated string (the recursion needs some stack: a Shell
        // process's default is enough).
        let r = unsafe {
            if self.case {
                ((*i).MatchPattern)(i, self.parsed.as_ptr(), s.as_ptr())
            } else {
                ((*i).MatchPatternNoCase)(i, self.parsed.as_ptr(), s.as_ptr())
            }
        };
        if r != 0 {
            return Ok(true);
        }
        // SAFETY: as above, straight after the match.
        match unsafe { ((*i).IoErr)(i) } {
            0 => Ok(false),
            e => Err(AmigaError::DosError(e)),
        }
    }

    /// The parsed form, for ObtainDirContext's EX_MatchString.
    pub(crate) fn parsed(&self) -> &[u8] {
        &self.parsed
    }

    pub(crate) fn case_sensitive(&self) -> bool {
        self.case
    }
}

/// AddPart: `dir` joined with `file` the AmigaDOS way (an absolute `file`
/// replaces `dir`; `/` goes up).
pub fn add_part(dir: &str, file: &str) -> Result<String> {
    let i = any()?;
    let f = c_copy(file.as_bytes())?;
    let mut buf = Vec::with_capacity(sys::MAX_DOS_PATH as usize + 1);
    buf.extend_from_slice(&c_copy(dir.as_bytes())?);
    buf.resize(sys::MAX_DOS_PATH as usize + 1, 0);
    // SAFETY: an open interface, a NUL-terminated directory in a buffer of
    // the size given, a NUL-terminated file part.
    let ok = unsafe { ((*i).AddPart)(i, buf.as_mut_ptr(), f.as_ptr(), buf.len() as u32) } != 0;
    if !ok {
        return Err(AmigaError::DosError(sys::ERROR_LINE_TOO_LONG));
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(end);
    String::from_utf8(buf).map_err(|_| AmigaError::InvalidArgument("AddPart result is not UTF-8"))
}

/// The offset FilePart/PathPart point to inside `path`.
fn part(path: &str, file: bool) -> Result<usize> {
    let i = any()?;
    let p = c_copy(path.as_bytes())?;
    // SAFETY: an open interface and a NUL-terminated string; the result
    // points into it.
    let r = unsafe {
        if file {
            ((*i).FilePart)(i, p.as_ptr()) as usize
        } else {
            ((*i).PathPart)(i, p.as_ptr()) as usize
        }
    };
    Ok(r.saturating_sub(p.as_ptr() as usize).min(path.len()))
}

/// FilePart: the last component of `path` ("c" of "dh0:a/b/c").
pub fn file_part(path: &str) -> Result<&str> {
    let o = part(path, true)?;
    Ok(path.get(o..).unwrap_or(""))
}

/// PathPart: `path` without its last component ("dh0:a/b" of "dh0:a/b/c").
pub fn path_part(path: &str) -> Result<&str> {
    let o = part(path, false)?;
    Ok(path.get(..o).unwrap_or(path))
}

/// One of the calling Process's standard streams (never closed by us).
pub struct Stream {
    fh: u32,
    _p: core::marker::PhantomData<*const ()>,
}

impl Stream {
    /// Write all of `buf` (Write; a short write is an error).
    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        fs::write_all_fh(self.fh, buf)
    }

    /// IsInteractive: a console rather than a file or pipe.
    pub fn is_interactive(&self) -> bool {
        // SAFETY: an open interface in a Process (Stream is !Send, made by
        // one), and the Process's own stream.
        any().map(|i| unsafe { ((*i).IsInteractive)(i, self.fh) } != 0).unwrap_or(false)
    }

    /// Read some bytes (Read; 0 at the end).
    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<usize> {
        fs::read_fh(self.fh, buf)
    }
}

impl core::fmt::Write for Stream {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_bytes(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

impl crate::io::Write for Stream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_bytes(buf).map(|_| buf.len())
    }
}

impl crate::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.read_bytes(buf)
    }
}

fn stream(f: fn(*mut DOSIFace) -> u32) -> Result<Stream> {
    let i = process()?;
    let fh = f(i);
    if fh == 0 {
        return Err(AmigaError::NullPointer);
    }
    Ok(Stream { fh, _p: core::marker::PhantomData })
}

/// Output(): the Process's output stream.
pub fn stdout() -> Result<Stream> {
    // SAFETY: an open interface in a Process.
    stream(|i| unsafe { ((*i).Output)(i) })
}

/// Input(): the Process's input stream.
pub fn stdin() -> Result<Stream> {
    // SAFETY: as above.
    stream(|i| unsafe { ((*i).Input)(i) })
}

/// ErrorOutput(), or Output() when the Process has no error stream.
pub fn stderr() -> Result<Stream> {
    // SAFETY: as above.
    stream(|i| unsafe {
        let e = ((*i).ErrorOutput)(i);
        if e != 0 { e } else { ((*i).Output)(i) }
    })
}
