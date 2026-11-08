//! Running a Shell command (System), the current-directory and
//! no-requester guards, and dates as text (DateToStr / StrToDate).

use alloc::string::String;

use super::fs::{File, Lock};
use super::{last_error, process, DateStamp};
use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, TagItem, TAG_DONE};

/// System(): run `command` as a Shell command line and wait for it; its
/// return code. Input and output are the calling Process's own, or the
/// given files (left open: in this synchronous form DOS does not close
/// them). The asynchronous form, which closes the streams it is given, is
/// not wrapped.
pub fn system(command: &str, input: Option<&File>, output: Option<&File>) -> Result<i32> {
    let i = process()?;
    let c = c_copy(command.as_bytes())?;
    let mut tags = alloc::vec::Vec::with_capacity(3);
    if let Some(f) = input {
        tags.push(TagItem { ti_Tag: sys::SYS_INPUT, ti_Data: f.bptr() });
    }
    if let Some(f) = output {
        tags.push(TagItem { ti_Tag: sys::SYS_OUTPUT, ti_Data: f.bptr() });
    }
    tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
    // SAFETY: an open interface in a Process; a NUL-terminated command;
    // handles we keep owning (synchronous System does not close them).
    let r = unsafe { ((*i).System)(i, c.as_ptr(), tags.as_ptr()) };
    if r == -1 { Err(last_error(i)) } else { Ok(r) }
}

/// The calling Process's current directory, changed while this lives
/// (SetCurrentDir); the old one is put back on drop. The new directory's
/// lock is borrowed, so it outlives the change.
pub struct CurrentDir<'l> {
    old: u32,
    _l: core::marker::PhantomData<&'l Lock>,
    _p: core::marker::PhantomData<*const ()>,
}

impl<'l> CurrentDir<'l> {
    pub fn enter(dir: &'l Lock) -> Result<Self> {
        let i = process()?;
        // SAFETY: an open interface in a Process and a lock we borrow for
        // as long as it is current; SetCurrentDir returns the old one,
        // which goes back on drop.
        let old = unsafe { ((*i).SetCurrentDir)(i, dir.bptr()) };
        Ok(Self { old, _l: core::marker::PhantomData, _p: core::marker::PhantomData })
    }
}

impl Drop for CurrentDir<'_> {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: the Process's previous current directory, put back.
            unsafe { ((*i).SetCurrentDir)(i, self.old) };
        }
    }
}

/// No "Please insert volume" requesters while this lives
/// (SetProcWindow(-1)); the old window pointer is put back on drop. Use
/// it around calls on names whose volume may not be mounted: they fail
/// with an error instead of waiting for the user.
pub struct NoRequesters {
    old: sys::APTR,
    _p: core::marker::PhantomData<*const ()>,
}

impl NoRequesters {
    pub fn new() -> Result<Self> {
        let i = process()?;
        // SAFETY: an open interface in a Process; -1 is the documented
        // "no requesters" value; the old value goes back on drop.
        let old = unsafe { ((*i).SetProcWindow)(i, usize::MAX as sys::CONST_APTR) };
        Ok(Self { old, _p: core::marker::PhantomData })
    }
}

impl Drop for NoRequesters {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: the Process's previous window pointer, put back.
            unsafe { ((*i).SetProcWindow)(i, self.old) };
        }
    }
}

/// DateToStr's and StrToDate's date formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateFormat {
    /// dd-mmm-yy
    Dos,
    /// yy-mmm-dd
    Int,
    /// mm-dd-yy
    Usa,
    /// dd-mm-yy
    Cdn,
    /// The locale's.
    Default,
    /// yyyy-mm-dd
    Iso,
}

impl DateFormat {
    fn code(self) -> u8 {
        match self {
            DateFormat::Dos => sys::FORMAT_DOS,
            DateFormat::Int => sys::FORMAT_INT,
            DateFormat::Usa => sys::FORMAT_USA,
            DateFormat::Cdn => sys::FORMAT_CDN,
            DateFormat::Default => sys::FORMAT_DEF,
            DateFormat::Iso => sys::FORMAT_ISO,
        }
    }
}

/// A date as DateToStr writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateText {
    pub day: String,
    pub date: String,
    pub time: String,
}

fn cut(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

impl DateStamp {
    /// DateToStr. `relative` lets it say "Today", "Yesterday", ...
    pub fn to_text(&self, format: DateFormat, relative: bool) -> Result<DateText> {
        let i = super::any()?;
        let (mut day, mut date, mut time) = ([0u8; 64], [0u8; 64], [0u8; 64]);
        let mut dt = sys::DateTime {
            dat_Stamp: *self,
            dat_Format: format.code(),
            dat_Flags: if relative { sys::DTF_SUBST } else { 0 },
            dat_StrDay: day.as_mut_ptr(),
            dat_StrDate: date.as_mut_ptr(),
            dat_StrTime: time.as_mut_ptr(),
        };
        // SAFETY: an open interface; a DateTime whose three buffers are 64
        // bytes each (LEN_DATSTRING is 16).
        if unsafe { ((*i).DateToStr)(i, &mut dt) } == 0 {
            return Err(AmigaError::InvalidArgument("DateToStr refused the date"));
        }
        Ok(DateText { day: cut(&day), date: cut(&date), time: cut(&time) })
    }

    /// StrToDate: `date` (in `format`) and/or `time` ("hh:mm:ss") as a
    /// DateStamp. A missing part keeps today's date or midnight.
    pub fn from_text(date: Option<&str>, time: Option<&str>, format: DateFormat) -> Result<Self> {
        let i = super::any()?;
        let d = date.map(|s| c_copy(s.as_bytes())).transpose()?;
        let t = time.map(|s| c_copy(s.as_bytes())).transpose()?;
        let mut dt = sys::DateTime {
            dat_Stamp: DateStamp::now()?,
            dat_Format: format.code(),
            dat_Flags: 0,
            dat_StrDay: core::ptr::null_mut(),
            dat_StrDate: d.as_ref().map_or(core::ptr::null_mut(), |v| v.as_ptr() as *mut u8),
            dat_StrTime: t.as_ref().map_or(core::ptr::null_mut(), |v| v.as_ptr() as *mut u8),
        };
        if t.is_none() {
            let s = dt.dat_Stamp;
            dt.dat_Stamp = DateStamp { ds_Days: s.ds_Days, ds_Minute: 0, ds_Tick: 0 };
        }
        // SAFETY: an open interface; a DateTime whose strings are ours and
        // NUL-terminated (StrToDate only reads them).
        if unsafe { ((*i).StrToDate)(i, &mut dt) } == 0 {
            return Err(AmigaError::InvalidArgument("StrToDate could not read the date"));
        }
        Ok(dt.dat_Stamp)
    }
}
