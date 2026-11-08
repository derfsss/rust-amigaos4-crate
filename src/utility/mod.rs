//! utility.library, safe (feature `utility`).
//!
//! [`Utility::get`] gives the interface: the program's `IUtility` when it is
//! set, otherwise one the crate opens once (see `iface`). Everything else
//! hangs off it:
//!
//! - dates: [`Utility::amiga_to_date`], [`Utility::date_to_amiga`]
//!   (Amiga2Date, CheckDate, Date2Amiga);
//! - [`tags`]: checked tag lists and the calls that search, filter, map
//!   and clone them;
//! - [`names`]: named objects and namespaces;
//! - [`text`]: Stricmp and friends (locale-dependent), UTF-8 compare and
//!   UCS-4 case mapping (Unicode simple folding);
//! - [`Sha1`], [`Random`], [`Utility::unique_id`], and
//!   [`NameList::sort_by`](crate::list::NameList::sort_by) (SortList).
//!
//! Left out on purpose: the skip lists and splay trees (`BTreeMap` and
//! [`AvlMap`](crate::avl::AvlMap) do the job), the string copies,
//! `memset`-likes and printf-likes (Rust's own), Pack/UnpackStructureTags
//! (raw offset tables) and RemNamedObject outside [`names`].

pub mod names;
pub mod tags;
pub mod text;

use core::cmp::Ordering;

use crate::error::{AmigaError, Result};
use crate::sys::{self, ClockData, UtilityIFace};

crate::iface::iface!(iface, UtilityIFace, IUtility, b"utility.library\0", b"main\0", 50);

/// utility.library's main interface.
#[derive(Clone, Copy)]
pub struct Utility {
    pub(crate) i: *mut UtilityIFace,
}

// SAFETY: a library interface may be called from any task; the calls
// wrapped here keep no per-caller state in the interface.
unsafe impl Send for Utility {}
// SAFETY: as above.
unsafe impl Sync for Utility {}

/// The epoch of Amiga dates: 1978-01-01 00:00:00, a Sunday.
pub const EPOCH: ClockData = ClockData { sec: 0, min: 0, hour: 0, mday: 1, month: 1, year: 1978, wday: 0 };

impl Utility {
    /// The interface. `Err(NullPointer)` only if utility.library cannot be
    /// opened (it is in the Kickstart).
    pub fn get() -> Result<Self> {
        iface().map(|i| Self { i })
    }

    /// The library's version and revision.
    pub fn version(&self) -> (u16, u16) {
        // SAFETY: an open interface: its InterfaceData (at offset 0) names
        // the library base, whose lib_Version and lib_Revision are at 20
        // and 22 (tests/layout/exec.c).
        unsafe {
            let base = (*(self.i as *const sys::InterfaceData)).lib_base as *const u8;
            (core::ptr::read(base.add(20) as *const u16), core::ptr::read(base.add(22) as *const u16))
        }
    }

    /// At least version `v`.`r`, else `Err(Unsupported(what))`.
    pub(crate) fn require(&self, v: u16, r: u16, what: &'static str) -> Result<()> {
        if self.version() >= (v, r) { Ok(()) } else { Err(AmigaError::Unsupported(what)) }
    }

    /// Amiga2Date: seconds since 1978-01-01 as a date (the machine's time
    /// zone: no conversion).
    pub fn amiga_to_date(&self, seconds: u32) -> ClockData {
        let mut cd = ClockData::default();
        // SAFETY: an open interface and a ClockData to fill.
        unsafe { ((*self.i).Amiga2Date)(self.i, seconds, &mut cd) };
        cd
    }

    /// CheckDate, then Date2Amiga: the date's seconds since 1978-01-01, or
    /// None for an illegal date (31 February, hour 24, a year before 1978,
    /// past 2114...). `wday` is not checked (CheckDate's documented bug).
    ///
    /// CheckDate answers 0 both for an illegal date and for the epoch
    /// itself; the epoch is told apart here by comparing the fields.
    pub fn date_to_amiga(&self, date: &ClockData) -> Option<u32> {
        // SAFETY: an open interface and a ClockData it only reads.
        let s = unsafe { ((*self.i).CheckDate)(self.i, date) };
        if s != 0 {
            return Some(s);
        }
        let is_epoch = (date.sec, date.min, date.hour, date.mday, date.month, date.year) == (0, 0, 0, 1, 1, 1978);
        if is_epoch { Some(0) } else { None }
    }

    /// Date2Amiga without the check: for a date known to be legal (for
    /// example one from [`Utility::amiga_to_date`]). An illegal date gives
    /// a meaningless number, never undefined behaviour.
    pub fn date_to_amiga_unchecked(&self, date: &ClockData) -> u32 {
        // SAFETY: an open interface and a ClockData it only reads.
        unsafe { ((*self.i).Date2Amiga)(self.i, date) }
    }

    /// GetUniqueID: a number no other call returns (until 2^32 calls).
    pub fn unique_id(&self) -> u32 {
        // SAFETY: an open interface; callable from anywhere.
        unsafe { ((*self.i).GetUniqueID)(self.i) }
    }
}

/// SHA-1 (MessageDigest_SHA_*): 160 bits, not for cryptography.
#[derive(Clone)]
pub struct Sha1 {
    u: Utility,
    state: sys::MessageDigest_SHA,
}

impl Sha1 {
    pub fn new(u: Utility) -> Self {
        let mut state = sys::MessageDigest_SHA { mdsha_Code: [0; 20], mdsha_Reserved: [0; 328] };
        // SAFETY: an open interface and a state to initialise.
        unsafe { ((*u.i).MessageDigest_SHA_Init)(u.i, &mut state) };
        Self { u, state }
    }

    pub fn update(&mut self, data: &[u8]) {
        // num_bytes is a LONG: feed at most 1 MiB per call.
        for chunk in data.chunks(1 << 20) {
            // SAFETY: an initialised state and `chunk.len()` readable bytes
            // (the call only reads them).
            unsafe {
                ((*self.u.i).MessageDigest_SHA_Update)(self.u.i, &mut self.state, chunk.as_ptr() as sys::APTR, chunk.len() as i32)
            };
        }
    }

    pub fn finish(mut self) -> [u8; 20] {
        // SAFETY: an initialised state; the digest lands in mdsha_Code.
        unsafe { ((*self.u.i).MessageDigest_SHA_Final)(self.u.i, &mut self.state) };
        self.state.mdsha_Code
    }

    /// The digest of `data` in one call.
    pub fn digest(u: Utility, data: &[u8]) -> [u8; 20] {
        let mut h = Self::new(u);
        h.update(data);
        h.finish()
    }
}

/// utility.library's Random(): values in 1..=2^31-1 from a state the
/// caller owns (not for cryptography).
#[derive(Clone)]
pub struct Random {
    u: Utility,
    state: sys::RandomState,
}

impl Random {
    /// A generator seeded with `high` and `low`; `Err(InvalidArgument)`
    /// when both are zero (the autodoc: the state must never be zero).
    pub fn new(u: Utility, high: i32, low: i32) -> Result<Self> {
        if high == 0 && low == 0 {
            return Err(AmigaError::InvalidArgument("Random seed is zero"));
        }
        Ok(Self { u, state: sys::RandomState { rs_High: high, rs_Low: low } })
    }

    pub fn next_u31(&mut self) -> u32 {
        // SAFETY: an open interface and a state that is not zero.
        unsafe { ((*self.u.i).Random)(self.u.i, &mut self.state) }
    }
}

/// The sign of a C compare result as an Ordering.
pub(crate) fn ordering(r: i32) -> Ordering {
    r.cmp(&0)
}
