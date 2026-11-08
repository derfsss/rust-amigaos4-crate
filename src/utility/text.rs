//! utility.library's string calls.
//!
//! Two families with different rules:
//!
//! - [`Utility::stricmp`], [`Utility::strnicmp`], [`Utility::to_lower`],
//!   [`Utility::to_upper`]: 8-bit text in the system's character set
//!   (Latin-1 on an English system). locale.library replaces them with the
//!   user's language rules, so results can differ between machines: use
//!   them where the user's sort order is wanted (file names, lists shown to
//!   the user), and Rust's ASCII calls where a fixed answer is.
//! - [`Utility::utf8_stricmp`], [`Utility::ucs4_to_lower`], ...: Unicode
//!   simple case folding (one character to one), the same on every
//!   machine. It differs from Rust's full mapping: `ß` stays `ß` here,
//!   Rust's `to_uppercase` gives `SS`.
//!
//! The OS takes NUL-terminated strings, so each argument is copied with a
//! NUL; one that contains a NUL itself is refused (`InvalidArgument`)
//! rather than silently cut short.

use alloc::string::String;
use core::cmp::Ordering;

use super::{ordering, Utility};
use crate::error::{AmigaError, Result};
use crate::sys;

pub(crate) use crate::cstr::c_copy;

impl Utility {
    /// Stricmp: case-insensitive compare of 8-bit text (locale rules when
    /// locale.library is running).
    pub fn stricmp(&self, a: &[u8], b: &[u8]) -> Result<Ordering> {
        let (a, b) = (c_copy(a)?, c_copy(b)?);
        // SAFETY: an open interface, two NUL-terminated strings.
        Ok(ordering(unsafe { ((*self.i).Stricmp)(self.i, a.as_ptr(), b.as_ptr()) }))
    }

    /// Strnicmp: as [`Utility::stricmp`], on at most `n` bytes.
    pub fn strnicmp(&self, a: &[u8], b: &[u8], n: usize) -> Result<Ordering> {
        let (a, b) = (c_copy(a)?, c_copy(b)?);
        let n = n.min(i32::MAX as usize) as i32;
        // SAFETY: an open interface, two NUL-terminated strings.
        Ok(ordering(unsafe { ((*self.i).Strnicmp)(self.i, a.as_ptr(), b.as_ptr(), n) }))
    }

    /// ToLower: one 8-bit character (locale rules when locale.library runs).
    pub fn to_lower(&self, c: u8) -> u8 {
        // SAFETY: an open interface; the character travels in a register
        // (the SDK's TEXT argument; the generated vtable types it APTR).
        unsafe { ((*self.i).ToLower)(self.i, c as usize as sys::APTR) as u8 }
    }

    /// ToUpper: as [`Utility::to_lower`].
    pub fn to_upper(&self, c: u8) -> u8 {
        // SAFETY: as in to_lower.
        unsafe { ((*self.i).ToUpper)(self.i, c as usize as sys::APTR) as u8 }
    }

    /// UTF8Stricmp (V54): Unicode simple case folding, no locale.
    pub fn utf8_stricmp(&self, a: &str, b: &str) -> Result<Ordering> {
        self.require(54, 0, "UTF8Stricmp")?;
        let (a, b) = (c_copy(a.as_bytes())?, c_copy(b.as_bytes())?);
        // SAFETY: an open interface (V54), two NUL-terminated strings.
        let r = unsafe { ((*self.i).UTF8Stricmp)(self.i, a.as_ptr(), b.as_ptr()) };
        utf8_result(r)
    }

    /// UTF8Strnicmp (V54): as [`Utility::utf8_stricmp`] on at most `n`
    /// characters (not bytes). `n == 0` compares nothing: Equal.
    pub fn utf8_strnicmp(&self, a: &str, b: &str, n: usize) -> Result<Ordering> {
        self.require(54, 0, "UTF8Strnicmp")?;
        if n == 0 {
            return Ok(Ordering::Equal);
        }
        let (a, b) = (c_copy(a.as_bytes())?, c_copy(b.as_bytes())?);
        let n = n.min(i32::MAX as usize) as i32;
        // SAFETY: an open interface (V54), two NUL-terminated strings.
        let r = unsafe { ((*self.i).UTF8Strnicmp)(self.i, a.as_ptr(), b.as_ptr(), n) };
        utf8_result(r)
    }

    /// UCS4ToLower (V54): simple lower-case mapping of one character.
    pub fn ucs4_to_lower(&self, c: char) -> Result<char> {
        self.require(54, 0, "UCS4ToLower")?;
        // SAFETY: an open interface (V54); any int32 is accepted.
        let r = unsafe { ((*self.i).UCS4ToLower)(self.i, c as i32) };
        Ok(char::from_u32(r as u32).unwrap_or(c))
    }

    /// UCS4ToUpper (V54): simple upper-case mapping of one character.
    pub fn ucs4_to_upper(&self, c: char) -> Result<char> {
        self.require(54, 0, "UCS4ToUpper")?;
        // SAFETY: as in ucs4_to_lower.
        let r = unsafe { ((*self.i).UCS4ToUpper)(self.i, c as i32) };
        Ok(char::from_u32(r as u32).unwrap_or(c))
    }

    /// UCS4Valid (V54): whether utility.library takes `v` as a character.
    pub fn ucs4_valid(&self, v: i32) -> Result<bool> {
        self.require(54, 0, "UCS4Valid")?;
        // SAFETY: an open interface (V54); any int32 is accepted.
        Ok(unsafe { ((*self.i).UCS4Valid)(self.i, v) } != 0)
    }

    /// Decode UTF-8 with UTF8Decode (V54), invalid sequences replaced by
    /// U+FFFD (UTF_INVALID_SUBST_FFFD). Bytes containing a NUL are refused.
    pub fn utf8_decode_lossy(&self, bytes: &[u8]) -> Result<String> {
        self.require(54, 0, "UTF8Decode")?;
        // utility.library 54 with UTF_INVALID_SUBST_FFFD takes the NUL as
        // part of a sequence cut short at the end and decodes on past it
        // (libtest). Pad with four more NULs, so it reads only
        // our own bytes, and stop once it has passed the terminator.
        let mut buf = c_copy(bytes)?;
        buf.extend_from_slice(&[0; 4]);
        let end = buf.as_ptr() as usize + bytes.len();
        let mut p: sys::CONST_STRPTR = buf.as_ptr();
        let mut out = String::with_capacity(bytes.len());
        while (p as usize) <= end {
            // SAFETY: `p` points into `buf`, NUL-terminated; UTF8Decode
            // advances it by one sequence and stops at the NUL.
            let v = unsafe { ((*self.i).UTF8Decode)(self.i, &mut p, sys::UTF_INVALID_SUBST_FFFD) };
            match v {
                0 => break,
                -1 => return Err(AmigaError::InvalidArgument("UTF8Decode refused the text")),
                v => out.push(char::from_u32(v as u32).unwrap_or('\u{FFFD}')),
            }
        }
        Ok(out)
    }
}

fn utf8_result(r: i32) -> Result<Ordering> {
    if r == i32::MAX {
        // MAXINT32: a string did not decode. Rust strings are valid UTF-8,
        // so this means the OS refused something Rust accepts.
        return Err(AmigaError::InvalidArgument("utility.library could not decode the text"));
    }
    Ok(ordering(r))
}
