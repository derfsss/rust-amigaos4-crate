//! locale.library, safe (feature `locale`).
//!
//! - [`Locale`]: OpenLocale (the user's default, or a preferences file),
//!   CloseLocale on drop. Its strings ([`Locale::text`]), character tests
//!   and case conversion, comparison and sort keys (StrnCmp, StrConvert),
//!   dates as text and back (FormatDate, ParseDate).
//! - [`Catalog`]: OpenCatalogA, CloseCatalog on drop; [`Catalog::get`]
//!   gives the translation or the built-in string. A missing catalog is
//!   not an error: the built-in strings are used.
//!
//! Strings are in the locale's code set ([`Locale::code_set`]: 4 is
//! ISO-8859-1, 106 UTF-8), not necessarily UTF-8: texts come back as
//! bytes, with `_lossy` helpers. [`Locale::format`] wraps FormatString32,
//! checking every command in the template against the typed `FmtArg`s
//! before the call.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::ffi::{c_char, CStr};

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::hook::Hook;
use crate::sync::Mutex;
use crate::sys::{self, DateStamp, LocaleIFace, TagItem, APTR, TAG_DONE};

crate::iface::iface!(iface, LocaleIFace, ILocale, b"locale.library\0", b"main\0", 50);

/// struct Locale field offsets (tests/layout/locale.c).
const LOC_LANGUAGE_NAME: usize = 4;
const LOC_CODE_SET: usize = 52;
const LOC_COUNTRY_CODE: usize = 56;
const LOC_GMT_OFFSET: usize = 64;

/// GetLocaleStr numbers (libraries/locale.h).
pub mod id {
    /// DAY_1 (Sunday) .. DAY_1 + 6.
    pub const DAY_1: u32 = 1;
    /// ABDAY_1 (Sun) ..
    pub const ABDAY_1: u32 = 8;
    /// MON_1 (January) .. MON_1 + 11.
    pub const MON_1: u32 = 15;
    /// ABMON_1 (Jan) ..
    pub const ABMON_1: u32 = 27;
    pub const YESSTR: u32 = 39;
    pub const NOSTR: u32 = 40;
    pub const AM_STR: u32 = 41;
    pub const PM_STR: u32 = 42;
}

/// How StrnCmp / StrConvert order text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    /// Byte order without case (ASCII rules).
    Ascii = 0,
    /// The language's order, accents ignored.
    Collate1 = 1,
    /// The language's order, accents and case as tie-breakers.
    Collate2 = 2,
}

/// A character test of [`Locale::is`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    AlNum,
    Alpha,
    Cntrl,
    Digit,
    Graph,
    Lower,
    Print,
    Punct,
    Space,
    Upper,
    XDigit,
}

fn bytes<'a>(p: *const u8) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    // SAFETY: a NUL-terminated string locale.library gave out, alive as
    // long as the caller's borrow of its owner (Locale, Catalog).
    unsafe { CStr::from_ptr(p as *const c_char) }.to_bytes()
}

/// An open locale; CloseLocale on drop.
pub struct Locale {
    _t: crate::track::Tracked,
    i: *mut LocaleIFace,
    loc: *mut sys::Locale,
}

/// Close a registered locale (the panic path; see `crate::track`).
unsafe fn release_locale(loc: usize, _: usize) {
    if let Ok(i) = iface() {
        // SAFETY: the registry calls this once, for a locale still open.
        unsafe { ((*i).CloseLocale)(i, loc as *mut sys::Locale) };
    }
}

/// Close a registered catalog.
unsafe fn release_catalog(cat: usize, _: usize) {
    if let Ok(i) = iface() {
        // SAFETY: as release_locale (CloseCatalog takes NULL too).
        unsafe { ((*i).CloseCatalog)(i, cat as *mut sys::Catalog) };
    }
}

impl Locale {
    /// The user's current locale (OpenLocale(NULL)).
    #[track_caller]
    pub fn current() -> Result<Self> {
        let i = iface()?;
        // SAFETY: an open interface; NULL asks for the default locale.
        let loc = unsafe { ((*i).OpenLocale)(i, core::ptr::null()) };
        if loc.is_null() { Err(AmigaError::NullPointer) } else { Ok(Self { i, loc, _t: crate::track::register("locale", loc as usize, 0, release_locale) }) }
    }

    /// A locale from a preferences file (ENV:Sys/locale.prefs style).
    #[track_caller]
    pub fn open(prefs: &str) -> Result<Self> {
        let i = iface()?;
        let n = c_copy(prefs.as_bytes())?;
        // SAFETY: an open interface and a NUL-terminated name.
        let loc = unsafe { ((*i).OpenLocale)(i, n.as_ptr()) };
        if loc.is_null() { Err(AmigaError::NullPointer) } else { Ok(Self { i, loc, _t: crate::track::register("locale", loc as usize, 0, release_locale) }) }
    }

    fn field<T: Copy>(&self, off: usize) -> T {
        // SAFETY: a field of our open Locale at its header offset.
        unsafe { core::ptr::read_unaligned((self.loc as *const u8).add(off) as *const T) }
    }

    /// loc_LanguageName ("english", ...).
    pub fn language(&self) -> String {
        String::from_utf8_lossy(bytes(self.field::<*const u8>(LOC_LANGUAGE_NAME))).into_owned()
    }

    /// loc_CodeSet: the IANA number of the locale's character set.
    pub fn code_set(&self) -> u32 {
        self.field(LOC_CODE_SET)
    }

    /// loc_CountryCode as its four bytes ("USA\0", ...).
    pub fn country_code(&self) -> [u8; 4] {
        self.field::<u32>(LOC_COUNTRY_CODE).to_be_bytes()
    }

    /// Minutes from GMT.
    pub fn gmt_offset(&self) -> i32 {
        self.field(LOC_GMT_OFFSET)
    }

    /// GetLocaleStr: day and month names, yes/no, AM/PM... (see [`id`]);
    /// None for an unknown number. In the locale's code set.
    pub fn text(&self, n: u32) -> Option<&[u8]> {
        // SAFETY: an open interface and locale; the string lives as long
        // as the locale (borrowed by the result).
        let p = unsafe { ((*self.i).GetLocaleStr)(self.i, self.loc, n) };
        (!p.is_null()).then(|| bytes(p))
    }

    /// As [`text`](Self::text), decoded leniently.
    pub fn text_lossy(&self, n: u32) -> Option<String> {
        self.text(n).map(|b| String::from_utf8_lossy(b).into_owned())
    }

    /// ConvToLower of a character code (in the locale's code set).
    pub fn to_lower(&self, c: u32) -> u32 {
        // SAFETY: an open interface and locale; any value is accepted.
        unsafe { ((*self.i).ConvToLower)(self.i, self.loc, c) }
    }

    /// ConvToUpper.
    pub fn to_upper(&self, c: u32) -> u32 {
        // SAFETY: as in to_lower.
        unsafe { ((*self.i).ConvToUpper)(self.i, self.loc, c) }
    }

    /// The Is* tests.
    pub fn is(&self, class: Class, c: u32) -> bool {
        let i = self.i;
        // SAFETY: an open interface (its vtable read) and locale; the Is*
        // calls accept any character value.
        unsafe {
            let f = match class {
                Class::AlNum => (*i).IsAlNum,
                Class::Alpha => (*i).IsAlpha,
                Class::Cntrl => (*i).IsCntrl,
                Class::Digit => (*i).IsDigit,
                Class::Graph => (*i).IsGraph,
                Class::Lower => (*i).IsLower,
                Class::Print => (*i).IsPrint,
                Class::Punct => (*i).IsPunct,
                Class::Space => (*i).IsSpace,
                Class::Upper => (*i).IsUpper,
                Class::XDigit => (*i).IsXDigit,
            };
            f(i, self.loc, c) != 0
        }
    }

    /// StrnCmp of two whole strings.
    pub fn cmp(&self, a: &[u8], b: &[u8], sort: Sort) -> Result<Ordering> {
        self.cmp_n(a, b, -1, sort)
    }

    /// StrnCmp of at most `n` characters (-1: all).
    pub fn cmp_n(&self, a: &[u8], b: &[u8], n: i32, sort: Sort) -> Result<Ordering> {
        let (a, b) = (c_copy(a)?, c_copy(b)?);
        // SAFETY: an open interface and locale, two NUL-terminated strings.
        Ok(unsafe { ((*self.i).StrnCmp)(self.i, self.loc, a.as_ptr(), b.as_ptr(), n, sort as u32) }.cmp(&0))
    }

    /// StrConvert: a sort key; byte order of keys is the `sort` order.
    pub fn sort_key(&self, s: &[u8], sort: Sort) -> Result<Vec<u8>> {
        let src = c_copy(s)?;
        let mut size = s.len() * 4 + 16;
        loop {
            let mut buf = alloc::vec![0u8; size];
            // SAFETY: an open interface and locale, a NUL-terminated
            // source and a buffer of the size given.
            let n = unsafe { ((*self.i).StrConvert)(self.i, self.loc, src.as_ptr(), buf.as_mut_ptr() as APTR, size as u32, sort as u32) } as usize;
            if n + 1 < size || size > 1 << 20 {
                buf.truncate(n.min(size));
                if let Some(z) = buf.iter().position(|&c| c == 0) {
                    buf.truncate(z);
                }
                return Ok(buf);
            }
            size *= 2;
        }
    }

    /// FormatDate: `date` written with `template` (%Y, %m, %d, %A, %H...).
    pub fn format_date(&self, template: &str, date: &DateStamp) -> Result<Vec<u8>> {
        let t = c_copy(template.as_bytes())?;
        let out = Mutex::new(Vec::new())?;
        {
            let o = &out;
            // FormatDate calls the hook with each character by value in
            // the message argument, the terminating NUL included.
            let hook = Hook::new(move |_loc: APTR, ch: APTR| -> u32 {
                let c = ch as usize as u32 as u8;
                if c != 0 {
                    if let Ok(mut v) = o.lock() {
                        v.push(c);
                    }
                }
                0
            })?;
            let mut d = *date;
            // SAFETY: an open interface and locale, a NUL-terminated
            // template, a DateStamp copy, and a hook alive across the call.
            unsafe { ((*self.i).FormatDate)(self.i, self.loc, t.as_ptr(), &mut d, hook.as_ptr()) };
        }
        Ok(out.into_inner())
    }

    /// FormatString32: `template` filled in from `args`, with the locale's
    /// conventions for `%D` and `%U` (grouped digits). Every command is
    /// checked against `args` first, since a wrong one would read past
    /// them: `%[n[U|L]$][-][0][width][.limit][l]type` with type `s` (an
    /// [`FmtArg::Str`]) or `c d D u U x X` (an [`FmtArg::Int`]); `%%` is a
    /// percent sign. `x` gives uppercase hex digits and `X` lowercase
    /// (RawDoFmt's convention). The `h`, `ll`, `b` and `p` forms are
    /// refused. `n$` picks argument n (from 1); `U$`/`L$` force the first
    /// character's case.
    pub fn format(&self, template: &str, args: &[FmtArg<'_>]) -> Result<Vec<u8>> {
        check_format(template.as_bytes(), args)?;
        let t = c_copy(template.as_bytes())?;
        // The data stream: one 32-bit word per argument; strings as
        // pointers to NUL-terminated copies kept alive here.
        let strings: Vec<Vec<u8>> = args
            .iter()
            .map(|a| match a {
                FmtArg::Str(s) => c_copy(s),
                FmtArg::Int(_) => Ok(Vec::new()),
            })
            .collect::<Result<_>>()?;
        let words: Vec<u32> = args
            .iter()
            .zip(&strings)
            .map(|(a, s)| match a {
                FmtArg::Int(v) => *v as u32,
                FmtArg::Str(_) => s.as_ptr() as u32,
            })
            .collect();
        let out = Mutex::new(Vec::new())?;
        {
            let o = &out;
            // As FormatDate: each character by value in the message
            // argument, the terminating NUL included.
            let hook = Hook::new(move |_loc: APTR, ch: APTR| -> u32 {
                let c = ch as usize as u32 as u8;
                if c != 0 {
                    if let Ok(mut v) = o.lock() {
                        v.push(c);
                    }
                }
                0
            })?;
            // SAFETY: an open interface and locale, a NUL-terminated
            // template whose every command check_format matched to a word
            // of `words` of the right kind (so no read past it), strings
            // alive across the call, and a hook alive across it.
            unsafe { ((*self.i).FormatString32)(self.i, self.loc, t.as_ptr(), words.as_ptr() as APTR, hook.as_ptr()) };
        }
        Ok(out.into_inner())
    }

    /// As [`format`](Self::format), decoded leniently.
    pub fn format_lossy(&self, template: &str, args: &[FmtArg<'_>]) -> Result<String> {
        self.format(template, args).map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    /// As [`format_date`](Self::format_date), decoded leniently.
    pub fn format_date_lossy(&self, template: &str, date: &DateStamp) -> Result<String> {
        self.format_date(template, date).map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    /// ParseDate: `input` read with `template` (only %a %A %b %B %d %e %h
    /// %H %I %m %M %p %S %y %Y are allowed; others are refused here). None
    /// when the input does not fit.
    pub fn parse_date(&self, template: &str, input: &[u8]) -> Result<Option<DateStamp>> {
        let mut it = template.bytes();
        while let Some(c) = it.next() {
            if c == b'%' {
                match it.next() {
                    Some(b'a' | b'A' | b'b' | b'B' | b'd' | b'e' | b'h' | b'H' | b'I' | b'm' | b'M' | b'p' | b'S' | b'y' | b'Y') => {}
                    _ => return Err(AmigaError::InvalidArgument("ParseDate template code not allowed")),
                }
            }
        }
        let t = c_copy(template.as_bytes())?;
        let src = c_copy(input)?;
        let pos = core::sync::atomic::AtomicUsize::new(0);
        let mut d = DateStamp::default();
        let ok = {
            let (s, p) = (&src, &pos);
            // ParseDate's hook returns the next input character, 0 at the
            // end.
            let hook = Hook::new(move |_loc: APTR, _msg: APTR| -> u32 {
                let k = p.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                s.get(k).copied().unwrap_or(0) as u32
            })?;
            // SAFETY: an open interface and locale, a DateStamp to fill, a
            // NUL-terminated template, a hook alive across the call.
            unsafe { ((*self.i).ParseDate)(self.i, self.loc, &mut d, t.as_ptr(), hook.as_ptr()) != 0 }
        };
        Ok(ok.then_some(d))
    }
}

/// One argument for [`Locale::format`].
#[derive(Clone, Copy, Debug)]
pub enum FmtArg<'a> {
    /// For `c d D u U x X` (32 bits; `u`/`U`/`x`/`X` read it unsigned).
    Int(i32),
    /// For `s`: bytes in the locale's code set (no NUL inside).
    Str(&'a [u8]),
}

/// Check every % command of `t` against `args` (see [`Locale::format`]).
fn check_format(t: &[u8], args: &[FmtArg<'_>]) -> Result<()> {
    let bad = |what: &'static str| Err(AmigaError::InvalidArgument(what));
    let mut next = 0usize;
    let mut k = 0usize;
    while k < t.len() {
        if t[k] != b'%' {
            k += 1;
            continue;
        }
        k += 1;
        if t.get(k) == Some(&b'%') {
            k += 1;
            continue;
        }
        // Optional position: digits then [U|L] then '$'.
        let start = k;
        let mut n = 0usize;
        while let Some(d @ b'0'..=b'9') = t.get(k) {
            n = n.saturating_mul(10).saturating_add((d - b'0') as usize);
            k += 1;
        }
        let mut pos = None;
        let mut j = k;
        if matches!(t.get(j), Some(b'U' | b'L')) {
            j += 1;
        }
        if k > start && t.get(j) == Some(&b'$') {
            if n == 0 {
                return bad("FormatString: argument positions start at 1");
            }
            pos = Some(n - 1);
            k = j + 1;
        } else {
            k = start;
        }
        if t.get(k) == Some(&b'-') {
            k += 1;
        }
        while matches!(t.get(k), Some(b'0'..=b'9')) {
            k += 1;
        }
        if t.get(k) == Some(&b'.') {
            k += 1;
            while matches!(t.get(k), Some(b'0'..=b'9')) {
                k += 1;
            }
        }
        match t.get(k) {
            Some(b'h') => return bad("FormatString: the h length is refused (32-bit arguments only)"),
            Some(b'l') => {
                k += 1;
                if t.get(k) == Some(&b'l') {
                    return bad("FormatString: the ll length is refused (32-bit arguments only)");
                }
            }
            _ => {}
        }
        let idx = pos.unwrap_or_else(|| {
            next += 1;
            next - 1
        });
        let Some(arg) = args.get(idx) else {
            return bad("FormatString: more commands than arguments");
        };
        match (t.get(k), arg) {
            (Some(b's'), FmtArg::Str(_)) => {}
            (Some(b'c' | b'd' | b'D' | b'u' | b'U' | b'x' | b'X'), FmtArg::Int(_)) => {}
            (Some(b's' | b'c' | b'd' | b'D' | b'u' | b'U' | b'x' | b'X'), _) => {
                return bad("FormatString: an argument of the wrong kind for its command")
            }
            _ => return bad("FormatString: a command type not supported here (b, p or unknown)"),
        }
        k += 1;
    }
    Ok(())
}

impl Drop for Locale {
    fn drop(&mut self) {
        // SAFETY: our locale, closed once.
        unsafe { ((*self.i).CloseLocale)(self.i, self.loc) };
    }
}

/// dos/errors.h: what OpenCatalog leaves in IoErr when there is no file.
const OBJECT_NOT_FOUND: i32 = 205;
const DIR_NOT_FOUND: i32 = 204;

/// How to open a catalog.
#[derive(Clone, Copy, Debug, Default)]
pub struct CatalogOptions<'a> {
    /// The language of the program's built-in strings (OC_BuiltInLanguage;
    /// default "english").
    pub builtin_language: Option<&'a str>,
    /// The catalog version wanted (OC_Version; 0: any).
    pub version: u32,
    /// A language to use instead of the user's (OC_Language).
    pub language: Option<&'a str>,
}

/// An open catalog (or none: the built-in strings); CloseCatalog on drop.
pub struct Catalog {
    _t: crate::track::Tracked,
    i: *mut LocaleIFace,
    cat: *mut sys::Catalog,
}

impl Catalog {
    /// OpenCatalogA(NULL, `name`, ...). A catalog that does not exist, or
    /// none needed (the user's language is the built-in one), gives a
    /// `Catalog` that answers with the built-in strings (IoErr 0 or
    /// "object not found"); only another failure is an `Err`. Call from a
    /// DOS Process (it reads files).
    #[track_caller]
    pub fn open(name: &str, opt: &CatalogOptions<'_>) -> Result<Self> {
        let i = iface()?;
        let n = c_copy(name.as_bytes())?;
        let bl = opt.builtin_language.map(|s| c_copy(s.as_bytes())).transpose()?;
        let lang = opt.language.map(|s| c_copy(s.as_bytes())).transpose()?;
        let mut tags = Vec::new();
        if let Some(b) = &bl {
            tags.push(TagItem { ti_Tag: sys::OC_BUILTIN_LANGUAGE, ti_Data: b.as_ptr() as u32 });
        }
        if opt.version != 0 {
            tags.push(TagItem { ti_Tag: sys::OC_VERSION, ti_Data: opt.version });
        }
        if let Some(l) = &lang {
            tags.push(TagItem { ti_Tag: sys::OC_LANGUAGE, ti_Data: l.as_ptr() as u32 });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // OpenCatalog leaves IoErr 0 when no catalog is needed: clear it
        // first so an old code is not read as a failure.
        clear_io_err();
        // SAFETY: an open interface; NULL locale (the autodoc's advice), a
        // NUL-terminated name, a TAG_DONE-ended list whose strings live
        // across the call.
        let cat = unsafe { ((*i).OpenCatalogA)(i, core::ptr::null_mut(), n.as_ptr(), tags.as_ptr()) };
        if cat.is_null() {
            // No catalog file is not a failure: the built-in strings
            // serve. locale.library 54 reports it with IoErr
            // ERROR_OBJECT_NOT_FOUND (205), not the 0 the autodoc implies.
            let e = io_err();
            if e != 0 && e != OBJECT_NOT_FOUND && e != DIR_NOT_FOUND {
                return Err(AmigaError::DosError(e));
            }
        }
        Ok(Self { i, cat, _t: crate::track::register("catalog", cat as usize, 0, release_catalog) })
    }

    /// Whether a catalog file was found (else the built-in strings serve).
    pub fn is_loaded(&self) -> bool {
        !self.cat.is_null()
    }

    /// GetCatalogStr: string `id`, or `builtin` when the catalog has none.
    /// The result lives as long as the catalog and `builtin`.
    pub fn get<'a>(&'a self, id: i32, builtin: &'a CStr) -> &'a [u8] {
        if self.cat.is_null() {
            return builtin.to_bytes();
        }
        // SAFETY: an open interface and catalog, a NUL-terminated default;
        // the result is the catalog's string or `builtin`.
        let p = unsafe { ((*self.i).GetCatalogStr)(self.i, self.cat, id, builtin.as_ptr() as *const u8) };
        bytes(p)
    }
}

impl Drop for Catalog {
    fn drop(&mut self) {
        // SAFETY: our catalog (or NULL, which CloseCatalog accepts), once.
        unsafe { ((*self.i).CloseCatalog)(self.i, self.cat) };
    }
}

#[cfg(feature = "dos")]
fn clear_io_err() {
    if let Ok(i) = crate::dos::process() {
        // SAFETY: an open interface in a Process.
        unsafe { ((*i).SetIoErr)(i, 0) };
    }
}

#[cfg(feature = "dos")]
fn io_err() -> i32 {
    crate::dos::io_err()
}

#[cfg(not(feature = "dos"))]
fn clear_io_err() {}

#[cfg(not(feature = "dos"))]
fn io_err() -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::{check_format, FmtArg};

    #[test]
    fn format_commands_are_checked_against_the_arguments() {
        let (i, s) = (FmtArg::Int(1), FmtArg::Str(b"x"));
        assert!(check_format(b"%ld %s %% %-8.3s %08lx", &[i, s, s, i]).is_ok());
        assert!(check_format(b"%2$s then %1$ld, %2U$s", &[i, s]).is_ok());
        assert!(check_format(b"no commands", &[]).is_ok());
        assert!(check_format(b"%ld %ld", &[i]).is_err(), "too few");
        assert!(check_format(b"%s", &[i]).is_err(), "int for s");
        assert!(check_format(b"%d", &[s]).is_err(), "string for d");
        assert!(check_format(b"%3$d", &[i, i]).is_err(), "position past the end");
        assert!(check_format(b"%0$d", &[i]).is_err(), "position 0");
        assert!(check_format(b"%hd", &[i]).is_err(), "h");
        assert!(check_format(b"%lld", &[i]).is_err(), "ll");
        assert!(check_format(b"%b", &[i]).is_err(), "b");
        assert!(check_format(b"%p", &[i]).is_err(), "p");
        assert!(check_format(b"trailing %", &[]).is_err(), "a lone % at the end");
    }
}
