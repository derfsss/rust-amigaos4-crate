//! Command-line arguments by template (ReadArgs), safe.
//!
//! [`Args::parse`] reads a string, [`Args::from_command_line`] the
//! program's own Shell arguments. The template is the Shell's:
//! `"FROM/A/M,TO/A,ALL/S,BUF=BUFFERS/K/N,QUIET/T"`. Each option is read
//! back by any of its names, at the type its modifiers give it: a /N
//! option cannot be read as a string, a /S one only as a switch.
//! Everything ReadArgs allocated is freed (FreeArgs) when the `Args` is
//! dropped, so the strings it gives out borrow it.

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, CStr};

use super::{last_error, process};
use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, RDArgs, TagItem, TAG_DONE};

/// What an option holds, from its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// No type modifier (also /K, /F, /A alone): a string.
    Text,
    /// /S: present or not.
    Switch,
    /// /T: yes/no/on/off, or absent.
    Toggle,
    /// /N: a 32-bit number.
    Number,
    /// /M: several strings.
    Multi,
}

/// One option of a template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opt {
    /// Its names ("BUF=BUFFERS" gives two), upper-case.
    pub names: Vec<String>,
    pub kind: Kind,
    /// /A
    pub required: bool,
    /// /K
    pub keyword: bool,
    /// /F
    pub rest: bool,
}

/// Split a template into its options (what ReadArgs indexes by comma).
pub fn parse_template(template: &str) -> Result<Vec<Opt>> {
    let mut v = Vec::new();
    for item in template.split(',') {
        let mut parts = item.split('/');
        let names: Vec<String> = parts.next().unwrap_or("").split('=').map(|n| n.trim().to_ascii_uppercase()).collect();
        if names.iter().any(|n| n.is_empty()) {
            return Err(AmigaError::DosError(sys::ERROR_BAD_TEMPLATE));
        }
        let mut o = Opt { names, kind: Kind::Text, required: false, keyword: false, rest: false };
        for m in parts {
            match m.trim().to_ascii_uppercase().as_str() {
                "S" => o.kind = Kind::Switch,
                "T" => o.kind = Kind::Toggle,
                "N" => o.kind = Kind::Number,
                "M" => o.kind = Kind::Multi,
                "A" => o.required = true,
                "K" => o.keyword = true,
                "F" => o.rest = true,
                _ => return Err(AmigaError::DosError(sys::ERROR_BAD_TEMPLATE)),
            }
        }
        v.push(o);
    }
    if v.iter().filter(|o| o.kind == Kind::Multi).count() > 1 {
        return Err(AmigaError::DosError(sys::ERROR_BAD_TEMPLATE));
    }
    Ok(v)
}

/// /T slots start with this, so an absent toggle can be told apart
/// (the autodoc: anything but -1, 0, 1).
const TOGGLE_UNSET: u32 = 0x5EED_0002;

/// Parsed arguments; FreeArgs (and the RDArgs freed) on drop.
pub struct Args {
    opts: Vec<Opt>,
    array: Vec<u32>,
    result: *mut RDArgs,
    own: *mut RDArgs,
    // Kept alive while ReadArgs' results may point into them.
    _template: Vec<u8>,
    _line: Vec<u8>,
    _p: core::marker::PhantomData<*const ()>,
}

impl Args {
    fn read(template: &str, line: Option<&str>) -> Result<Self> {
        let i = process()?;
        let opts = parse_template(template)?;
        let tpl = c_copy(template.as_bytes())?;
        let mut array: Vec<u32> = opts.iter().map(|o| if o.kind == Kind::Toggle { TOGGLE_UNSET } else { 0 }).collect();
        let mut text = Vec::new();
        let mut own: *mut RDArgs = core::ptr::null_mut();
        if let Some(l) = line {
            // A CSource must end with a newline (the autodoc's BUGS).
            let mut s = String::from(l.trim_end_matches('\n'));
            s.push('\n');
            text = c_copy(s.as_bytes())?;
            let tags = [TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
            // SAFETY: an open interface in a Process; an RDArgs to pass
            // ReadArgs, freed with FreeDosObject of the same type.
            own = unsafe { ((*i).AllocDosObject)(i, sys::DOS_RDARGS, tags.as_ptr()) } as *mut RDArgs;
            if own.is_null() {
                return Err(AmigaError::AllocationFailed);
            }
            // SAFETY: the fresh RDArgs: the source is our string (kept in
            // `_line`), no own buffer, never prompt.
            unsafe {
                core::ptr::addr_of_mut!((*own).RDA_Source.CS_Buffer).write_unaligned(text.as_ptr());
                core::ptr::addr_of_mut!((*own).RDA_Source.CS_Length).write_unaligned(text.len() as i32 - 1);
                core::ptr::addr_of_mut!((*own).RDA_Source.CS_CurChr).write_unaligned(0);
                core::ptr::addr_of_mut!((*own).RDA_Buffer).write_unaligned(core::ptr::null_mut());
                core::ptr::addr_of_mut!((*own).RDA_BufSiz).write_unaligned(0);
                core::ptr::addr_of_mut!((*own).RDA_Flags).write_unaligned(sys::RDAF_NOPROMPT);
            }
        }
        // SAFETY: an open interface in a Process; a NUL-terminated template;
        // one zeroed (or toggle-preset) longword per option; our RDArgs or
        // NULL (then ReadArgs reads the Process's own arguments).
        let result = unsafe { ((*i).ReadArgs)(i, tpl.as_ptr(), array.as_mut_ptr() as *mut i32, own) };
        if result.is_null() {
            let e = last_error(i);
            if !own.is_null() {
                // SAFETY: our RDArgs, unused after the failed call.
                unsafe { ((*i).FreeDosObject)(i, sys::DOS_RDARGS, own as sys::APTR) };
            }
            return Err(e);
        }
        Ok(Self { opts, array, result, own, _template: tpl, _line: text, _p: core::marker::PhantomData })
    }

    /// ReadArgs over `line` (a newline is added when missing).
    pub fn parse(template: &str, line: &str) -> Result<Self> {
        Self::read(template, Some(line))
    }

    /// ReadArgs over the program's own command line (the Process's
    /// arguments; the Shell gives them on Input()).
    pub fn from_command_line(template: &str) -> Result<Self> {
        Self::read(template, None)
    }

    /// The template's options.
    pub fn options(&self) -> &[Opt] {
        &self.opts
    }

    fn slot(&self, name: &str, kind: Kind) -> Result<u32> {
        let n = name.to_ascii_uppercase();
        let i = self.opts.iter().position(|o| o.names.contains(&n)).ok_or(AmigaError::InvalidArgument("no such option in the template"))?;
        if self.opts[i].kind != kind {
            return Err(AmigaError::InvalidArgument("the option has another type"));
        }
        Ok(self.array[i])
    }

    /// A /S option: whether it was given.
    pub fn switch(&self, name: &str) -> Result<bool> {
        self.slot(name, Kind::Switch).map(|v| v != 0)
    }

    /// A /T option: Some(on) when given.
    pub fn toggle(&self, name: &str) -> Result<Option<bool>> {
        self.slot(name, Kind::Toggle).map(|v| (v != TOGGLE_UNSET).then_some(v != 0))
    }

    /// A string option, when given.
    pub fn text(&self, name: &str) -> Result<Option<&str>> {
        let p = self.slot(name, Kind::Text)?;
        Ok((p != 0).then(|| {
            // SAFETY: a NUL-terminated string ReadArgs made, freed only by
            // FreeArgs when self is dropped.
            unsafe { CStr::from_ptr(p as *const c_char) }.to_str().unwrap_or("")
        }))
    }

    /// A /N option, when given.
    pub fn number(&self, name: &str) -> Result<Option<i32>> {
        let p = self.slot(name, Kind::Number)?;
        // SAFETY: a pointer to the int32 ReadArgs converted, alive until
        // FreeArgs.
        Ok((p != 0).then(|| unsafe { core::ptr::read(p as *const i32) }))
    }

    /// A /M option's strings (empty when none).
    pub fn multi(&self, name: &str) -> Result<Vec<&str>> {
        let p = self.slot(name, Kind::Multi)? as *const *const c_char;
        let mut v = Vec::new();
        if p.is_null() {
            return Ok(v);
        }
        // SAFETY: ReadArgs' NULL-terminated array of NUL-terminated
        // strings, alive until FreeArgs.
        unsafe {
            let mut k = 0;
            loop {
                let s = core::ptr::read(p.add(k));
                if s.is_null() || k > 4096 {
                    break;
                }
                v.push(CStr::from_ptr(s).to_str().unwrap_or(""));
                k += 1;
            }
        }
        Ok(v)
    }
}

impl Drop for Args {
    fn drop(&mut self) {
        if let Ok(i) = process() {
            // SAFETY: the RDArgs ReadArgs returned, freed once; then ours
            // (if we gave one), with FreeDosObject of its type.
            unsafe {
                ((*i).FreeArgs)(i, self.result);
                if !self.own.is_null() {
                    ((*i).FreeDosObject)(i, sys::DOS_RDARGS, self.own as sys::APTR);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates() {
        let t = parse_template("FROM/A/M,TO/A,ALL/S,BUF=BUFFERS/K/N,QUIET/T,REST/F").unwrap();
        assert_eq!(t.len(), 6);
        assert_eq!(t[0].kind, Kind::Multi);
        assert!(t[0].required);
        assert_eq!(t[3].names, ["BUF", "BUFFERS"]);
        assert_eq!(t[3].kind, Kind::Number);
        assert!(t[3].keyword);
        assert_eq!(t[4].kind, Kind::Toggle);
        assert!(t[5].rest && t[5].kind == Kind::Text);
        assert!(parse_template("A/M,B/M").is_err());
        assert!(parse_template("A/X").is_err());
        assert!(parse_template("A,,B").is_err());
    }
}
