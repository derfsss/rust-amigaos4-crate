//! libtest: the crate's safe layers over utility.library, expansion/PCI,
//! dos.library and the other libraries, on a real AmigaOS.
//!
//!   libtest [utility|pci|dos|process|args|dos3|dos4|timer|net|locale|app|gfx|win|layers|wb|dt|con|elf|ain|all] (gui, wbgui, congui, aingui: need input)
//!
//! Prints one line per check and "libtest: N/M passed"; returns 0 when
//! every check passed, 20 otherwise. Lines starting "info:" record what
//! the OS answered where the autodocs leave it open.
//!
//! The glue leaves `IUtility` NULL, so `Utility::get` opens the library
//! itself: that path is what runs here.
//!
//! utility:
//! - the interface, opened by the crate; the same one on a second get;
//! - dates: the epoch, a day later, leap days, illegal dates refused, the
//!   epoch told apart from an illegal date, 4000 round trips checked
//!   against a Rust date computation;
//! - SHA-1 against the FIPS 180-1 vectors, one million 'a' fed in odd
//!   chunk sizes;
//! - Random: a zero seed refused, the range, the same seed twice;
//! - GetUniqueID from 4 tasks at once: 20000 numbers, all different;
//! - Stricmp/Strnicmp/ToLower/ToUpper, a NUL refused; UTF-8 compare and
//!   UCS-4 case mapping, UCS4Valid's range, lossy UTF-8 decoding;
//! - tag lists: refused without TAG_DONE, with TAG_MORE, skipping past
//!   the end; find/get/iterate (TAG_IGNORE and TAG_SKIP); filter, map,
//!   apply, filter_changes, PackBoolTags, TagInArray; a clone changed and
//!   refreshed;
//! - named objects: user space, publish, find, duplicates in a NODUPS
//!   namespace, case, find_all, a drop that waits for another task's
//!   holder, memory around 300 cycles;
//! - NameList::sort_by (SortList).

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt::{self, Write};
use core::panic::PanicInfo;

use amigaos4::allocator::SharedExecAllocator;
use amigaos4::error::AmigaError;
use amigaos4::dos;
use amigaos4::list::NameList;
use amigaos4::pci::{cfg, BarKind, LockMode, Pci, PciDevice, Query};
use amigaos4::sys::{self, ClockData, TagItem, TAG_DONE, TAG_IGNORE, TAG_MORE, TAG_SKIP};
use amigaos4::task::{self, Builder};
use amigaos4::utility::names::{self, Named, Options, Space};
use amigaos4::utility::tags::{Filter, TagClone, TagList, TagListMut};
use amigaos4::utility::{Random, Sha1, Utility};

#[global_allocator]
static ALLOCATOR: SharedExecAllocator = SharedExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

struct Out;
impl Write for Out {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buf = [0u8; 128];
        for chunk in s.as_bytes().chunks(buf.len() - 1) {
            buf[..chunk.len()].copy_from_slice(chunk);
            buf[chunk.len()] = 0;
            // SAFETY: a NUL-terminated buffer on the stack.
            unsafe { sys::dos_put_str(buf.as_ptr() as _) };
        }
        Ok(())
    }
}
/// One line to the Shell and, since a redirected Shell output only
/// reaches its file when the program ends, to the serial debug output.
macro_rules! say {
    ($($arg:tt)*) => {{
        let _ = writeln!(Out, $($arg)*);
        amigaos4::serial_println!("libtest: {}", format_args!($($arg)*));
    }};
}

struct Tally {
    pass: u32,
    total: u32,
}
impl Tally {
    fn check(&mut self, ok: bool, what: fmt::Arguments) {
        self.total += 1;
        if ok {
            self.pass += 1;
        }
        say!("{} {}", if ok { "PASS" } else { "FAIL" }, what);
    }
}

fn avail() -> u32 {
    // SAFETY: AvailMem only reads.
    unsafe { sys::exec_avail_mem(0) }
}

fn timebase_hz() -> u64 {
    let mut hz = 0u64;
    let tags = [
        TagItem { ti_Tag: sys::GCIT_TIMEBASE_SPEED, ti_Data: &mut hz as *mut u64 as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: the tag points at a u64 on this stack.
    unsafe { sys::exec_get_cpuinfo(tags.as_ptr()) };
    hz.max(1)
}

fn ms_since(t0: u64) -> u64 {
    (amigaos4::arch::timebase() - t0) * 1000 / timebase_hz()
}

/// Busy-wait `ms` milliseconds (no OS call: usable in an exec task).
fn busy_ms(ms: u64) {
    let t0 = amigaos4::arch::timebase();
    let end = timebase_hz() * ms / 1000;
    while amigaos4::arch::timebase() - t0 < end {
        core::hint::spin_loop();
    }
}

fn hex(d: &[u8]) -> String {
    let mut s = String::new();
    for b in d {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

// ---------------------------------------------------------------------------
// utility: dates
// ---------------------------------------------------------------------------

/// Days since 1970-01-01 for a civil date (H. Hinnant's days_from_civil).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The date `secs` after 1978-01-01, computed in Rust.
fn rust_date(secs: u32) -> ClockData {
    let base = days_from_civil(1978, 1, 1);
    let days = secs as i64 / 86400;
    let rem = secs as i64 % 86400;
    // civil_from_days
    let z = base + days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    // 1978-01-01 was a Sunday (wday 0).
    let wday = (days % 7) as u16;
    ClockData {
        sec: (rem % 60) as u16,
        min: (rem / 60 % 60) as u16,
        hour: (rem / 3600) as u16,
        mday: d as u16,
        month: m as u16,
        year: y as u16,
        wday,
    }
}

fn date(y: u16, m: u16, d: u16, h: u16, mi: u16, s: u16) -> ClockData {
    ClockData { sec: s, min: mi, hour: h, mday: d, month: m, year: y, wday: 0 }
}

fn test_dates(t: &mut Tally, u: Utility) {
    let e = u.amiga_to_date(0);
    t.check(e == amigaos4::utility::EPOCH, format_args!("date: second 0 is 1978-01-01 00:00:00, a Sunday ({:?})", e));
    t.check(u.date_to_amiga(&e) == Some(0), format_args!("date: the epoch is a legal date (CheckDate's 0 told apart)"));
    let d1 = u.amiga_to_date(86400 + 3661);
    t.check(
        (d1.year, d1.month, d1.mday, d1.hour, d1.min, d1.sec, d1.wday) == (1978, 1, 2, 1, 1, 1, 1),
        format_args!("date: 90061 s is 1978-01-02 01:01:01, a Monday"),
    );
    let ok = |y, m, d| u.date_to_amiga(&date(y, m, d, 0, 0, 0)).is_some();
    t.check(ok(1980, 2, 29) && !ok(1981, 2, 29) && ok(2000, 2, 29) && !ok(2100, 2, 29), format_args!("date: leap days 1980 and 2000 yes, 1981 and 2100 no"));
    let bad = [
        date(1990, 2, 31, 0, 0, 0),
        date(1990, 4, 31, 0, 0, 0),
        date(1990, 0, 1, 0, 0, 0),
        date(1990, 13, 1, 0, 0, 0),
        date(1990, 1, 0, 0, 0, 0),
        date(1990, 1, 1, 24, 0, 0),
        date(1990, 1, 1, 0, 60, 0),
        date(1990, 1, 1, 0, 0, 60),
        date(1977, 12, 31, 23, 59, 59),
    ];
    let refused = bad.iter().filter(|d| u.date_to_amiga(d).is_none()).count();
    t.check(refused == bad.len(), format_args!("date: {}/{} illegal dates refused (31 Feb, 31 Apr, month 0/13, day 0, 24:00, :60, 1977)", refused, bad.len()));
    let mut wrong_wday = date(1990, 6, 15, 12, 0, 0);
    wrong_wday.wday = 6;
    t.check(u.date_to_amiga(&wrong_wday).is_some(), format_args!("date: a wrong wday is not checked (the documented bug)"));

    // 4000 round trips across the whole range, against Rust.
    let mut bad_rt = 0;
    let mut bad_rust = 0;
    let mut first = None;
    for i in 0..4000u32 {
        let s = i.wrapping_mul(1_073_807) ^ (i << 7);
        let d = u.amiga_to_date(s);
        if d != rust_date(s) {
            bad_rust += 1;
            if first.is_none() {
                first = Some((s, d, rust_date(s)));
            }
        }
        if u.date_to_amiga(&d) != Some(s) {
            bad_rt += 1;
        }
    }
    if let Some((s, d, r)) = first {
        say!("info: date {} OS {:?} Rust {:?}", s, d, r);
    }
    t.check(bad_rt == 0 && bad_rust == 0, format_args!("date: 4000 seconds to date and back ({} wrong), each date as Rust computes it ({} differ)", bad_rt, bad_rust));
    let last = u.amiga_to_date(u32::MAX);
    say!("info: date: the last second, {}: {:?}", u32::MAX, last);
}

// ---------------------------------------------------------------------------
// utility: SHA-1, Random, unique IDs
// ---------------------------------------------------------------------------

fn test_sha(t: &mut Tally, u: Utility) {
    let v = [
        (&b""[..], "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
        (&b"abc"[..], "a9993e364706816aba3e25717850c26c9cd0d89d"),
        (&b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"[..], "84983e441c3bd26ebaae4aa1f95129e5e54670f1"),
    ];
    for (data, want) in v {
        let got = hex(&Sha1::digest(u, data));
        t.check(got == want, format_args!("SHA-1 of {} bytes: {}", data.len(), got));
    }
    let a = alloc::vec![b'a'; 1000];
    let mut all = true;
    for step in [1usize, 63, 64, 65, 1000] {
        let mut h = Sha1::new(u);
        let mut left = 1_000_000usize;
        while left > 0 {
            let n = step.min(left).min(a.len());
            h.update(&a[..n]);
            left -= n;
        }
        let got = hex(&h.finish());
        if got != "34aa973cd4c4daa4f61eeb2bdbad27316534016f" {
            all = false;
            say!("info: SHA-1 of a million 'a' in steps of {}: {}", step, got);
        }
    }
    t.check(all, format_args!("SHA-1 of one million 'a', fed 1, 63, 64, 65 and 1000 bytes at a time"));
    let mut h = Sha1::new(u);
    h.update(b"ab");
    let h2 = h.clone();
    h.update(b"c");
    t.check(hex(&h.finish()) == v[1].1 && hex(&h2.finish()) == "da23614e02469a0d7c7bd1bdab5c9c474b1904dc", format_args!("SHA-1: a clone carries on from the same state"));
}

fn test_random(t: &mut Tally, u: Utility) {
    t.check(matches!(Random::new(u, 0, 0), Err(AmigaError::InvalidArgument(_))), format_args!("Random: a zero seed is refused"));
    let (Ok(mut a), Ok(mut b)) = (Random::new(u, 12345, 678), Random::new(u, 12345, 678)) else {
        return t.check(false, format_args!("Random::new"));
    };
    let mut same = true;
    let mut in_range = true;
    let mut first = [0u32; 4];
    for i in 0..10000 {
        let (x, y) = (a.next_u31(), b.next_u31());
        same &= x == y;
        in_range &= (1..=0x7fff_ffff).contains(&x);
        if i < 4 {
            first[i] = x;
        }
    }
    say!("info: Random(12345, 678) starts {:?}", first);
    t.check(same && in_range, format_args!("Random: 10000 values in 1..2^31-1, the same seed gives the same sequence"));
}

fn test_unique(t: &mut Tally, u: Utility) {
    let mut hs = Vec::new();
    for _ in 0..4 {
        let h = Builder::new(b"libtest ids\0").priority(task::priority().saturating_sub(1)).stack(16384).spawn(move || {
            let mut v = Vec::with_capacity(5000);
            for _ in 0..5000 {
                v.push(u.unique_id());
            }
            v
        });
        match h {
            Ok(h) => hs.push(h),
            Err(e) => return t.check(false, format_args!("spawn: {}", e)),
        }
    }
    let mut all: Vec<u32> = Vec::new();
    for h in hs {
        if let Ok(v) = h.join() {
            all.extend(v);
        }
    }
    let n = all.len();
    all.sort_unstable();
    all.dedup();
    t.check(n == 20000 && all.len() == 20000, format_args!("GetUniqueID: 4 tasks x 5000, {} of {} different", all.len(), n));
}

// ---------------------------------------------------------------------------
// utility: text
// ---------------------------------------------------------------------------

fn test_text(t: &mut Tally, u: Utility) {
    let eq = u.stricmp(b"Hello World", b"hELLO wORLD");
    let lt = u.stricmp(b"apple", b"Banana");
    let short = u.stricmp(b"abc", b"ABCD");
    t.check(eq == Ok(Ordering::Equal) && lt == Ok(Ordering::Less) && short == Ok(Ordering::Less), format_args!("Stricmp: equal without case, apple < Banana, abc < ABCD"));
    t.check(u.strnicmp(b"abcX", b"ABCy", 3) == Ok(Ordering::Equal) && u.strnicmp(b"abcX", b"ABCy", 4) == Ok(Ordering::Less), format_args!("Strnicmp: 3 bytes equal, 4 not"));
    t.check(matches!(u.stricmp(b"a\0b", b"a"), Err(AmigaError::InvalidArgument(_))), format_args!("Stricmp: a string with a NUL inside is refused"));
    t.check(u.to_lower(b'Q') == b'q' && u.to_upper(b'q') == b'Q' && u.to_upper(b'1') == b'1', format_args!("ToLower/ToUpper on ASCII"));
    say!("info: ToUpper(0xE9) = 0x{:02x}, ToLower(0xC9) = 0x{:02x} (Latin-1 e acute; locale-dependent)", u.to_upper(0xE9), u.to_lower(0xC9));

    let r = u.utf8_stricmp("Ärger Öl Übel", "ärger öl üBEL");
    t.check(r == Ok(Ordering::Equal), format_args!("UTF8Stricmp: German umlauts compare equal without case ({:?})", r));
    let r2 = (u.utf8_stricmp("a", "b"), u.utf8_stricmp("ÿ", "a"));
    t.check(r2 == (Ok(Ordering::Less), Ok(Ordering::Greater)), format_args!("UTF8Stricmp: a < b, y-diaeresis > a ({:?})", r2));
    let r3 = (u.utf8_strnicmp("ÄÖXa", "äöXb", 3), u.utf8_strnicmp("ÄÖXa", "äöXb", 4), u.utf8_strnicmp("x", "y", 0));
    t.check(r3 == (Ok(Ordering::Equal), Ok(Ordering::Less), Ok(Ordering::Equal)), format_args!("UTF8Strnicmp counts characters, not bytes ({:?})", r3));

    let maps = [
        ('A', u.ucs4_to_lower('A')),
        ('Σ', u.ucs4_to_lower('Σ')),
        ('é', u.ucs4_to_upper('é')),
        ('ß', u.ucs4_to_upper('ß')),
        ('İ', u.ucs4_to_lower('İ')),
    ];
    let ok = maps[0].1 == Ok('a') && maps[1].1 == Ok('σ') && maps[2].1 == Ok('É') && maps[3].1 == Ok('ß');
    t.check(ok, format_args!("UCS4ToLower/Upper: A->a, Sigma->sigma, e acute->E acute, sharp s stays (simple folding)"));
    say!("info: UCS4ToLower(U+0130) = {:?} (Rust's to_lowercase gives two characters)", maps[4].1.map(|c| c as u32));
    let probes = [0i32, 1, 0xD7FF, 0xD800, 0xDFFF, 0xE000, 0xFFFE, 0xFFFF, 0x10FFFF, 0x110000, -1];
    let mut line = String::new();
    for p in probes {
        let _ = write!(line, " {:#x}={}", p, u.ucs4_valid(p).map(|b| b as u8).unwrap_or(9));
    }
    say!("info: UCS4Valid:{}", line);
    t.check(u.ucs4_valid(0x41) == Ok(true) && u.ucs4_valid(0x110000) == Ok(false), format_args!("UCS4Valid: 'A' yes, U+110000 no"));

    let s = u.utf8_decode_lossy(b"a\xc3\xa9\xffz\xe2\x82");
    say!("info: UTF8Decode lossy of a, e acute, 0xFF, z, a cut euro sign: {:?}", s);
    t.check(
        s.as_deref().map(|s| s.starts_with("aé") && s.contains('z') && s.contains('\u{FFFD}')).unwrap_or(false),
        format_args!("UTF8Decode: valid text decoded, bad bytes replaced by U+FFFD"),
    );
    t.check(u.utf8_decode_lossy(b"ok\0no").is_err(), format_args!("UTF8Decode: bytes with a NUL inside are refused"));
}

// ---------------------------------------------------------------------------
// utility: tags
// ---------------------------------------------------------------------------

const TA: u32 = sys::TAG_USER + 1;
const TB: u32 = sys::TAG_USER + 2;
const TC: u32 = sys::TAG_USER + 3;
const TX: u32 = sys::TAG_USER + 10;

fn ti(tag: u32, data: u32) -> TagItem {
    TagItem { ti_Tag: tag, ti_Data: data }
}

fn tags_of(l: &TagList<'_>) -> Vec<(u32, u32)> {
    l.iter().map(|i| (i.ti_Tag - sys::TAG_USER, i.ti_Data)).collect()
}

fn test_tags(t: &mut Tally, u: Utility) {
    let no_done = [ti(TA, 1), ti(TB, 2)];
    let more = [ti(TA, 1), ti(TAG_MORE, 0x1000), ti(TAG_DONE, 0)];
    let skip_out = [ti(TA, 1), ti(TAG_SKIP, 3), ti(TAG_DONE, 0)];
    t.check(
        TagList::new(u, &no_done).is_err() && TagList::new(u, &more).is_err() && TagList::new(u, &skip_out).is_err(),
        format_args!("tags: a list without TAG_DONE, with TAG_MORE, or skipping past its end is refused"),
    );

    let items = [ti(TA, 1), ti(TAG_IGNORE, 99), ti(TB, 2), ti(TAG_SKIP, 1), ti(TC, 98), ti(TC, 3), ti(TAG_DONE, 0), ti(TX, 7)];
    let Ok(l) = TagList::new(u, &items) else { return t.check(false, format_args!("tags: a good list refused")) };
    let it = tags_of(&l);
    t.check(it == [(1, 1), (2, 2), (3, 3)] && l.as_slice().len() == 7, format_args!("tags: NextTagItem skips TAG_IGNORE and a TAG_SKIP'd item, stops at TAG_DONE: {:?}", it));
    t.check(
        l.find(TC).map(|i| i.ti_Data) == Some(3) && l.find(TX).is_none() && l.get(TB, 0) == 2 && l.get(TX, 77) == 77,
        format_args!("tags: FindTagItem finds C=3 (not the skipped one), not X after TAG_DONE; GetTagData with a default"),
    );

    let mut m = [ti(TA, 1), ti(TB, 2), ti(TC, 3), ti(TAG_DONE, 0)];
    let left = TagListMut::new(u, &mut m).and_then(|mut l| l.filter(&[TA, TC, TAG_DONE], Filter::Keep));
    t.check(left == Ok(2) && m[1].ti_Tag == TAG_IGNORE && m[0].ti_Tag == TA, format_args!("tags: FilterTagItems keeps A and C, B becomes TAG_IGNORE ({:?})", left));
    let mut m = [ti(TA, 1), ti(TB, 2), ti(TC, 3), ti(TAG_DONE, 0)];
    let left = TagListMut::new(u, &mut m).and_then(|mut l| l.filter(&[TA, TAG_DONE], Filter::Drop));
    t.check(left == Ok(2) && m[0].ti_Tag == TAG_IGNORE, format_args!("tags: FilterTagItems NOT drops A"));
    t.check(TagListMut::new(u, &mut m).and_then(|mut l| l.filter(&[TA], Filter::Drop)).is_err(), format_args!("tags: a filter array without TAG_DONE is refused"));

    let map = [ti(TA, TX), ti(TAG_DONE, 0)];
    let mut m = [ti(TA, 1), ti(TB, 2), ti(TAG_DONE, 0)];
    let keep = TagList::new(u, &map).and_then(|mp| TagListMut::new(u, &mut m).and_then(|mut l| l.map(&mp, true)));
    let kept = (m[0].ti_Tag, m[1].ti_Tag);
    let mut m = [ti(TA, 1), ti(TB, 2), ti(TAG_DONE, 0)];
    let rem = TagList::new(u, &map).and_then(|mp| TagListMut::new(u, &mut m).and_then(|mut l| l.map(&mp, false)));
    t.check(
        keep.is_ok() && rem.is_ok() && kept == (TX, TB) && (m[0].ti_Tag, m[1].ti_Tag) == (TX, TAG_IGNORE),
        format_args!("tags: MapTags A->X, B kept or removed"),
    );
    let bad_map = [ti(TA, TAG_MORE), ti(TAG_DONE, 0)];
    let mut m = [ti(TA, 1), ti(TAG_DONE, 0)];
    let r = TagList::new(u, &bad_map).and_then(|mp| TagListMut::new(u, &mut m).and_then(|mut l| l.map(&mp, true)));
    t.check(r.is_err() && m[0].ti_Tag == TA, format_args!("tags: a map to TAG_MORE is refused and changes nothing"));

    let ch = [ti(TB, 20), ti(TX, 70), ti(TAG_DONE, 0)];
    let mut m = [ti(TA, 1), ti(TB, 2), ti(TAG_DONE, 0)];
    if let (Ok(c), Ok(mut l)) = (TagList::new(u, &ch), TagListMut::new(u, &mut m)) {
        l.apply(&c);
    }
    t.check(m[1].ti_Data == 20 && m[0].ti_Data == 1, format_args!("tags: ApplyTagChanges B=20, X (not in the list) ignored"));

    let mut orig = [ti(TA, 1), ti(TB, 2), ti(TC, 3), ti(TAG_DONE, 0)];
    let mut change = [ti(TA, 1), ti(TB, 5), ti(TAG_DONE, 0)];
    if let (Ok(mut c), Ok(mut o)) = (TagListMut::new(u, &mut change), TagListMut::new(u, &mut orig)) {
        c.filter_changes(&mut o, true);
    }
    t.check(
        change[0].ti_Tag == TAG_IGNORE && change[1].ti_Tag == TB && orig[1].ti_Data == 5,
        format_args!("tags: FilterTagChanges: the same A is ignored, the new B applied"),
    );

    // The autodoc's PackBoolTags example: flags 0x800002, tag1 TRUE, tag2
    // FALSE, tag3 TRUE, a map of 0x0001 / 0x0002 / 0x0004.
    let list = [ti(TA, 1), ti(TB, 0), ti(TC, 1), ti(TAG_DONE, 0)];
    let bmap = [ti(TA, 1), ti(TB, 2), ti(TC, 4), ti(TAG_DONE, 0)];
    let r = TagList::new(u, &list).and_then(|l| TagList::new(u, &bmap).map(|b| l.pack_bool(0x800002, &b)));
    t.check(r == Ok(0x800005), format_args!("tags: PackBoolTags 0x800002 -> {:x?} (want 0x800005; the autodoc prints 0x80005)", r));
    t.check(
        u.tag_in_array(TB, &[TA, TB, TAG_DONE]) == Ok(true) && u.tag_in_array(TX, &[TA, TB, TAG_DONE]) == Ok(false) && u.tag_in_array(TA, &[TA]).is_err(),
        format_args!("tags: TagInArray, an array without TAG_DONE refused"),
    );

    let src = [ti(TA, 1), ti(TAG_IGNORE, 0), ti(TB, 2), ti(TAG_DONE, 0)];
    let Ok(sl) = TagList::new(u, &src) else { return };
    match TagClone::new(sl) {
        Ok(mut c) => {
            let before = tags_of(&c.as_list());
            say!("info: CloneTagItems of A, TAG_IGNORE, B gives {} items through TAG_DONE", c.as_list().as_slice().len());
            if let Ok(mut m) = c.as_mut() {
                let _ = m.filter(&[TA, TAG_DONE], Filter::Drop);
            }
            let changed = tags_of(&c.as_list());
            c.refresh();
            let after = tags_of(&c.as_list());
            t.check(
                before == [(1, 1), (2, 2)] && changed == [(2, 2)] && after == before,
                format_args!("tags: a clone has A and B, filtered it has B, refreshed A and B again ({:?})", after),
            );
        }
        Err(e) => t.check(false, format_args!("CloneTagItems: {}", e)),
    }
    let m0 = avail();
    for _ in 0..2000 {
        let _ = TagClone::new(sl);
    }
    say!("info: tags: free memory around 2000 clones: {:+} bytes", avail() as i64 - m0 as i64);
}

// ---------------------------------------------------------------------------
// utility: named objects
// ---------------------------------------------------------------------------

fn test_names(t: &mut Tally, u: Utility) {
    let root = Space::root();
    let name = "libtest.object";
    let mut n = match Named::new(u, name, Options { user_space: 64, ..Default::default() }) {
        Ok(n) => n,
        Err(e) => return t.check(false, format_args!("Named::new: {}", e)),
    };
    let us = n.user_space();
    let aligned = us.as_ptr() as usize % 4 == 0;
    let len = us.len();
    us.copy_from_slice(&[0x5a; 64]);
    t.check(n.name() == name && len == 64 && aligned, format_args!("names: an object with 64 bytes of long-aligned user space, named {}", n.name()));
    let none = names::find(u, root, name).map(|f| f.is_none());
    t.check(none == Ok(true), format_args!("names: not found before it is published"));
    let p = match n.publish(root) {
        Ok(p) => p,
        Err(_) => return t.check(false, format_args!("names: publish into the root namespace")),
    };
    let found = names::find(u, root, "LIBTEST.Object");
    let ok = match &found {
        Ok(Some(f)) => f.name() == name && f.object() == p.object() && {
            // SAFETY: the user space of an object we published, 64 bytes,
            // not written by anyone while we read it.
            unsafe { *(f.object() as *const u8).add(63) == 0x5a }
        },
        _ => false,
    };
    t.check(ok, format_args!("names: found without case in the root namespace, the same user space"));
    drop(found);

    // A NODUPS namespace inside an object, with case.
    let space_obj = match Named::new(u, "libtest.space", Options { namespace: true, no_dups: true, case: true, ..Default::default() }) {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("names: a namespace object: {}", e)),
    };
    let Some(sp) = space_obj.space() else { return t.check(false, format_args!("names: the object has a namespace")) };
    let a = Named::new(u, "dup", Options::default()).ok().and_then(|n| n.publish(sp).ok());
    let b = Named::new(u, "dup", Options::default()).map(|n| n.publish(sp).is_err());
    let c = Named::new(u, "Dup", Options::default()).ok().and_then(|n| n.publish(sp).ok());
    t.check(a.is_some() && b == Ok(true) && c.is_some(), format_args!("names: a NODUPS namespace refuses a second \"dup\", takes \"Dup\" (case)"));
    let f_case = names::find(u, sp, "DUP").map(|f| f.is_none());
    let all = names::find_all(u, sp, None).map(|v| v.len());
    t.check(f_case == Ok(true) && all == Ok(2), format_args!("names: with case \"DUP\" is not found; all objects: {:?}", all));
    drop((a, c));
    t.check(names::find_all(u, sp, None).map(|v| v.len()) == Ok(0), format_args!("names: dropped objects leave the namespace"));

    // Duplicates in the root (no NODUPS there): both found, in priority order.
    let hi = Named::new(u, "libtest.twin", Options { priority: 10, ..Default::default() }).ok().and_then(|n| n.publish(root).ok());
    let lo = Named::new(u, "libtest.twin", Options { priority: -10, user_space: 4, ..Default::default() }).ok().and_then(|n| n.publish(root).ok());
    let twins = names::find_all(u, root, Some("libtest.twin")).unwrap_or_default();
    let order_ok = twins.len() == 2 && twins[0].object().is_null() && !twins[1].object().is_null();
    t.check(hi.is_some() && lo.is_some() && order_ok, format_args!("names: two objects of one name in the root, found by priority ({})", twins.len()));
    drop(twins);
    drop((hi, lo));

    // A holder in another task: dropping the Published waits for it.
    let Ok(Some(f)) = names::find(u, root, name) else { return t.check(false, format_args!("names: find for the holder")) };
    let holder = Builder::new(b"libtest holder\0").priority(task::priority().saturating_sub(1)).stack(16384).spawn(move || {
        busy_ms(300);
        drop(f);
    });
    let Ok(holder) = holder else { return t.check(false, format_args!("names: spawn the holder")) };
    let t0 = amigaos4::arch::timebase();
    drop(p);
    let ms = ms_since(t0);
    let _ = holder.join();
    let gone = names::find(u, root, name).map(|f| f.is_none());
    t.check((250..2000).contains(&ms) && gone == Ok(true), format_args!("names: the drop waited {} ms for the other task's holder, then the object was gone", ms));

    let m0 = avail();
    for i in 0..300u32 {
        if let Ok(n) = Named::new(u, "libtest.cycle", Options { user_space: 32 + i % 64, ..Default::default() }) {
            if let Ok(p) = n.publish(root) {
                let f = names::find(u, root, "libtest.cycle");
                drop(f);
                drop(p);
            }
        }
    }
    say!("info: names: free memory around 300 make/publish/find/drop cycles: {:+} bytes", avail() as i64 - m0 as i64);
}

fn test_sort(t: &mut Tally) {
    let Ok(mut l) = NameList::new(0) else { return t.check(false, format_args!("NameList::new")) };
    let mut names: Vec<String> = Vec::new();
    for i in 0..100u32 {
        let s = alloc::format!("n{:05}", i.wrapping_mul(7919) % 1000);
        let _ = l.push_back(&s, (i % 3) as i8);
        names.push(s);
    }
    let r = l.sort_by(|a, b| a.name().cmp(b.name()));
    names.sort();
    let got: Vec<&str> = l.iter().map(|n| n.name()).collect();
    t.check(r.is_ok() && got.iter().zip(names.iter()).all(|(a, b)| *a == b.as_str()) && got.len() == 100, format_args!("SortList: 100 nodes sorted by name ({:?})", r));

    // Equal keys: is the order kept?
    let Ok(mut e) = NameList::new(0) else { return };
    for i in 0..50u32 {
        let _ = e.push_back(&alloc::format!("{:02}", i), (i % 2) as i8);
    }
    let _ = e.sort_by(|a, b| a.pri().cmp(&b.pri()));
    let order: Vec<&str> = e.iter().map(|n| n.name()).collect();
    let stable = order.windows(2).all(|w| {
        let (a, b) = (w[0], w[1]);
        let (pa, pb) = (a.parse::<u32>().unwrap_or(0) % 2, b.parse::<u32>().unwrap_or(0) % 2);
        pa < pb || (pa == pb && a < b)
    });
    say!("info: SortList keeps the order of equal keys: {}", stable);
    t.check(e.iter().map(|n| n.pri()).collect::<Vec<_>>().windows(2).all(|w| w[0] <= w[1]), format_args!("SortList: 50 nodes sorted by priority"));
}


/// Raw named-object calls: which release brings RemNamedObject's reply,
/// and is the object gone then. Frees nothing.
fn probe_names() {
    // SAFETY: probe code: utility.library opened here; the objects, port
    // and message are made and used only here and never freed (a leak).
    unsafe {
        let base = sys::exec_open_library(b"utility.library\0".as_ptr(), 50);
        let i = sys::exec_get_interface(base, b"main\0".as_ptr(), 1, core::ptr::null()) as *mut sys::UtilityIFace;
        if i.is_null() {
            return say!("probe: no IUtility");
        }
        let tags = [TagItem { ti_Tag: sys::ANO_NAMESPACE, ti_Data: 0 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        let n = b"probe.rem2\0";
        let o = ((*i).AllocNamedObjectA)(i, n.as_ptr(), tags.as_ptr());
        let add = ((*i).AddNamedObject)(i, core::ptr::null_mut(), o);
        let held = ((*i).FindNamedObject)(i, core::ptr::null_mut(), n.as_ptr(), core::ptr::null_mut());
        let own = ((*i).FindNamedObject)(i, core::ptr::null_mut(), n.as_ptr(), core::ptr::null_mut());
        let r = ((*i).AttemptRemNamedObject)(i, o);
        say!("probe: add {} held {} own {} attempt {}", add, held == o, own == o, r);
        let port = sys::exec_alloc_sys_object(sys::ASOT_PORT, core::ptr::null()) as *mut sys::MsgPort;
        let mt = [TagItem { ti_Tag: sys::ASOMSG_REPLYPORT, ti_Data: port as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        let msg = sys::exec_alloc_sys_object(sys::ASOT_MESSAGE, mt.as_ptr()) as *mut sys::Message;
        let got = |step: &str| {
            let m = sys::exec_get_msg(port);
            let findable = {
                let f = ((*i).FindNamedObject)(i, core::ptr::null_mut(), n.as_ptr(), core::ptr::null_mut());
                if !f.is_null() {
                    ((*i).ReleaseNamedObject)(i, f);
                }
                f == o
            };
            say!("probe: after {}: reply {} (ours {}), findable {}", step, !m.is_null(), m == msg, findable);
            !m.is_null()
        };
        ((*i).RemNamedObject)(i, o, msg);
        if got("RemNamedObject") {
            return;
        }
        ((*i).ReleaseNamedObject)(i, held);
        if got("the other holder's release") {
            let own2 = ((*i).FindNamedObject)(i, core::ptr::null_mut(), n.as_ptr(), core::ptr::null_mut());
            let a2 = ((*i).AttemptRemNamedObject)(i, o);
            let f = ((*i).FindNamedObject)(i, core::ptr::null_mut(), n.as_ptr(), core::ptr::null_mut());
            say!("probe: after the reply: found again {}, attempt {}, findable {}", own2 == o, a2, f == o);
            return;
        }
        ((*i).ReleaseNamedObject)(i, o);
        let _ = got("the creator's release");
    }
}

fn run_utility(t: &mut Tally) {
    // SAFETY: reading the glue's global (left NULL on purpose).
    let global_null = unsafe { sys::IUtility }.is_null();
    let u = match Utility::get() {
        Ok(u) => u,
        Err(e) => return t.check(false, format_args!("Utility::get: {}", e)),
    };
    let (v, r) = u.version();
    t.check(global_null && v >= 50 && Utility::get().map(|u2| u2.version()) == Ok((v, r)), format_args!("utility: opened by the crate (IUtility left NULL), utility.library {}.{}", v, r));
    say!("-- {}", "test_dates");
    test_dates(t, u);
    say!("-- {}", "test_sha");
    test_sha(t, u);
    say!("-- {}", "test_random");
    test_random(t, u);
    say!("-- {}", "test_unique");
    test_unique(t, u);
    say!("-- {}", "test_text");
    test_text(t, u);
    say!("-- {}", "test_tags");
    test_tags(t, u);
    say!("-- {}", "test_names");
    test_names(t, u);
    say!("-- {}", "test_sort");
    test_sort(t);
}

// ---------------------------------------------------------------------------
// expansion and pci
// ---------------------------------------------------------------------------

fn run_expansion(t: &mut Tally) {
    match amigaos4::expansion::machine_info() {
        Ok(m) => {
            say!(
                "info: machine {} \"{}\", {} MB available, total {:?}, extensions \"{}\", {} tags understood",
                m.machine, m.name, m.memory >> 20, m.total_physical.map(|b| b >> 20), m.extensions, m.tags_understood
            );
            // SAFETY: reading the glue's global (left NULL on purpose).
            let null = unsafe { sys::IExpansion }.is_null();
            t.check(
                null && m.machine == sys::MACHINETYPE_AMIGAONE && !m.name.is_empty() && m.memory > 0,
                format_args!("expansion: GetMachineInfo through the crate's own interface: an AmigaOne (type {})", m.machine),
            );
        }
        Err(e) => t.check(false, format_args!("expansion::machine_info: {}", e)),
    }
}

/// The capability list walked by hand from 0x34.
fn manual_caps(d: &PciDevice) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    if d.status() & cfg::STATUS_CAP_LIST == 0 {
        return v;
    }
    let mut off = d.read_config8(cfg::CAPABILITY_LIST).unwrap_or(0) as u32 & 0xFC;
    while off != 0 && v.len() < 48 {
        let id = d.read_config8(off).unwrap_or(0) as u32;
        v.push((id, off));
        off = d.read_config8(off + 1).unwrap_or(0) as u32 & 0xFC;
    }
    v
}

fn run_pci(t: &mut Tally) {
    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IPCI }.is_null();
    let pci = match Pci::get() {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("Pci::get: {}", e)),
    };
    let all = match pci.devices(&Query::any()) {
        Ok(v) => v,
        Err(e) => return t.check(false, format_args!("pci: devices: {}", e)),
    };
    t.check(null && all.len() >= 3, format_args!("pci: the crate's own interface lists {} devices", all.len()));
    for d in &all {
        let (b, dv, f) = d.address();
        let bars: Vec<String> = d
            .bars()
            .iter()
            .map(|b| alloc::format!("{}:{}@{:#x}/{:#x}+{:#x}", b.index, if b.kind == BarKind::Io { "io" } else { "mem" }, b.base, b.physical, b.size))
            .collect();
        let caps: Vec<String> = d.capabilities().iter().map(|c| alloc::format!("{:#x}@{:#x}", c.id, c.offset)).collect();
        say!(
            "info: {:02x}:{:02x}.{} {:04x}:{:04x} class {:06x} rev {} hdr {:#x} flags {:#x} irq {:?} (pin {} line {}) bars [{}] caps [{}]",
            b, dv, f, d.vendor_id(), d.device_id(), d.class_code(), d.revision(), d.header_type(), d.flags(), d.interrupt(),
            d.interrupt_pin(), d.interrupt_line(), bars.join(" "), caps.join(" ")
        );
    }

    // Each device found again by its IDs and by its address.
    let mut by_id = 0;
    let mut by_addr = 0;
    for d in &all {
        let a = d.address();
        let same = pci
            .devices(&Query::any().vendor(d.vendor_id()).device(d.device_id()))
            .map(|v| v.iter().any(|x| x.address() == a))
            .unwrap_or(false);
        by_id += same as u32;
        let at = pci.find(&Query::at(a.0, a.1, a.2), 0).ok().flatten();
        by_addr += at.map(|x| x.vendor_id() == d.vendor_id() && x.device_id() == d.device_id()).unwrap_or(false) as u32;
    }
    t.check(
        by_id == all.len() as u32 && by_addr == all.len() as u32,
        format_args!("pci: every device found again by vendor:device ({}) and by bus:dev.fn ({})", by_id, by_addr),
    );

    // A class query returns only that class.
    if let Some(first) = all.first() {
        let base = first.class_code() & 0x00FF_0000;
        let v = pci.devices(&Query::any().class(base, 0x00FF_0000)).unwrap_or_default();
        let want = all.iter().filter(|d| d.class_code() & 0x00FF_0000 == base).count();
        t.check(
            v.len() == want && v.iter().all(|d| d.class_code() & 0x00FF_0000 == base),
            format_args!("pci: base class {:02x}: {} devices, as many as in the full list ({})", base >> 16, v.len(), want),
        );
    }
    let none = (pci.devices(&Query::any().vendor(0xFFFE)).map(|v| v.len()), pci.find(&Query::at(255, 31, 7), 0).map(|d| d.is_none()));
    t.check(none == (Ok(0), Ok(true)), format_args!("pci: no device for vendor FFFE or at ff:1f.7"));
    t.check(pci.find(&Query::at(0, 1, 0).vendor(1), 0).is_err(), format_args!("pci: an address query with another condition is refused"));

    if let Some(d) = all.first() {
        let ok = d.read_config32(0) == Ok(((d.device_id() as u32) << 16) | d.vendor_id() as u32);
        let refused = d.read_config8(256).is_err() && d.read_config16(3).is_err() && d.read_config32(0xFE).is_err();
        t.check(ok && refused, format_args!("pci: ReadConfigLong(0) is device<<16|vendor (native order); offset 256 and unaligned reads refused"));
    }

    // The library's capability list against a walk by hand.
    let mut with_caps = 0;
    let mut agree = 0;
    for d in &all {
        let man = manual_caps(d);
        if man.is_empty() {
            continue;
        }
        with_caps += 1;
        let lib: Vec<(u32, u32)> = d.capabilities().iter().map(|c| (c.id, c.offset)).collect();
        let first = d.find_capability(man[0].0).map(|c| (c.id, c.offset));
        if lib == man && first == Some(man[0]) {
            agree += 1;
        } else {
            say!("info: caps differ on {:04x}:{:04x}: library {:x?} by hand {:x?}", d.vendor_id(), d.device_id(), lib, man);
        }
    }
    t.check(with_caps > 0 && agree == with_caps, format_args!("pci: capability lists ({} devices) match a walk from 0x34; FindCapability finds the first", with_caps));

    // BAR kinds against the BAR registers' low bits (header type 0).
    let mut bars = 0;
    let mut bad = 0;
    for d in all.iter().filter(|d| d.header_type() & 0x7F == 0) {
        for b in d.bars() {
            bars += 1;
            let reg = d.read_config32(0x10 + 4 * b.index as u32).unwrap_or(0);
            let io = reg & 1 == 1;
            if io != (b.kind == BarKind::Io) {
                bad += 1;
                say!("info: BAR {} of {:04x}:{:04x}: register {:#x}, library {:?}", b.index, d.vendor_id(), d.device_id(), reg, b.kind);
            }
        }
    }
    t.check(bars > 0 && bad == 0, format_args!("pci: {} BARs, I/O or memory as their registers say", bars));

    let m0 = avail();
    for _ in 0..200 {
        for d in pci.devices(&Query::any()).unwrap_or_default() {
            let _ = d.bars();
            let _ = d.capabilities();
        }
    }
    say!("info: pci: free memory around 200 rounds of find all, BARs, capabilities: {:+} bytes", avail() as i64 - m0 as i64);

    // Locks: a device a driver holds is refused; one nobody holds can be
    // locked once exclusively, or shared by two.
    let mut refused = Vec::new();
    let mut free = None;
    let mut free_is_plain = true;
    for d in all.iter() {
        let a = d.address();
        let Ok(Some(mut x)) = pci.find(&Query::at(a.0, a.1, a.2), 0) else { continue };
        let (v, id, hdr) = (x.vendor_id(), x.device_id(), x.header_type());
        let got = x.lock(LockMode::Exclusive).map(|_| ());
        match got {
            Err(AmigaError::InUse) => refused.push(alloc::format!("{:04x}:{:04x}", v, id)),
            Ok(_) => {
                // Prefer a virtio device with an I/O BAR: its legacy
                // register 0 (host features) is read-only, so the I/O
                // window test reads it without side effects.
                let virtio_io = v == 0x1af4 && x.bars().iter().any(|b| b.kind == BarKind::Io);
                if hdr & 0x7F == 0 && v != 0x1106 && (free.is_none() || virtio_io && free_is_plain) {
                    free = Some(a);
                    free_is_plain = !virtio_io;
                }
            }
            Err(e) => say!("info: lock: {}", e),
        }
    }
    say!("info: devices a driver holds (exclusive lock refused): {:?}", refused);
    t.check(!refused.is_empty(), format_args!("pci: Lock refuses devices that drivers hold ({})", refused.len()));
    let Some(a) = free else { return t.check(false, format_args!("pci: no free device to test locks on")) };
    let find = || pci.find(&Query::at(a.0, a.1, a.2), 0).ok().flatten();
    let (Some(mut x), Some(mut y)) = (find(), find()) else { return t.check(false, format_args!("pci: find the free device twice")) };
    say!("info: lock tests on {:02x}:{:02x}.{} {:04x}:{:04x}", a.0, a.1, a.2, x.vendor_id(), x.device_id());
    {
        let l1 = x.lock(LockMode::Exclusive);
        let l2 = y.lock(LockMode::Exclusive).map(|_| ());
        let l3 = y.lock(LockMode::Shared).map(|_| ());
        let other = Builder::new(b"libtest locker\0")
            .priority(task::priority())
            .stack(16384)
            .spawn(move || {
                let p = Pci::get().ok()?;
                let mut d = p.find(&Query::at(a.0, a.1, a.2), 0).ok()??;
                let r = d.lock(LockMode::Exclusive).map(|_| ()).err();
                Some(r)
            })
            .ok()
            .and_then(|h| h.join().ok());
        say!("info: pci: held exclusively by this task: a second exclusive lock {:?}, a shared one {:?} (same task)", l2, l3);
        t.check(
            l1.is_ok() && other == Some(Some(Some(AmigaError::InUse))),
            format_args!("pci: held exclusively, another task's lock is refused ({:?})", other),
        );
    }
    {
        let s1 = x.lock(LockMode::Shared);
        let s2 = y.lock(LockMode::Shared).map(|_| ());
        let ok = s1.is_ok() && s2.is_ok();
        drop(s1);
        let e = y.lock(LockMode::Exclusive).map(|_| ());
        t.check(ok && e.is_ok(), format_args!("pci: two shared locks together; exclusive again once both are gone"));
    }

    // An I/O BAR of the free device: bounds checks, reads.
    let lk = x.lock(LockMode::Exclusive);
    match lk {
        Ok(l) => {
            let io = l.device().bars().into_iter().find(|b| b.kind == BarKind::Io);
            match io.map(|b| (b.index, l.io(b.index))) {
                Some((i, Ok(r))) => {
                    let inside = r.read8(0).is_ok() && r.read8(r.size() - 1).is_ok();
                    let outside = r.read8(r.size()).is_err() && r.read32(r.size() - 2).is_err() && r.read16(1).is_err();
                    let f = r.read32(0);
                    say!("info: I/O BAR {} at port {:#x}, {} bytes: first long {:x?}", i, r.port(), r.size(), f);
                    t.check(inside && outside, format_args!("pci: an I/O window reads inside its BAR and refuses outside or unaligned"));
                    if l.device().vendor_id() == 0x1af4 {
                        // Legacy virtio: VIRTIO_F_NOTIFY_ON_EMPTY (24) or
                        // the device's own bits are set; all-ones or zero
                        // would be a byte-order or routing fault.
                        let ok = matches!(f, Ok(v) if v != 0 && v != u32::MAX);
                        t.check(ok, format_args!("pci: a virtio device's host features through the I/O window: {:x?}", f));
                    }
                }
                _ => say!("info: the free device has no I/O BAR"),
            }
            let mem = l.device().bars().into_iter().find(|b| b.kind != BarKind::Io);
            if let Some(b) = mem {
                t.check(l.io(b.index).is_err(), format_args!("pci: a memory BAR is not an I/O window"));
            }
            let c = l.device().command();
            let c2 = l.set_decoding(c & cfg::COMMAND_IO != 0, c & cfg::COMMAND_MEMORY != 0);
            t.check(c2 == c, format_args!("pci: set_decoding with the current bits leaves COMMAND {:#x} as it was ({:#x})", c, c2));
        }
        Err(e) => t.check(false, format_args!("pci: lock the free device again: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// dos (D1): files, locks, examine, directories, names, vars, patterns
// ---------------------------------------------------------------------------

const TD: &str = "RAM:libtest.d";

/// Remove what an earlier run may have left.
fn dos_clean() {
    if let Ok(rd) = dos::fs::read_dir(TD) {
        let names: Vec<(String, bool)> = rd.filter_map(|e| e.ok()).map(|m| { let d = m.is_dir(); (m.name, d) }).collect();
        for (n, d) in names {
            let p = alloc::format!("{}/{}", TD, n);
            if d {
                if let Ok(rd2) = dos::fs::read_dir(&p) {
                    let inner: Vec<String> = rd2.filter_map(|e| e.ok()).map(|m| m.name).collect();
                    for i in inner {
                        let _ = dos::fs::delete(&alloc::format!("{}/{}", p, i));
                    }
                }
            }
            let _ = dos::fs::delete(&p);
        }
    }
    let _ = dos::fs::delete(TD);
}

fn pattern_bytes(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i.wrapping_mul(31) ^ (i >> 7)) as u8).collect()
}

fn run_dos(t: &mut Tally) {
    t.check(dos::is_process(), format_args!("dos: libtest runs as a DOS Process"));
    let in_task = Builder::new(b"libtest dos task\0").priority(task::priority()).stack(16384).spawn(|| {
        (
            dos::is_process(),
            dos::fs::metadata("RAM:").err(),
            dos::fs::File::open("RAM:x", dos::fs::OpenMode::Read).err(),
            dos::DateStamp::now().is_ok(),
            dos::add_part("RAM:a", "b").ok(),
        )
    });
    let r = in_task.ok().and_then(|h| h.join().ok());
    t.check(
        matches!(&r, Some((false, Some(AmigaError::WrongTask), Some(AmigaError::WrongTask), true, Some(s))) if s == "RAM:a/b"),
        format_args!("dos: in an exec task, DOS I/O is WrongTask; DateStamp and AddPart work ({:?})", r.as_ref().map(|x| (x.0, x.1, x.3))),
    );

    let f205 = dos::fault(205);
    say!("info: Fault(205) = \"{}\"", f205);
    t.check(f205.to_ascii_lowercase().contains("not found"), format_args!("dos: Fault gives DOS's text for an error"));

    dos_clean();
    t.check(dos::fs::create_dir(TD).is_ok() && dos::fs::exists(TD) == Ok(true), format_args!("dos: CreateDir {}", TD));
    let again = dos::fs::create_dir(TD);
    t.check(again == Err(AmigaError::DosError(sys::ERROR_OBJECT_EXISTS)), format_args!("dos: CreateDir again is ERROR_OBJECT_EXISTS ({:?})", again));

    // A file written, read back, sought, sized.
    let data = pattern_bytes(100_000);
    let fname = alloc::format!("{}/data.bin", TD);
    let w = dos::fs::File::write_new(&fname, &data);
    let back = dos::fs::File::read_to_vec(&fname);
    t.check(w.is_ok() && back.as_deref() == Ok(&data[..]), format_args!("dos: 100000 bytes written and read back exactly ({:?})", back.as_ref().map(|v| v.len())));
    match dos::fs::File::open(&fname, dos::fs::OpenMode::Update) {
        Ok(mut f) => {
            let len = f.len();
            let p = f.seek(dos::fs::SeekFrom::Start(50_000));
            let mut b = [0u8; 16];
            let n = f.read(&mut b);
            let pos = f.position();
            let end = f.seek(dos::fs::SeekFrom::End(-10));
            let cur = f.seek(dos::fs::SeekFrom::Current(-90));
            t.check(
                len == Ok(100_000) && p == Ok(50_000) && n == Ok(16) && b[..] == data[50_000..50_016] && pos == Ok(50_016) && end == Ok(99_990) && cur == Ok(99_900),
                format_args!("dos: GetFileSize, ChangeFilePosition from start, current and end, GetFilePosition"),
            );
            let tr = f.set_len(1234);
            let l2 = f.len();
            let _ = f.seek(dos::fs::SeekFrom::End(0));
            let app = f.write_all(b"tail");
            let l3 = f.len();
            t.check(tr.is_ok() && l2 == Ok(1234) && app.is_ok() && l3 == Ok(1238), format_args!("dos: ChangeFileSize to 1234, 4 bytes appended: {:?}", l3));
            let m = f.metadata();
            let path = f.path();
            t.check(
                m.as_ref().map(|m| m.is_file() && m.size == Some(1238) && m.name == "data.bin").unwrap_or(false)
                    && path.as_deref().map(|p| p.ends_with("libtest.d/data.bin")).unwrap_or(false),
                format_args!("dos: ExamineObject of the open file, NameFromFH \"{}\"", path.as_deref().unwrap_or("?")),
            );
            let lk = f.lock();
            let c = f.close();
            t.check(c.is_ok() && lk.as_ref().map(|l| l.metadata().map(|m| m.size) == Ok(Some(1238))).unwrap_or(false), format_args!("dos: DupLockFromFH, Close"));
        }
        Err(e) => t.check(false, format_args!("dos: open {} for update: {}", fname, dos::describe(&e))),
    }
    let missing = dos::fs::File::open("RAM:libtest.d/none", dos::fs::OpenMode::Read).err();
    t.check(missing == Some(AmigaError::DosError(sys::ERROR_OBJECT_NOT_FOUND)), format_args!("dos: opening a missing file is ERROR_OBJECT_NOT_FOUND ({:?})", missing.map(|e| dos::describe(&e))));
    t.check(dos::fs::File::open("RAM:a\0b", dos::fs::OpenMode::Read).err() == Some(AmigaError::InvalidArgument("string contains a NUL")), format_args!("dos: a name with a NUL inside is refused"));

    // Locks.
    let l1 = dos::fs::Lock::new(TD, dos::fs::LockMode::Shared);
    let l2 = dos::fs::Lock::new("RAM:libtest.d/", dos::fs::LockMode::Shared);
    match (&l1, &l2) {
        (Ok(a), Ok(b)) => {
            let same = a.same(b);
            let dup = a.try_clone().and_then(|c| c.same(a));
            let p = a.path();
            let mut depth = 0;
            let mut cur = a.try_clone().ok();
            while let Some(l) = cur {
                cur = l.parent().ok().flatten();
                depth += 1;
                if depth > 10 {
                    break;
                }
            }
            t.check(
                same == Ok(dos::fs::Sameness::Same) && dup == Ok(dos::fs::Sameness::Same) && depth == 2 && p.as_deref().map(|s| s.ends_with(":libtest.d")).unwrap_or(false),
                format_args!("dos: SameLock, DupLock, ParentDir up to the root ({} levels), NameFromLock \"{}\"", depth, p.as_deref().unwrap_or("?")),
            );
            let ram = dos::fs::Lock::new("RAM:", dos::fs::LockMode::Shared);
            let dir_ex = dos::fs::Lock::new(TD, dos::fs::LockMode::Exclusive).map(|_| ()).err();
            say!("info: dos: an exclusive lock on a directory others hold shared: {:?} (V53+ makes directory locks shared)", dir_ex);
            let fl = dos::fs::Lock::new(&fname, dos::fs::LockMode::Shared);
            let ex = dos::fs::Lock::new(&fname, dos::fs::LockMode::Exclusive).map(|_| ()).err();
            drop(fl);
            let ex2 = dos::fs::Lock::new(&fname, dos::fs::LockMode::Exclusive).map(|_| ()).err();
            t.check(
                ram.and_then(|r| r.same(a)) == Ok(dos::fs::Sameness::SameVolume) && ex == Some(AmigaError::DosError(sys::ERROR_OBJECT_IN_USE)) && ex2.is_none(),
                format_args!("dos: RAM: and the directory are on the same volume; an exclusive lock on a file held shared is ERROR_OBJECT_IN_USE ({:?}), granted once it is free", ex),
            );
        }
        _ => t.check(false, format_args!("dos: Lock {}: {:?} {:?}", TD, l1.as_ref().err(), l2.as_ref().err())),
    }
    drop((l1, l2));

    // A directory of 20 files and a subdirectory, listed.
    let mut made = 0;
    for k in 0..20 {
        let n = alloc::format!("{}/f{:02}.{}", TD, k, if k % 2 == 0 { "txt" } else { "dat" });
        if dos::fs::File::write_new(&n, &pattern_bytes(k * 10)).is_ok() {
            made += 1;
        }
    }
    let sub = alloc::format!("{}/sub", TD);
    let _ = dos::fs::create_dir(&sub);
    let entries: Vec<dos::fs::Metadata> = dos::fs::read_dir(TD).map(|r| r.filter_map(|e| e.ok()).collect()).unwrap_or_default();
    let files = entries.iter().filter(|m| m.is_file() && m.name.starts_with('f')).count();
    let sizes_ok = entries.iter().filter(|m| m.name.starts_with('f')).all(|m| m.size == Some(m.name[1..3].parse::<u64>().unwrap_or(99) * 10));
    let dirs = entries.iter().filter(|m| m.is_dir()).count();
    t.check(made == 20 && files == 20 && sizes_ok && dirs == 1 && entries.len() == 22, format_args!("dos: ExamineDir lists 20 files with their sizes, 1 directory, data.bin ({} entries)", entries.len()));
    let txt = dos::Pattern::new("f#?.TXT", false).and_then(|p| dos::fs::read_dir_matching(TD, &p).map(|r| r.filter_map(|e| e.ok()).count()));
    t.check(txt == Ok(10), format_args!("dos: ExamineDir with a pattern (f#?.TXT, no case): {:?} entries", txt));
    let lk = dos::fs::Lock::new(TD, dos::fs::LockMode::Shared).and_then(|l| l.read_dir().map(|r| r.count()));
    t.check(lk == Ok(22), format_args!("dos: listing through a lock: {:?}", lk));

    // Names: protection, comment, date, rename, delete.
    let f0 = alloc::format!("{}/f00.txt", TD);
    let when = dos::DateStamp { ds_Days: 10000, ds_Minute: 600, ds_Tick: 0 };
    let p = dos::fs::set_protection(&f0, 0x0000_0005);
    let c = dos::fs::set_comment(&f0, "a comment");
    let d = dos::fs::set_date(&f0, &when);
    let m = dos::fs::metadata(&f0);
    t.check(
        p.is_ok() && c.is_ok() && d.is_ok() && m.as_ref().map(|m| m.protection & 0xFF == 5 && m.comment == "a comment" && m.date == when).unwrap_or(false),
        format_args!("dos: SetProtection, SetComment, SetDate read back by ExamineObject ({:?})", m.as_ref().map(|m| (m.protection, m.date))),
    );
    let _ = dos::fs::set_protection(&f0, 0);
    let rn = dos::fs::rename(&f0, &alloc::format!("{}/renamed", TD));
    t.check(rn.is_ok() && dos::fs::exists(&f0) == Ok(false) && dos::fs::exists(&alloc::format!("{}/renamed", TD)) == Ok(true), format_args!("dos: Rename"));
    let _ = dos::fs::File::write_new(&alloc::format!("{}/in", sub), b"x");
    let ne = dos::fs::delete(&sub);
    t.check(ne == Err(AmigaError::DosError(sys::ERROR_DIRECTORY_NOT_EMPTY)), format_args!("dos: deleting a directory with something in it is ERROR_DIRECTORY_NOT_EMPTY ({:?})", ne));

    // Memory around 100 rounds of make, write, examine, list, delete.
    let m0 = avail();
    for k in 0..100 {
        let n = alloc::format!("{}/cycle{}", TD, k % 7);
        let _ = dos::fs::File::write_new(&n, &pattern_bytes(512));
        let _ = dos::fs::metadata(&n);
        let _ = dos::fs::read_dir(TD).map(|r| r.count());
        let _ = dos::fs::File::read_to_vec(&n);
        let _ = dos::fs::delete(&n);
    }
    say!("info: dos: free memory around 100 file cycles: {:+} bytes", avail() as i64 - m0 as i64);
    dos_clean();
    t.check(dos::fs::exists(TD) == Ok(false), format_args!("dos: everything deleted again"));

    // Variables.
    let lv = dos::set_var("libtest_local", b"one two", dos::VarScope::Local);
    let g1 = dos::get_var("libtest_local", dos::VarScope::Local);
    let g2 = dos::get_var("libtest_local", dos::VarScope::Any);
    let g3 = dos::get_var("libtest_local", dos::VarScope::Global);
    let dl = dos::delete_var("libtest_local", dos::VarScope::Local);
    let g4 = dos::get_var("libtest_local", dos::VarScope::Local);
    t.check(
        lv.is_ok() && g1.as_ref().ok() == Some(&Some(String::from("one two"))) && g2 == g1 && g3 == Ok(None) && dl.is_ok() && g4 == Ok(None),
        format_args!("dos: a local variable set, read (local and any, not global), deleted ({:?} {:?} {:?} {:?} {:?} {:?})", lv, g1, g2, g3, dl, g4),
    );
    let big: Vec<u8> = (0..3000).map(|i| b"ab\ncd"[i % 5]).collect();
    let gv = dos::set_var("libtest_global", &big, dos::VarScope::Global);
    let gg = dos::get_var_bytes("libtest_global", dos::VarScope::Global);
    let gd = dos::delete_var("libtest_global", dos::VarScope::Global);
    t.check(
        gv.is_ok() && gg.as_ref().ok().and_then(|o| o.as_ref()).map(|v| v == &big).unwrap_or(false) && gd.is_ok() && dos::get_var("libtest_global", dos::VarScope::Global) == Ok(None),
        format_args!("dos: a 3000-byte global variable with newlines in ENV: read back whole, deleted ({:?})", gg.as_ref().map(|o| o.as_ref().map(|v| v.len()))),
    );

    // Patterns and names.
    let pr = (|| -> Result<(bool, bool, bool, bool, bool, bool), AmigaError> {
        let p = dos::Pattern::new("#?.info", false)?;
        let alt = dos::Pattern::new("(foo|bar)x", true)?;
        let plain = dos::Pattern::new("abc", true)?;
        Ok((p.matches("Disk.INFO")?, p.matches("a.infox")?, alt.matches("barx")?, alt.matches("BARX")?, p.is_wild(), plain.is_wild()))
    })();
    t.check(pr == Ok((true, false, true, false, true, false)), format_args!("dos: patterns #?.info (no case), (foo|bar)x (case), a plain one not wild ({:?})", pr));
    let parts = (
        dos::add_part("RAM:a", "b"),
        dos::add_part("RAM:a/b", "/c"),
        dos::add_part("RAM:a", "DH0:x"),
        dos::file_part("DH0:a/b/c"),
        dos::path_part("DH0:a/b/c"),
        dos::file_part("DH0:"),
    );
    t.check(
        parts.0.as_deref() == Ok("RAM:a/b") && parts.1.as_deref() == Ok("RAM:a/b//c") && parts.2.as_deref() == Ok("DH0:x") && parts.3 == Ok("c") && parts.4 == Ok("DH0:a/b") && parts.5 == Ok(""),
        format_args!("dos: AddPart (joins, \"/c\" kept for DOS to resolve), FilePart, PathPart ({:?})", parts),
    );

    // Dates.
    let ds = dos::DateStamp::from_seconds(86400 * 365 + 3661);
    let secs = ds.and_then(|d| d.to_seconds());
    let now = dos::DateStamp::now();
    t.check(
        ds.map(|d| (d.ds_Days, d.ds_Minute, d.ds_Tick)) == Ok((365, 61, 50)) && secs == Ok(86400 * 365 + 3661) && now.map(|n| n > ds.unwrap_or_default() && n.ds_Days > 16000).unwrap_or(false),
        format_args!("dos: SecondsToDateStamp/DateStampToSeconds, DateStamp now {:?} (unix {:?})", now, now.map(|n| n.to_unix())),
    );
    let out = dos::stdout();
    t.check(out.is_ok() && dos::stderr().is_ok(), format_args!("dos: Output and ErrorOutput (interactive: {:?})", out.map(|o| o.is_interactive())));
}

// ---------------------------------------------------------------------------
// dos (D2): child processes
// ---------------------------------------------------------------------------

const PD: &str = "RAM:libtest.p";

fn run_process(t: &mut Tally) {
    use dos::process::{self as proc, ProcessBuilder};
    let me = proc::pid();
    say!("info: dos {:?}, this process's PID {}, parent {}", dos::version(), me, proc::parent_pid());

    // One child: its result, that it is a Process, its PID and parent.
    let c = proc::spawn("libtest child", || (dos::is_process(), proc::pid(), proc::parent_pid(), 6 * 7));
    match c {
        Ok(c) => {
            let pid = c.pid();
            let r = c.wait();
            t.check(
                matches!(r, Ok((true, p, pp, 42)) if p == pid && pp == me && p != me),
                format_args!("process: a child Process returns 42, knows its PID {} and its parent {} ({:?})", pid, me, r),
            );
        }
        Err(e) => t.check(false, format_args!("process: spawn: {}", dos::describe(&e))),
    }

    // The CreateNewProc race: children above the parent that end at once.
    let m0 = avail();
    let mut good = 0;
    for k in 0..200u32 {
        let c = ProcessBuilder::new("libtest quick").and_then(|b| b.priority(task::priority().saturating_add(5)).spawn(move || k * 3));
        if c.and_then(|c| c.wait()) == Ok(k * 3) {
            good += 1;
        }
    }
    t.check(good == 200, format_args!("process: 200 children 5 above the parent that end at once: {}/200 results", good));
    say!("info: process: free memory around 200 children: {:+} bytes", avail() as i64 - m0 as i64);

    // Children using DOS: 8 at once, each writes its own file.
    let _ = dos::fs::create_dir(PD);
    let mut kids = Vec::new();
    for k in 0..8u32 {
        let c = proc::spawn("libtest writer", move || {
            let name = alloc::format!("{}/w{}", PD, k);
            let data: Vec<u8> = (0..4096u32).map(|i| (i * (k + 1)) as u8).collect();
            dos::fs::File::write_new(&name, &data).is_ok() && dos::fs::metadata(&name).map(|m| m.size) == Ok(Some(4096))
        });
        match c {
            Ok(c) => kids.push(c),
            Err(e) => say!("info: spawn writer {}: {}", k, dos::describe(&e)),
        }
    }
    let ok = kids.into_iter().map(|c| c.wait() == Ok(true)).filter(|&b| b).count();
    let files = (0..8u32)
        .filter(|k| {
            let want: Vec<u8> = (0..4096u32).map(|i| (i * (k + 1)) as u8).collect();
            dos::fs::File::read_to_vec(&alloc::format!("{}/w{}", PD, k)).map(|v| v == want).unwrap_or(false)
        })
        .count();
    t.check(ok == 8 && files == 8, format_args!("process: 8 children at once write and examine files through DOS: {} reported, {} right", ok, files));

    // Output and current directory given to the child.
    let out = dos::fs::File::open(&alloc::format!("{}/out.txt", PD), dos::fs::OpenMode::Create);
    let dir = dos::fs::Lock::new(PD, dos::fs::LockMode::Shared);
    match (out, dir) {
        (Ok(o), Ok(d)) => {
            let c = ProcessBuilder::new("libtest out").map(|b| b.output(o).current_dir(d)).and_then(|b| {
                b.spawn(|| {
                    use core::fmt::Write as _;
                    let w = dos::stdout().map(|mut s| write!(s, "hello from pid {}\n", proc::pid()).is_ok()).unwrap_or(false);
                    let rel = dos::fs::File::write_new("relative.txt", b"in the given directory").is_ok();
                    w && rel
                })
            });
            let r = c.and_then(|c| c.wait());
            let text = dos::fs::File::read_to_vec(&alloc::format!("{}/out.txt", PD)).map(|v| String::from_utf8_lossy(&v).into_owned());
            let rel = dos::fs::exists(&alloc::format!("{}/relative.txt", PD));
            t.check(
                r == Ok(true) && text.as_deref().map(|s| s.starts_with("hello from pid ")).unwrap_or(false) && rel == Ok(true),
                format_args!("process: the child's Output() is the given file ({:?}); a relative name lands in the given directory", text.as_deref().map(|s| s.trim_end())),
            );
        }
        (o, d) => t.check(false, format_args!("process: open the output and lock the directory: {:?} {:?}", o.err(), d.err())),
    }

    // is_running, then a drop that waits.
    let flag = alloc::sync::Arc::new(core::sync::atomic::AtomicU32::new(0));
    let f2 = flag.clone();
    match ProcessBuilder::new("libtest slow").and_then(|b| {
        b.priority(task::priority().saturating_sub(1)).spawn(move || {
            busy_ms(200);
            f2.store(7, core::sync::atomic::Ordering::Release);
        })
    }) {
        Ok(c) => {
            let running = c.is_running();
            drop(c);
            let after = flag.load(core::sync::atomic::Ordering::Acquire);
            t.check(running && after == 7, format_args!("process: is_running while it works; dropping the Child waited for it ({})", after));
        }
        Err(e) => t.check(false, format_args!("process: spawn slow: {}", dos::describe(&e))),
    }

    // Clean up.
    if let Ok(rd) = dos::fs::read_dir(PD) {
        let names: Vec<String> = rd.filter_map(|e| e.ok()).map(|m| m.name).collect();
        for n in names {
            let _ = dos::fs::delete(&alloc::format!("{}/{}", PD, n));
        }
    }
    let _ = dos::fs::delete(PD);
    t.check(dos::fs::exists(PD) == Ok(false), format_args!("process: test files removed"));
}

// ---------------------------------------------------------------------------
// dos (D2): ReadArgs
// ---------------------------------------------------------------------------

fn run_args(t: &mut Tally, what: &str) {
    use dos::args::Args;
    const TPL: &str = "FROM/A/M,TO/A,ALL/S,BUF=BUFFERS/K/N,QUIET/T,NAME/K";
    match Args::parse(TPL, "a b c dest ALL BUF=12 QUIET=on NAME \"two words\"") {
        Ok(a) => {
            let r = (a.multi("from"), a.text("TO"), a.switch("all"), a.number("BUF"), a.number("buffers"), a.toggle("QUIET"), a.text("NAME"));
            t.check(
                r == (Ok(alloc::vec!["a", "b", "c"]), Ok(Some("dest")), Ok(true), Ok(Some(12)), Ok(Some(12)), Ok(Some(true)), Ok(Some("two words"))),
                format_args!("args: /M /A /S /K/N (by either name) /T and a quoted /K string ({:?})", r),
            );
            t.check(a.text("ALL").is_err() && a.number("TO").is_err() && a.switch("NOPE").is_err(), format_args!("args: reading an option at another type, or one not in the template, is refused"));
        }
        Err(e) => t.check(false, format_args!("args: parse: {}", dos::describe(&e))),
    }
    match Args::parse(TPL, "x y QUIET=off") {
        Ok(a) => {
            let r = (a.multi("FROM"), a.text("TO"), a.switch("ALL"), a.number("BUF"), a.toggle("QUIET"), a.text("NAME"));
            t.check(
                r == (Ok(alloc::vec!["x"]), Ok(Some("y")), Ok(false), Ok(None), Ok(Some(false)), Ok(None)),
                format_args!("args: /A takes the last of /M; absent options are false or None; QUIET=off ({:?})", r),
            );
        }
        Err(e) => t.check(false, format_args!("args: parse 2: {}", dos::describe(&e))),
    }
    let q = Args::parse(TPL, "x y").map(|a| a.toggle("QUIET"));
    t.check(q == Ok(Ok(None)), format_args!("args: an absent /T is None ({:?})", q));
    let errs = (
        Args::parse(TPL, "only").err(),
        Args::parse(TPL, "x y BUF=zz").err(),
        Args::parse("A", "1 2").err(),
        Args::parse(TPL, "x y BUF").err(),
    );
    t.check(
        errs == (
            Some(AmigaError::DosError(sys::ERROR_REQUIRED_ARG_MISSING)),
            Some(AmigaError::DosError(sys::ERROR_BAD_NUMBER)),
            Some(AmigaError::DosError(sys::ERROR_TOO_MANY_ARGS)),
            Some(AmigaError::DosError(sys::ERROR_KEY_NEEDS_ARG)),
        ),
        format_args!("args: missing /A, bad number, too many, keyword without value give their DOS errors ({:?})", errs),
    );
    t.check(Args::parse("A/X", "1").is_err() && Args::parse("A/M,B/M", "1").is_err(), format_args!("args: bad templates refused before DOS sees them"));
    let m0 = avail();
    for k in 0..500 {
        let _ = Args::parse(TPL, &alloc::format!("f{} g{} to{} BUF={}", k, k, k, k)).map(|a| a.number("BUF"));
    }
    say!("info: args: free memory around 500 parses: {:+} bytes", avail() as i64 - m0 as i64);
    let own = Args::from_command_line("WHAT,OUT/K").map(|a| a.text("WHAT").map(|o| o.map(String::from)));
    say!("info: args: ReadArgs on libtest's own command line: {:?}", own);
    t.check(
        matches!(&own, Ok(Ok(Some(w))) if w == what),
        format_args!("args: ReadArgs on the program's own command line finds \"{}\"", what),
    );
}

// ---------------------------------------------------------------------------
// dos (D3): volumes, disk info, notification, System, guards, date text
// ---------------------------------------------------------------------------

const ND: &str = "RAM:libtest.n";

/// Poll `f` for up to `ms` milliseconds.
fn within(ms: u64, f: impl Fn() -> bool) -> bool {
    let t0 = amigaos4::arch::timebase();
    loop {
        if f() {
            return true;
        }
        if ms_since(t0) >= ms {
            return false;
        }
        busy_ms(5);
    }
}

fn run_dos3(t: &mut Tally) {
    use dos::misc::{system, CurrentDir, DateFormat, NoRequesters};
    use dos::volume::{disk_info, names, DiskState, Kinds};

    let vols = names(Kinds { volumes: true, devices: false, assigns: false });
    let devs = names(Kinds { volumes: false, devices: true, assigns: false });
    let asg = names(Kinds { volumes: false, devices: false, assigns: true });
    say!("info: volumes {:?}", vols);
    say!("info: assigns (first 8) {:?}", asg.as_ref().map(|v| v.iter().take(8).collect::<Vec<_>>()));
    t.check(
        vols.as_ref().map(|v| !v.is_empty() && v.iter().all(|n| n.ends_with(':'))).unwrap_or(false)
            && devs.as_ref().map(|v| v.iter().any(|n| n == "RAM:")).unwrap_or(false)
            && asg.as_ref().map(|v| v.iter().any(|n| n.eq_ignore_ascii_case("LIBS:"))).unwrap_or(false),
        format_args!("volume: volumes, devices (RAM:) and assigns (LIBS:) listed with their colons"),
    );
    let sys_info = disk_info("SYS:");
    say!("info: SYS: {:?} ({} MB free)", sys_info, sys_info.map(|d| d.free_bytes() >> 20).unwrap_or(0));
    t.check(
        sys_info.map(|d| d.state == DiskState::Validated && d.blocks > 0 && d.blocks_used <= d.blocks && d.bytes_per_block >= 512).unwrap_or(false),
        format_args!("volume: GetDiskInfo of SYS: is a validated disk with sensible blocks"),
    );
    t.check(disk_info("NOSUCHDEV:").is_err(), format_args!("volume: GetDiskInfo of a missing device fails"));

    // Notification.
    let _ = dos::fs::create_dir(ND);
    match dos::notify::Watcher::new(ND) {
        Ok(w) => {
            let quiet = !w.changed();
            let _ = dos::fs::File::write_new(&alloc::format!("{}/a", ND), b"x");
            let saw = within(1000, || w.changed());
            let again = within(200, || w.changed());
            let _ = dos::fs::delete(&alloc::format!("{}/a", ND));
            let del = within(1000, || w.changed());
            t.check(quiet && saw && !again && del, format_args!("notify: a directory watcher signals once for a new file, again for its deletion ({} {} {} {})", quiet, saw, again, del));
        }
        Err(e) => t.check(false, format_args!("notify: Watcher::new: {}", dos::describe(&e))),
    }
    let file = alloc::format!("{}/watched", ND);
    let _ = dos::fs::File::write_new(&file, b"1");
    match dos::notify::Watcher::new(&file) {
        Ok(w) => {
            let _ = dos::fs::File::write_new(&file, b"22");
            let saw = within(1000, || w.changed());
            t.check(saw, format_args!("notify: a file watcher signals when the file is rewritten"));
        }
        Err(e) => t.check(false, format_args!("notify: watch a file: {}", dos::describe(&e))),
    }
    let m0 = avail();
    for _ in 0..200 {
        let _ = dos::notify::Watcher::new(ND);
    }
    say!("info: notify: free memory around 200 watchers: {:+} bytes", avail() as i64 - m0 as i64);

    // System.
    let out = alloc::format!("{}/sys.txt", ND);
    match dos::fs::File::open(&out, dos::fs::OpenMode::Create) {
        Ok(f) => {
            let rc = system("Echo \"hello from System\"", None, Some(&f));
            let rc2 = system("Copy RAM:libtest.n/none RAM:libtest.n/none2 QUIET", None, Some(&f));
            drop(f);
            let text = dos::fs::File::read_to_vec(&out).map(|v| String::from_utf8_lossy(&v).into_owned());
            t.check(
                rc == Ok(0) && matches!(rc2, Ok(n) if n >= 5) && text.as_deref().map(|s| s.contains("hello from System")).unwrap_or(false),
                format_args!("system: Echo into the given file returns 0, a failing Copy returns {:?}", rc2),
            );
        }
        Err(e) => t.check(false, format_args!("system: open output: {}", dos::describe(&e))),
    }

    // Current directory and no requesters.
    match dos::fs::Lock::new(ND, dos::fs::LockMode::Shared) {
        Ok(dir) => {
            let before = dos::fs::Lock::new("", dos::fs::LockMode::Shared).and_then(|l| l.same(&dir));
            let inside;
            {
                let _g = CurrentDir::enter(&dir);
                inside = dos::fs::Lock::new("", dos::fs::LockMode::Shared).and_then(|l| l.same(&dir));
                let _ = dos::fs::File::write_new("rel.txt", b"r");
            }
            let after = dos::fs::Lock::new("", dos::fs::LockMode::Shared).and_then(|l| l.same(&dir));
            t.check(
                inside == Ok(dos::fs::Sameness::Same) && before != Ok(dos::fs::Sameness::Same) && after == before && dos::fs::exists(&alloc::format!("{}/rel.txt", ND)) == Ok(true),
                format_args!("misc: CurrentDir changes the current directory while it lives and puts it back"),
            );
        }
        Err(e) => t.check(false, format_args!("misc: lock {}: {}", ND, dos::describe(&e))),
    }
    let t0 = amigaos4::arch::timebase();
    let r = {
        let _g = NoRequesters::new();
        dos::fs::metadata("NOSUCHVOLUME:x")
    };
    let ms = ms_since(t0);
    t.check(r.is_err() && ms < 3000, format_args!("misc: with NoRequesters a missing volume fails at once ({} ms, {:?})", ms, r.err().map(|e| dos::describe(&e))));

    // Dates as text.
    let epoch = dos::DateStamp { ds_Days: 0, ds_Minute: 0, ds_Tick: 0 };
    let e = epoch.to_text(DateFormat::Iso, false);
    say!("info: the epoch as text: {:?}", e);
    let when = dos::DateStamp::from_text(Some("2026-11-05"), Some("12:34:56"), DateFormat::Iso);
    let back = when.and_then(|d| d.to_text(DateFormat::Iso, false));
    t.check(
        e.as_ref().map(|d| d.date == "1978-01-01" && d.time == "00:00:00").unwrap_or(false)
            && back.as_ref().map(|d| d.date == "2026-11-05" && d.time == "12:34:56").unwrap_or(false),
        format_args!("misc: DateToStr of the epoch, StrToDate and back ({:?})", back),
    );
    let now = dos::DateStamp::now().and_then(|d| d.to_text(DateFormat::Dos, true));
    say!("info: now, DOS format with Today etc.: {:?}", now);
    t.check(now.map(|d| !d.day.is_empty()).unwrap_or(false), format_args!("misc: DateToStr of now with relative day names"));

    if let Ok(rd) = dos::fs::read_dir(ND) {
        let names: Vec<String> = rd.filter_map(|e| e.ok()).map(|m| m.name).collect();
        for n in names {
            let _ = dos::fs::delete(&alloc::format!("{}/{}", ND, n));
        }
    }
    let _ = dos::fs::delete(ND);
    t.check(dos::fs::exists(ND) == Ok(false), format_args!("dos3: test files removed"));
}

const D4: &str = "RAM:libtest.d4";

fn run_dos4(t: &mut Tally) {
    use alloc::sync::Arc;
    use core::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
    use dos::fs::{create_dir, exists, File, Lock, LockMode};
    use dos::volume::{assign_add, assign_late, assign_lock, assign_path, names, remove_assign, remove_assign_entry, AssignKind, Kinds};

    // The process list.
    let me = dos::process::pid();
    let list = dos::process::processes();
    let mine = list.as_ref().ok().and_then(|v| v.iter().find(|p| p.pid == me).cloned());
    say!("info: processes: {} listed; this one {:?}", list.as_ref().map(|v| v.len()).unwrap_or(0), mine);
    say!("info: processes: PIDs in scan order {:?}", list.as_ref().map(|v| v.iter().map(|p| p.pid).collect::<Vec<_>>()));
    t.check(
        mine.as_ref().map(|p| p.parent_pid == dos::process::parent_pid() && p.priority == task::priority()).unwrap_or(false)
            && list.as_ref().map(|v| v.len() >= 3 && v.iter().all(|p| p.pid <= v[0].pid)).unwrap_or(false),
        format_args!("processes: this Process listed with its parent and priority, the newest first"),
    );
    let (go, ready) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
    let (g2, r2) = (go.clone(), ready.clone());
    match dos::process::spawn("libtest scanned child", move || {
        r2.store(true, AtomicOrdering::Release);
        while !g2.load(AtomicOrdering::Acquire) {
            // SAFETY: a Process; Delay(1) is 1/50 s.
            unsafe { sys::dos_delay(1) };
        }
    }) {
        Ok(c) => {
            within(2000, || ready.load(AtomicOrdering::Acquire));
            let found = dos::process::processes().ok().and_then(|v| v.into_iter().find(|p| p.pid == c.pid()));
            go.store(true, AtomicOrdering::Release);
            let _ = c.wait();
            let gone = dos::process::processes().map(|v| !v.iter().any(|p| p.name == "libtest scanned child")).unwrap_or(false);
            t.check(
                found.as_ref().map(|p| p.name == "libtest scanned child" && p.parent_pid == me).unwrap_or(false) && gone,
                format_args!("processes: a child is listed by name with this Process as parent, and gone after it ends ({:?})", found),
            );
        }
        Err(e) => t.check(false, format_args!("processes: spawn: {}", dos::describe(&e))),
    }

    // A detached process: it reports by message, then is gone on its own.
    match amigaos4::msg::Port::<u32>::new() {
        Ok(port) => {
            let tx = port.sender();
            let pid = dos::process::ProcessBuilder::new("libtest detached").and_then(|b| {
                b.spawn_detached(move || {
                    if let Ok(m) = amigaos4::msg::Message::new(dos::process::pid()) {
                        let _ = tx.send(m);
                    }
                    // A leftover the process forgets: released at its end.
                    if let Ok(s) = amigaos4::signal::Signal::alloc() {
                        core::mem::forget(s);
                    }
                })
            });
            let got = port.recv().map(|m| *m);
            let gone = within(3000, || {
                // SAFETY: FindTask by a NUL-terminated name only reads the lists.
                unsafe { sys::exec_find_task(b"libtest detached\0".as_ptr() as _) }.is_null()
            });
            t.check(
                matches!(pid, Ok(p) if p != 0 && got == Ok(p)) && gone,
                format_args!("process: a detached process ran (PID {:?}, it said {:?}) and was gone by itself", pid, got),
            );
        }
        Err(e) => t.check(false, format_args!("process: Port::new: {}", e)),
    }

    // Assigns.
    let _ = create_dir(D4);
    let (a_dir, b_dir) = (alloc::format!("{}/a", D4), alloc::format!("{}/b", D4));
    let _ = create_dir(&a_dir);
    let _ = create_dir(&b_dir);
    let _ = File::write_new(&alloc::format!("{}/in_a", a_dir), b"a");
    let _ = File::write_new(&alloc::format!("{}/in_b", b_dir), b"b");
    let listed = |n: &str| names(Kinds { volumes: false, devices: false, assigns: true }).map(|v| v.iter().any(|x| x.eq_ignore_ascii_case(n))).unwrap_or(false);

    let p = assign_path("RUSTTP", D4);
    let via = exists("RUSTTP:a/in_a");
    let rm = remove_assign("RUSTTP", AssignKind::Path);
    t.check(
        p.is_ok() && via == Ok(true) && rm.is_ok() && !listed("RUSTTP:"),
        format_args!("assign: a path assign reaches its directory, and is removed"),
    );

    match (Lock::new(&a_dir, LockMode::Shared), Lock::new(&b_dir, LockMode::Shared), Lock::new(&b_dir, LockMode::Shared)) {
        (Ok(la), Ok(lb), Ok(lb_mine)) => {
            let made = assign_lock("RUSTTL:", la).is_ok();
            let a_only = exists("RUSTTL:in_a") == Ok(true) && exists("RUSTTL:in_b") == Ok(false);
            let added = assign_add("RUSTTL", lb, None).is_ok();
            let both = exists("RUSTTL:in_a") == Ok(true) && exists("RUSTTL:in_b") == Ok(true);
            let taken = remove_assign_entry("RUSTTL", &lb_mine).is_ok();
            let a_again = exists("RUSTTL:in_b") == Ok(false) && exists("RUSTTL:in_a") == Ok(true);
            let shown = listed("RUSTTL:");
            let gone = remove_assign("RUSTTL", AssignKind::Lock).is_ok() && !listed("RUSTTL:");
            t.check(
                made && a_only && added && both && taken && a_again && shown && gone,
                format_args!("assign: a lock assign, a second directory added, that entry taken out, the assign removed ({} {} {} {} {} {} {} {})", made, a_only, added, both, taken, a_again, shown, gone),
            );
            // The entry's lock is still ours and usable.
            t.check(lb_mine.path().map(|p| p.ends_with("libtest.d4/b")).unwrap_or(false), format_args!("assign: RemAssignList left the caller's lock alone"));
        }
        _ => t.check(false, format_args!("assign: locks on the test directories")),
    }
    let late = assign_late("RUSTTA", &a_dir);
    let late_listed = listed("RUSTTA:");
    let late_rm = remove_assign("RUSTTA", AssignKind::Late);
    t.check(late.is_ok() && late_listed && late_rm.is_ok() && !listed("RUSTTA:"), format_args!("assign: a late assign made, listed and removed"));
    let bad = Lock::new(&a_dir, LockMode::Shared).map(|l| assign_lock("", l));
    t.check(
        matches!(bad, Ok(Err((AmigaError::DosError(sys::ERROR_INVALID_COMPONENT_NAME), ref l))) if l.path().is_ok()),
        format_args!("assign: an empty name is refused and the lock handed back"),
    );
    drop(bad);

    // Variable notification.
    const VAR: &str = "libtest_watched";
    let _ = dos::delete_var(VAR, dos::VarScope::Global);
    match dos::notify::VarWatcher::new(VAR) {
        Ok(w) => {
            let quiet = !w.changed();
            let _ = dos::set_var(VAR, b"one", dos::VarScope::Global);
            let made = within(1000, || w.changed());
            let _ = dos::set_var(VAR, b"two", dos::VarScope::Global);
            let changed = within(1000, || w.changed());
            let _ = dos::set_var("libtest_other", b"x", dos::VarScope::Global);
            let other = within(200, || w.changed());
            let _ = dos::delete_var(VAR, dos::VarScope::Global);
            let deleted = within(1000, || w.changed());
            drop(w);
            t.check(
                quiet && made && changed && !other && deleted,
                format_args!("varwatch: signalled on create, change and delete, not for another variable ({} {} {} {} {})", quiet, made, changed, other, deleted),
            );
        }
        Err(e) => t.check(false, format_args!("varwatch: new: {}", dos::describe(&e))),
    }
    let _ = dos::delete_var("libtest_other", dos::VarScope::Global);
    // A watch made and dropped; a change after it must not reach a freed hook.
    if let Ok(w) = dos::notify::VarWatcher::new(VAR) {
        drop(w);
    }
    let _ = dos::set_var(VAR, b"after", dos::VarScope::Global);
    let _ = dos::delete_var(VAR, dos::VarScope::Global);
    t.check(true, format_args!("varwatch: a variable changed after its watcher was dropped (no crash)"));

    // LoadSeg / RunCommand.
    match dos::process::SegList::load("C:Version") {
        Ok(seg) => {
            let ok = seg.run("exec.library", 0);
            let again = seg.run("exec.library", 0);
            let missing = seg.run("no_such_thing.library", 0);
            t.check(
                ok == Ok(0) && again == Ok(0) && matches!(missing, Ok(rc) if rc >= 5),
                format_args!("seglist: C:Version loaded and run twice (rc {:?} {:?}), a missing library rc {:?}", ok, again, missing),
            );
        }
        Err(e) => t.check(false, format_args!("seglist: load C:Version: {}", dos::describe(&e))),
    }
    t.check(dos::process::SegList::load("RAM:no_such_program").is_err(), format_args!("seglist: a missing file fails to load"));
    let m0 = avail();
    for _ in 0..50 {
        if let Ok(s) = dos::process::SegList::load("C:Version") {
            drop(s);
        }
    }
    say!("info: seglist: free memory around 50 loads: {:+} bytes", avail() as i64 - m0 as i64);

    for f in ["a/in_a", "b/in_b", "a", "b", ""] {
        let _ = dos::fs::delete(&if f.is_empty() { String::from(D4) } else { alloc::format!("{}/{}", D4, f) });
    }
    t.check(exists(D4) == Ok(false), format_args!("dos4: test files removed"));
}

/// Which way picture.datatype accepts pixels for a new DTST_RAM object.
fn probe_dt_write() {
    use amigaos4::library::Library;
    let (Ok(dl), Ok(il)) = (Library::open("datatypes.library", 45), Library::open("intuition.library", 50)) else {
        return say!("dtprobe: libraries");
    };
    let (Ok(d), Ok(i)) = (dl.interface::<sys::DataTypesIFace>("main", 1), il.interface::<sys::IntuitionIFace>("main", 1)) else {
        return say!("dtprobe: interfaces");
    };
    let (d, i) = (d.as_ptr(), i.as_ptr());
    const DUMMY: u32 = sys::TAG_USER + 0x1000;
    let (w, h) = (16u32, 8u32);
    let px = alloc::vec![0x80u8; (w * h * 4) as usize];
    // (base name or group, source mode, dest mode, DoDTMethod, pixel format)
    for &(by_group, src_mode, dst_mode, via_dt, fmt) in &[
        (false, true, true, true, 2u32),
        (true, false, true, false, 2),
        (true, true, true, false, 2),
        (true, true, true, true, 2),
        (true, false, false, false, 2),
        (false, false, false, false, 2),
        (true, false, true, false, 0),
        (true, true, true, true, 0),
    ] {
        let base = b"picture\0";
        let mut tags = alloc::vec![TagItem { ti_Tag: DUMMY + 101, ti_Data: 1 }];
        if by_group {
            tags.push(TagItem { ti_Tag: DUMMY + 31, ti_Data: u32::from_be_bytes(*b"pict") });
        } else {
            tags.push(TagItem { ti_Tag: DUMMY + 30, ti_Data: base.as_ptr() as u32 });
        }
        if src_mode {
            tags.push(TagItem { ti_Tag: DUMMY + 250, ti_Data: 1 });
        }
        if dst_mode {
            tags.push(TagItem { ti_Tag: DUMMY + 251, ti_Data: 1 });
        }
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: a probe: an open interface and a TAG_DONE-ended list.
        let o = unsafe { ((*d).NewDTObjectA)(d, core::ptr::null(), tags.as_mut_ptr() as *mut _) };
        if o.is_null() {
            say!("dtprobe group {} src {} dst {} dt {} fmt {}: NewDTObject failed", by_group, src_mode, dst_mode, via_dt, fmt);
            continue;
        }
        let mut bmh: u32 = 0;
        let mut g = [TagItem { ti_Tag: DUMMY + 201, ti_Data: &mut bmh as *mut u32 as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our object; one pointer to fill.
        unsafe { ((*d).GetDTAttrsA)(d, o, g.as_mut_ptr() as *mut _) };
        if bmh != 0 {
            let b = bmh as *mut u8;
            // SAFETY: the object's BitMapHeader.
            unsafe {
                core::ptr::write_unaligned(b as *mut u16, w as u16);
                core::ptr::write_unaligned(b.add(2) as *mut u16, h as u16);
                *b.add(8) = 24;
            }
        }
        let bpp = if fmt == 0 { 3 } else { 4 };
        let msg: [u32; 8] = [0x660, px.as_ptr() as u32, fmt, w * bpp, 0, 0, w, h];
        // SAFETY: our object and a pdtBlitPixelArray over px.
        let r = unsafe {
            if via_dt {
                ((*d).DoDTMethodA)(d, o, core::ptr::null_mut(), core::ptr::null_mut(), msg.as_ptr() as _)
            } else {
                ((*i).IDoMethodA)(i, o, msg.as_ptr() as _) as u32
            }
        };
        // SAFETY: a dos call in a Process.
        let err = unsafe { sys::dos_io_err() };
        let mut back = alloc::vec![0u8; (w * h * 4) as usize];
        let rmsg: [u32; 8] = [0x661, back.as_mut_ptr() as u32, 2, w * 4, 0, 0, w, h];
        // SAFETY: our object and a pdtBlitPixelArray over `back`.
        let rr = unsafe { ((*i).IDoMethodA)(i, o, rmsg.as_ptr() as _) };
        let (bw, bh, bd) = if bmh != 0 {
            // SAFETY: the object's BitMapHeader.
            unsafe { (core::ptr::read_unaligned(bmh as *const u16), core::ptr::read_unaligned((bmh + 2) as *const u16), *((bmh + 8) as *const u8)) }
        } else {
            (0, 0, 0)
        };
        say!(
            "dtprobe group {} src {} dst {} dt {} fmt {}: bmh {:#x} WRITEPIXELARRAY {} IoErr {}; READ {} first {:x?} bmh now {}x{}x{}",
            by_group, src_mode, dst_mode, via_dt, fmt, bmh, r, err, rr, &back[..8], bw, bh, bd
        );
        // SAFETY: our object, disposed once.
        unsafe { ((*d).DisposeDTObject)(d, o) };
    }

    // The BitMap route: our own ARGB bitmap handed over with PDTA_BitMap.
    use amigaos4::graphics::{BitMap, BitMapOptions, PixFmt};
    for &(with_mode, set_bmh) in &[(true, true), (false, true), (true, false)] {
        let mut tags = alloc::vec![
            TagItem { ti_Tag: DUMMY + 101, ti_Data: 1 },
            TagItem { ti_Tag: DUMMY + 31, ti_Data: u32::from_be_bytes(*b"pict") },
            TagItem { ti_Tag: DUMMY + 251, ti_Data: 1 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: as above.
        let o = unsafe { ((*d).NewDTObjectA)(d, core::ptr::null(), tags.as_mut_ptr() as *mut _) };
        let Ok(mut bm) = BitMap::new(w, h, 32, &BitMapOptions { pixel_format: Some(PixFmt::A8R8G8B8), clear: true, user_private: true }) else {
            say!("dtprobe bitmap: AllocBitMap failed");
            break;
        };
        if let Some(mut l) = bm.lock() {
            let bpr = l.bytes_per_row() as usize;
            let b = l.bytes_mut();
            for y in 0..h as usize {
                for x in 0..w as usize {
                    b[y * bpr + x * 4..][..4].copy_from_slice(&[0xFF, 0x11 * x as u8, 0x22 * y as u8, 0x33]);
                }
            }
        }
        let mut bmh: u32 = 0;
        let mut g = [TagItem { ti_Tag: DUMMY + 201, ti_Data: &mut bmh as *mut u32 as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: as above.
        unsafe { ((*d).GetDTAttrsA)(d, o, g.as_mut_ptr() as *mut _) };
        if set_bmh && bmh != 0 {
            let b = bmh as *mut u8;
            // SAFETY: the object's BitMapHeader.
            unsafe {
                core::ptr::write_unaligned(b as *mut u16, w as u16);
                core::ptr::write_unaligned(b.add(2) as *mut u16, h as u16);
                *b.add(8) = 24;
            }
        }
        let mut set = alloc::vec![TagItem { ti_Tag: DUMMY + 202, ti_Data: bm.as_ptr() as u32 }];
        if with_mode {
            set.push(TagItem { ti_Tag: DUMMY + 200, ti_Data: 0 });
        }
        set.push(TagItem { ti_Tag: DUMMY + 125, ti_Data: w });
        set.push(TagItem { ti_Tag: DUMMY + 124, ti_Data: h });
        set.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: our object takes the bitmap (the picture class frees it).
        let sr = unsafe { ((*d).SetDTAttrsA)(d, o, core::ptr::null_mut(), core::ptr::null_mut(), set.as_mut_ptr() as *mut _) };
        core::mem::forget(bm);
        let mut back = alloc::vec![0u8; (w * h * 4) as usize];
        let rmsg: [u32; 8] = [0x661, back.as_mut_ptr() as u32, 2, w * 4, 0, 0, w, h];
        // SAFETY: as above.
        let rr = unsafe { ((*i).IDoMethodA)(i, o, rmsg.as_ptr() as _) };
        let path = b"RAM:dtprobe.iff\0";
        // SAFETY: a dos call in a Process with a NUL-terminated name.
        let fh = unsafe { sys::dos_open(path.as_ptr() as _, sys::MODE_NEWFILE) };
        let wmsg: [u32; 5] = [0x650, 0, fh, 0, 0];
        // SAFETY: our object and a dtWrite naming an open file.
        let wr = if fh != 0 { unsafe { ((*d).DoDTMethodA)(d, o, core::ptr::null_mut(), core::ptr::null_mut(), wmsg.as_ptr() as _) } } else { 0 };
        if fh != 0 {
            // SAFETY: the file opened above.
            unsafe { sys::dos_close(fh) };
        }
        let size = amigaos4::dos::fs::metadata("RAM:dtprobe.iff").map(|m| m.size);
        say!("dtprobe bitmap mode {} bmh {}: SetDTAttrs {} READ {} first {:x?} (want ff 11 22 33 at x1y1); DTM_WRITE {} size {:?}", with_mode, set_bmh, sr, rr, &back[(w as usize + 1) * 4..][..4], wr, size);
        // SAFETY: our object, disposed once (with the bitmap).
        unsafe { ((*d).DisposeDTObject)(d, o) };
    }

    // DTM_WRITE on a picture loaded from a file.
    if let Ok(o) = amigaos4::datatypes::DtObject::from_memory(&dt_bmp(), &amigaos4::datatypes::DtOptions::default()) {
        let r = o.save_iff("RAM:dtprobe2.iff");
        let back = amigaos4::datatypes::examine("RAM:dtprobe2.iff").map(|i| i.name);
        say!("dtprobe save of a loaded picture: {:?}, examined as {:?}", r, back);
    }
}

/// How much memory 1000 watchers cost, on RAM: and on SYS:.
fn probe_notify() {
    let _ = dos::fs::create_dir(ND);
    for (path, n) in [(ND, 1000u32), ("SYS:S", 1000), (ND, 1000)] {
        let m0 = avail();
        let mut ok = 0;
        for _ in 0..n {
            ok += dos::notify::Watcher::new(path).is_ok() as u32;
        }
        say!("info: notifyprobe: {} watchers on {} ({} started): {:+} bytes", n, path, ok, avail() as i64 - m0 as i64);
    }
    // Raw: AllocDosObject + FreeDosObject only, then with Start/EndNotify.
    let name = b"RAM:libtest.n ";
    for start in [false, true] {
        let m0 = avail();
        for _ in 0..1000 {
            let sig = amigaos4::signal::Signal::alloc();
            let Ok(sig) = sig else { break };
            let tags = [
                TagItem { ti_Tag: sys::ADO_NOTIFY_NAME, ti_Data: name.as_ptr() as u32 },
                TagItem { ti_Tag: sys::ADO_NOTIFY_METHOD, ti_Data: sys::NRF_SEND_SIGNAL },
                TagItem { ti_Tag: sys::ADO_NOTIFY_TASK, ti_Data: amigaos4::signal::current_task() as u32 },
                TagItem { ti_Tag: sys::ADO_NOTIFY_SIGNAL_NUMBER, ti_Data: sig.bit() as u32 },
                TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
            ];
            // SAFETY: probe code: IDOS is set by the glue; a request made,
            // optionally started and ended, and freed here.
            unsafe {
                let r = sys::dos_alloc_dos_object(sys::DOS_NOTIFYREQUEST, tags.as_ptr());
                if r.is_null() {
                    break;
                }
                if start {
                    sys::dos_start_notify(r as *mut sys::NotifyRequest);
                    sys::dos_end_notify(r as *mut sys::NotifyRequest);
                }
                sys::dos_free_dos_object(sys::DOS_NOTIFYREQUEST, r);
            }
        }
        say!("info: notifyprobe: raw x1000, StartNotify/EndNotify {}: {:+} bytes", start, avail() as i64 - m0 as i64);
    }
    let m0 = avail();
    for _ in 0..1000 {
        let _ = amigaos4::signal::Signal::alloc();
    }
    say!("info: notifyprobe: 1000 signal bits alone: {:+} bytes", avail() as i64 - m0 as i64);
    let _ = dos::fs::delete(ND);
}

// ---------------------------------------------------------------------------
// timer
// ---------------------------------------------------------------------------

fn run_timer(t: &mut Tally) {
    use amigaos4::timer::{self as tm, AmigaTimer, TimerUnit, TimerVal};
    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::ITimer }.is_null();
    let e0 = tm::eclock();
    let u0 = tm::get_up_time();
    say!("info: timer: E-clock {:?}, up time {:?}, system time {:?}", e0, u0, tm::sys_time());
    t.check(null && e0.map(|e| e.freq > 0).unwrap_or(false) && u0 > TimerVal::ZERO, format_args!("timer: ITimer opened by the crate (ITimer left NULL): E-clock and up time"));

    let ms = |f: &mut dyn FnMut()| {
        let a = tm::eclock().unwrap_or(amigaos4::timer::EClock { ticks: 0, freq: 1 });
        f();
        let b = tm::eclock().unwrap_or(a);
        b.micros_since(&a) / 1000
    };

    let mut timer = match AmigaTimer::open(TimerUnit::MicroHz) {
        Ok(x) => x,
        Err(e) => return t.check(false, format_args!("timer: open MicroHz: {}", e)),
    };
    let d = ms(&mut || {
        let _ = timer.delay(TimerVal::from_millis(100));
    });
    t.check((95..400).contains(&d), format_args!("timer: a 100 ms delay took {} ms", d));

    // The bug: a delay while a started request is pending.
    timer.start(TimerVal::from_millis(300));
    let pending_after_start = !timer.is_done();
    let mask = timer.signal_mask();
    let d2 = ms(&mut || {
        let _ = timer.delay(TimerVal::from_millis(50));
    });
    let still = !timer.is_done();
    let got_signal = within(1000, || amigaos4::signal::peek(mask) != 0);
    let done = timer.is_done();
    timer.finish();
    let _ = amigaos4::signal::take(mask);
    t.check(
        pending_after_start && (45..250).contains(&d2) && still && got_signal && done,
        format_args!("timer: a 50 ms delay ({} ms) while a 300 ms request is pending leaves it running, and its signal still arrives", d2),
    );

    timer.start(TimerVal::new(5, 0));
    let d3 = ms(&mut || timer.abort());
    t.check(d3 < 100 && timer.is_done(), format_args!("timer: a 5 s request aborted in {} ms", d3));

    let a = tm::get_up_time();
    let _ = timer.delay(TimerVal::from_millis(20));
    let b = timer.up_time();
    let diff = b.checked_sub(a).map(|v| v.as_millis());
    t.check(matches!(diff, Some(x) if (15..300).contains(&x)), format_args!("timer: GetUpTime moves on by {:?} ms across a 20 ms delay", diff));
    tm::micro_delay(0);
    let md = ms(&mut || tm::micro_delay(2000));
    t.check(md >= 2, format_args!("timer: MicroDelay(2000) took {} ms", md));
    drop(timer);

    match AmigaTimer::open(TimerUnit::EClock) {
        Ok(mut et) => {
            let freq = et.eclock().freq as u64;
            let d4 = ms(&mut || {
                let _ = et.delay(TimerVal::from_millis(100));
            });
            let d5 = ms(&mut || {
                let _ = et.delay_ticks(freq / 10);
            });
            t.check((95..400).contains(&d4) && (95..400).contains(&d5), format_args!("timer: EClock unit: 100 ms as a TimerVal ({} ms) and as {} ticks ({} ms)", d4, freq / 10, d5));
        }
        Err(e) => t.check(false, format_args!("timer: open EClock: {}", e)),
    }
    match AmigaTimer::open(TimerUnit::WaitUntil) {
        Ok(mut wt) => {
            let target = wt.get_sys_time().checked_add(TimerVal::from_millis(100)).unwrap_or(TimerVal::ZERO);
            let d6 = ms(&mut || {
                let _ = wt.delay(target);
            });
            let bad = wt.delay_ticks(5).is_err();
            t.check((80..400).contains(&d6) && bad, format_args!("timer: WaitUntil now + 100 ms waited {} ms; delay_ticks refused on a non-EClock unit", d6));
        }
        Err(e) => t.check(false, format_args!("timer: open WaitUntil: {}", e)),
    }

    // A timer dropped with a request pending, and memory around cycles.
    let d7 = ms(&mut || {
        if let Ok(mut x) = AmigaTimer::open(TimerUnit::VBlank) {
            x.start(TimerVal::new(10, 0));
        }
    });
    t.check(d7 < 200, format_args!("timer: dropping a timer with a 10 s request pending took {} ms", d7));
    let m0 = avail();
    for _ in 0..200 {
        if let Ok(mut x) = AmigaTimer::open(TimerUnit::MicroHz) {
            let _ = x.delay(TimerVal::new(0, 500));
            x.start(TimerVal::new(1, 0));
        }
    }
    say!("info: timer: free memory around 200 open/delay/start/drop cycles: {:+} bytes", avail() as i64 - m0 as i64);

    // Waiting with a limit, ended by Ctrl-C.
    use amigaos4::signal::{Cancel, Signal};
    use amigaos4::timer::{Waited, Waiter};
    let (Ok(mut w), Ok(s)) = (Waiter::new(), Signal::alloc()) else {
        return t.check(false, format_args!("waiter: Waiter::new / Signal::alloc"));
    };
    let t0 = amigaos4::arch::timebase();
    let r = w.wait_ms(s.mask(), 150);
    let ms = ms_since(t0);
    t.check(r == Ok(Waited::TimedOut) && (140..1000).contains(&ms), format_args!("waiter: nothing came: timed out after {} ms ({:?})", ms, r));
    // SAFETY: our own task and bit.
    unsafe { s.target().send() };
    let r = w.wait_ms(s.mask(), 2000);
    t.check(r == Ok(Waited::Signals(s.mask())) && !s.take(), format_args!("waiter: a signal already set ends the wait at once, cleared ({:?})", r));
    // SAFETY: our own task; CTRL-C as the Shell's Break would send it.
    unsafe { amigaos4::signal::SignalTarget::new(amigaos4::signal::current_task(), Cancel::MASK).send() };
    let r = w.wait_ms(s.mask(), 2000);
    t.check(r == Err(AmigaError::Cancelled) && Cancel::check().is_ok(), format_args!("waiter: Ctrl-C ends the wait with Cancelled, cleared ({:?})", r));
    // SAFETY: as above.
    unsafe { amigaos4::signal::SignalTarget::new(amigaos4::signal::current_task(), Cancel::MASK).send() };
    t.check(Cancel::check() == Err(AmigaError::Cancelled) && Cancel::check().is_ok(), format_args!("waiter: Cancel::check sees Ctrl-C once"));
    match amigaos4::msg::Port::<u32>::new() {
        Ok(p) => {
            let t1 = amigaos4::arch::timebase();
            let none = p.recv_timeout(&mut w, 120);
            let ms1 = ms_since(t1);
            let tx = p.sender();
            let h = amigaos4::task::spawn(b"libtest sender\0", move || {
                if let Ok(m) = amigaos4::msg::Message::new(42u32) {
                    let _ = tx.send(m);
                }
            });
            let got = p.recv_timeout(&mut w, 2000).map(|m| m.map(|m| *m));
            if let Ok(h) = h {
                let _ = h.join();
            }
            t.check(
                matches!(none, Ok(None)) && (110..1000).contains(&ms1) && got == Ok(Some(42)),
                format_args!("waiter: Port::recv_timeout gives None after {} ms, then a child's message ({:?})", ms1, got),
            );
        }
        Err(e) => t.check(false, format_args!("waiter: Port::new: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// net (bsdsocket)
// ---------------------------------------------------------------------------

fn run_net(t: &mut Tally) {
    use amigaos4::bsdsocket::{self as bs, SockAddrIn, SocketLib};
    use amigaos4::net::{self, Shutdown};
    use core::time::Duration;

    let lib = match SocketLib::open(4) {
        Ok(l) => l,
        Err(e) => return t.check(false, format_args!("net: open bsdsocket.library: {}", e)),
    };
    say!("info: net: bsdsocket.library {:?}, host name {:?}", lib.version(), lib.hostname());
    let lo = |port: u16| SockAddrIn::v4([127, 0, 0, 1], port);

    let r1 = lib.resolve("localhost");
    let r2 = lib.resolve("127.0.0.1");
    t.check(
        r1.as_ref().map(|v| v.contains(&[127, 0, 0, 1])).unwrap_or(false) && r2 == Ok(alloc::vec![[127, 0, 0, 1]]),
        format_args!("net: resolve localhost {:?} and a dotted address", r1),
    );
    let refuse = lib.set_opt(0, bs::SOL_SOCKET, bs::SO_RCVTIMEO, 1);
    t.check(matches!(refuse, Err(AmigaError::InvalidArgument(_))), format_args!("net: set_opt refuses SO_RCVTIMEO (not a 4-byte value)"));

    // UDP loopback, wait, timeouts.
    let (a, b) = match (lib.udp_bind(&lo(0)), lib.udp_bind(&lo(0))) {
        (Ok(a), Ok(b)) => (a, b),
        (a, b) => return t.check(false, format_args!("net: udp_bind: {:?} {:?}", a.err(), b.err())),
    };
    let pb = b.local_addr().map(|x| x.sin_port).unwrap_or(0);
    let t0 = amigaos4::arch::timebase();
    let idle = lib.wait(&[&b.sock], &[], 0, Some(Duration::from_millis(200)));
    let idle_ms = ms_since(t0);
    t.check(
        matches!(&idle, Ok(r) if r.timed_out && r.readable == [false]) && (150..1000).contains(&idle_ms),
        format_args!("net: WaitSelect on a quiet socket times out after {} ms", idle_ms),
    );
    let sent = a.send_to(b"ping over udp", &lo(pb));
    let ready = lib.wait(&[&b.sock], &[&a.sock], 0, Some(Duration::from_secs(2)));
    let mut buf = [0u8; 64];
    let got = b.recv_from(&mut buf);
    let pa = a.local_addr().map(|x| x.sin_port).unwrap_or(1);
    t.check(
        sent == Ok(13)
            && matches!(&ready, Ok(r) if r.readable == [true] && r.writable == [true] && !r.timed_out)
            && matches!(got, Ok((13, from)) if from.sin_port == pa && from.ip() == [127, 0, 0, 1])
            && &buf[..13] == b"ping over udp",
        format_args!("net: UDP datagram over loopback: WaitSelect says readable/writable, recvfrom gives the sender port {}", pa),
    );
    let _ = b.sock.set_nonblocking(true);
    let wb = b.recv_from(&mut buf);
    t.check(matches!(&wb, Err(e) if net::would_block(e)), format_args!("net: non-blocking recv on an empty socket is EWOULDBLOCK ({:?})", wb.err()));
    let _ = b.sock.set_nonblocking(false);
    let to = b.sock.set_read_timeout(Some(Duration::from_millis(200)));
    let t1 = amigaos4::arch::timebase();
    let tr = b.recv_from(&mut buf);
    let tms = ms_since(t1);
    t.check(to.is_ok() && tr.is_err() && (150..1500).contains(&tms), format_args!("net: SO_RCVTIMEO 200 ms (a timeval): recv gave up after {} ms ({:?})", tms, tr.err()));

    match amigaos4::signal::Signal::alloc() {
        Ok(sig) => {
            // SAFETY: our own task, alive while it runs this.
            unsafe { sig.target().send() };
            let r = lib.wait(&[&b.sock], &[], sig.mask(), Some(Duration::from_secs(2)));
            t.check(matches!(&r, Ok(x) if x.signals == sig.mask() && !x.timed_out), format_args!("net: WaitSelect returns for an exec signal ({:?})", r.as_ref().map(|x| x.signals)));
        }
        Err(e) => t.check(false, format_args!("net: Signal::alloc: {}", e)),
    }
    drop((a, b));

    // TCP echo against a child process with its own library base.
    let listener = match lib.tcp_listen(&lo(0), 4) {
        Ok(l) => l,
        Err(e) => return t.check(false, format_args!("net: tcp_listen: {}", e)),
    };
    let port = listener.local_addr().map(|x| x.sin_port).unwrap_or(0);
    let child = dos::process::spawn("libtest tcp client", move || -> Result<usize, AmigaError> {
        let lib = SocketLib::open(4)?;
        let mut s = lib.tcp_connect(&SockAddrIn::v4([127, 0, 0, 1], port))?;
        s.sock.set_nodelay(true)?;
        let data: Vec<u8> = (0..20_000u32).map(|i| (i * 7 + 3) as u8).collect();
        s.write_all(&data)?;
        s.sock.shutdown(Shutdown::Write)?;
        let mut back = alloc::vec![0u8; data.len()];
        s.read_exact(&mut back)?;
        Ok(if back == data { data.len() } else { 0 })
    });
    let ready = lib.wait(&[&listener.sock], &[], 0, Some(Duration::from_secs(5)));
    let mut echoed = 0usize;
    let mut peer = None;
    if matches!(&ready, Ok(r) if r.readable == [true]) {
        if let Ok((mut c, from)) = listener.accept() {
            peer = Some(from.ip());
            let _ = c.sock.set_read_timeout(Some(Duration::from_secs(5)));
            let mut buf = [0u8; 4096];
            loop {
                match c.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if c.write_all(&buf[..n]).is_err() {
                            break;
                        }
                        echoed += n;
                    }
                }
            }
            let _ = c.sock.take_error();
        }
    }
    let r = child.and_then(|c| c.wait());
    t.check(
        echoed == 20_000 && matches!(r, Ok(Ok(20_000))) && peer == Some([127, 0, 0, 1]),
        format_args!("net: 20000 bytes echoed over TCP loopback to a child process ({} echoed, child {:?})", echoed, r),
    );

    let m0 = avail();
    for _ in 0..200 {
        if let Ok(u) = lib.udp_bind(&lo(0)) {
            let _ = u.sock.set_linger(Some(Duration::from_secs(1)));
        }
    }
    say!("info: net: free memory around 200 sockets made and closed: {:+} bytes", avail() as i64 - m0 as i64);
    let leak = lib.udp_bind(&lo(0)).map(|u| u.sock.fd());
    t.check(matches!(leak, Ok(fd) if fd < 8), format_args!("net: sockets are closed on drop (a new one gets number {:?})", leak));
}

// ---------------------------------------------------------------------------
// locale
// ---------------------------------------------------------------------------

fn run_locale(t: &mut Tally) {
    use amigaos4::locale::{id, Catalog, CatalogOptions, Class, Locale, Sort};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::ILocale }.is_null();
    let loc = match Locale::current() {
        Ok(l) => l,
        Err(e) => return t.check(false, format_args!("locale: Locale::current: {}", e)),
    };
    let lang = loc.language();
    let english = lang.eq_ignore_ascii_case("english");
    say!("info: locale: language {:?}, code set {}, country {:?}, GMT offset {} min", lang, loc.code_set(), core::str::from_utf8(&loc.country_code()), loc.gmt_offset());
    t.check(null && !lang.is_empty(), format_args!("locale: the current locale through the crate's own interface (ILocale left NULL)"));
    {
        use amigaos4::locale::FmtArg::{Int, Str};
        let plain = loc.format_lossy("%ld eyes, %s and %08lx|%-5ld|%.3s", &[Int(2), Str(b"feet"), Int(0xBEEF), Int(-7), Str(b"abcdef")]);
        let moved = loc.format_lossy("%3$ld ears, %1$ld eyes and %2$ld feet", &[Int(2), Int(3), Int(4)]);
        let grouped = loc.format_lossy("%lU", &[Int(1_234_567)]);
        let refused = loc.format("%ld %ld", &[Int(1)]);
        say!("info: locale: FormatString {:?} / {:?} / %lU of 1234567 {:?}", plain, moved, grouped);
        t.check(
            plain.as_deref() == Ok("2 eyes, feet and 0000BEEF|-7   |abc")
                && moved.as_deref() == Ok("4 ears, 2 eyes and 3 feet")
                && grouped.as_ref().map(|g| g.chars().filter(|c| c.is_ascii_digit()).collect::<String>() == "1234567").unwrap_or(false)
                && refused.is_err(),
            format_args!("locale: FormatString32 with strings, numbers, widths, positions; a missing argument refused"),
        );
    }
    let names = (loc.text_lossy(id::DAY_1), loc.text_lossy(id::MON_1 + 11), loc.text_lossy(id::YESSTR), loc.text(9999));
    say!("info: locale: day 1 {:?}, month 12 {:?}, yes {:?}", names.0, names.1, names.2);
    t.check(
        names.0.as_deref().map(|s| !english || s == "Sunday").unwrap_or(false)
            && names.1.as_deref().map(|s| !english || s == "December").unwrap_or(false)
            && names.3.is_none(),
        format_args!("locale: GetLocaleStr day and month names, None past the end"),
    );
    let cls = (
        loc.is(Class::Digit, b'7' as u32),
        loc.is(Class::Digit, b'x' as u32),
        loc.is(Class::XDigit, b'f' as u32),
        loc.is(Class::Upper, b'Q' as u32),
        loc.is(Class::Space, b' ' as u32),
        loc.is(Class::Punct, b'!' as u32),
        loc.is(Class::Alpha, 0xE9),
    );
    t.check(cls == (true, false, true, true, true, true, true), format_args!("locale: Is* on 7, x, f, Q, space, !, e-acute ({:?})", cls));
    t.check(
        loc.to_upper(b'a' as u32) == b'A' as u32 && loc.to_lower(b'Z' as u32) == b'z' as u32 && loc.to_upper(0xE9) == 0xC9,
        format_args!("locale: ConvToUpper/Lower on ASCII and Latin-1 e-acute"),
    );
    let c = (
        loc.cmp(b"abc", b"ABC", Sort::Ascii),
        loc.cmp(b"a", b"b", Sort::Ascii),
        loc.cmp_n(b"abcdef", b"abcxyz", 3, Sort::Ascii),
        loc.cmp(b"pere", b"p\xe8re", Sort::Collate1),
        loc.cmp(b"pere", b"p\xe8re", Sort::Collate2),
    );
    say!("info: locale: StrnCmp results {:?}", c);
    t.check(
        c.0 == Ok(Ordering::Equal) && c.1 == Ok(Ordering::Less) && c.2 == Ok(Ordering::Equal) && c.4 != Ok(Ordering::Equal),
        format_args!("locale: StrnCmp: ASCII without case, length-limited, Collate2 tells accents apart"),
    );
    let k1 = loc.sort_key(b"apple", Sort::Collate2);
    let k2 = loc.sort_key(b"Banana", Sort::Collate2);
    t.check(matches!((&k1, &k2), (Ok(a), Ok(b)) if !a.is_empty() && a < b), format_args!("locale: StrConvert keys sort apple before Banana ({:?})", k1.as_ref().map(|v| v.len())));

    let epoch = dos::DateStamp { ds_Days: 0, ds_Minute: 0, ds_Tick: 0 };
    let f = loc.format_date_lossy("%Y-%m-%d %H:%M:%S|%A|%%", &epoch);
    let leap = dos::DateStamp { ds_Days: 366, ds_Minute: 61, ds_Tick: 0 };
    let f2 = loc.format_date_lossy("%Y-%m-%d %H:%M", &leap);
    t.check(
        f.as_deref().map(|s| s.starts_with("1978-01-01 00:00:00|") && s.ends_with("|%") && (!english || s.contains("Sunday"))).unwrap_or(false)
            && f2.as_deref() == Ok("1979-01-02 01:01"),
        format_args!("locale: FormatDate {:?} and {:?}", f, f2),
    );
    let p = (
        loc.parse_date("%Y-%m-%d", b"1978-01-01"),
        loc.parse_date("%Y-%m-%d", b"1979-01-02"),
        loc.parse_date("%Y-%m-%d", b"2000-02-30"),
        loc.parse_date("%Y-%m-%d", b"1978/01/01"),
        loc.parse_date("%j", b"001"),
    );
    t.check(
        matches!(p.0, Ok(Some(d)) if d.ds_Days == 0) && matches!(p.1, Ok(Some(d)) if d.ds_Days == 366) && p.2 == Ok(None) && p.3 == Ok(None) && p.4.is_err(),
        format_args!("locale: ParseDate: two dates, 30 February and a wrong separator refused, %j refused before the call ({:?})", (p.0, p.2, p.3)),
    );

    // Catalogs: a missing one falls back to the built-in strings.
    let _ = dos::set_var("libtest_dummy", b"x", dos::VarScope::Local);
    let missing = Catalog::open("libtest_nonexistent.catalog", &CatalogOptions { builtin_language: Some("english"), ..Default::default() });
    match &missing {
        Ok(cat) => {
            let s = cat.get(1, c"Hello built-in");
            t.check(!cat.is_loaded() && s == b"Hello built-in", format_args!("locale: a missing catalog is Ok and gives the built-in string"));
        }
        Err(e) => t.check(false, format_args!("locale: a missing catalog is an error: {}", e)),
    }
    let sys = Catalog::open("Sys/libs.catalog", &CatalogOptions::default());
    say!("info: locale: Sys/libs.catalog loaded {:?}; string 1000 {:?}", sys.as_ref().map(|c| c.is_loaded()), sys.as_ref().map(|c| String::from_utf8_lossy(c.get(1000, c"DEF")).into_owned()));
    t.check(sys.is_ok(), format_args!("locale: open Sys/libs.catalog (loaded or built-in)"));
    drop((missing, sys));

    let m0 = avail();
    for _ in 0..200 {
        if let Ok(l) = Locale::current() {
            let _ = l.format_date("%Y", &epoch);
        }
        let _ = Catalog::open("libtest_nonexistent.catalog", &CatalogOptions::default());
    }
    say!("info: locale: free memory around 200 locale/catalog/format cycles: {:+} bytes", avail() as i64 - m0 as i64);
}

// ---------------------------------------------------------------------------
// application + prefsobjects
// ---------------------------------------------------------------------------

fn next_within(app: &amigaos4::application::Application, ms: u64) -> Option<(amigaos4::application::AppId, amigaos4::application::Event)> {
    let t0 = amigaos4::arch::timebase();
    loop {
        if let Some(e) = app.next() {
            return Some(e);
        }
        if ms_since(t0) >= ms {
            return None;
        }
        busy_ms(5);
    }
}

fn run_app(t: &mut Tally) {
    use amigaos4::application::{self as ap, Application, Event, RegisterOptions, Request};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IApplication }.is_null();
    let app = match Application::register("libtest", &RegisterOptions { url_identifier: Some("rust.test"), no_icon: true, ..Default::default() }) {
        Ok(a) => a,
        Err(e) => return t.check(false, format_args!("application: register: {}", e)),
    };
    let ident = app.identifier();
    let id = app.id();
    say!("info: application: id {:?}, identifier {:?}", id, ident);
    let found = (ap::find("libtest"), ap::find_identifier(&ident));
    let listed = ap::list().map(|v| v.iter().any(|(i, n)| *i == id && n.contains("libtest")));
    t.check(
        null && id.0 != 0 && ident.contains("libtest") && found == (Ok(Some(id)), Ok(Some(id))) && listed == Ok(true),
        format_args!("application: registered through a v2 interface the crate opened; found by name and identifier, listed"),
    );

    let s1 = app.send(id, Request::Custom("hello from libtest"));
    let s2 = app.send(id, Request::OpenDoc("RAM:some/doc.txt"));
    let s3 = app.send(id, Request::Quit);
    // The strings sent above were freed when send returned: what arrives
    // must be the library's copies.
    let e1 = next_within(&app, 1000);
    let e2 = next_within(&app, 1000);
    let e3 = next_within(&app, 1000);
    let more = next_within(&app, 100);
    t.check(
        s1.is_ok() && s2.is_ok() && s3.is_ok()
            && e1 == Some((id, Event::Custom(String::from("hello from libtest"))))
            && e2 == Some((id, Event::OpenDoc(String::from("RAM:some/doc.txt"))))
            && e3 == Some((id, Event::Quit))
            && more.is_none(),
        format_args!("application: Custom, OpenDoc and Quit sent to ourselves arrive in order with their texts ({:?})", e1),
    );

    let u1 = Application::register("libtest_unique", &RegisterOptions { unique: true, no_icon: true, ..Default::default() });
    let u2 = Application::register("libtest_unique", &RegisterOptions { unique: true, no_icon: true, ..Default::default() });
    let got = u1.as_ref().ok().and_then(|a| next_within(a, 1000));
    t.check(u1.is_ok() && u2.is_err() && matches!(got, Some((_, Event::Unique))), format_args!("application: a unique application refuses a second instance and tells the first ({:?})", got));
    drop((u1, u2));

    drop(app);
    let gone = ap::find("libtest");
    t.check(gone == Ok(None), format_args!("application: unregistered on drop ({:?})", gone));
    let again = Application::register("libtest", &RegisterOptions { no_icon: true, ..Default::default() }).map(|a| a.id());
    t.check(matches!(again, Ok(n) if n != id), format_args!("application: a new registration gets a new ID ({:?})", again));

    let m0 = avail();
    for k in 0..100u32 {
        if let Ok(a) = Application::register("libtest_cycle", &RegisterOptions { no_icon: true, ..Default::default() }) {
            let _ = a.send(a.id(), Request::Custom(&alloc::format!("n{}", k)));
            let _ = next_within(&a, 200);
        }
    }
    say!("info: application: free memory around 100 register/send/receive/unregister cycles: {:+} bytes", avail() as i64 - m0 as i64);

    run_prefs(t);
}

fn run_prefs(t: &mut Tally) {
    use amigaos4::prefs::{PrefsArray, PrefsDict, Value};

    let build = || -> Result<PrefsDict, AmigaError> {
        let mut d = PrefsDict::new()?;
        d.set_str("name", "libtest")?;
        d.set_int("answer", 42)?;
        d.set_int("negative", -7)?;
        d.set_bool("enabled", true)?;
        let mut a = PrefsArray::new()?;
        a.push(Value::Str(String::from("one")))?;
        a.push(Value::Str(String::from("two")))?;
        a.push(Value::Int(3))?;
        d.set("list", Value::Array(a))?;
        let mut inner = PrefsDict::new()?;
        inner.set_str("deep", "value")?;
        d.set("inner", Value::Dict(inner))?;
        Ok(d)
    };
    let d = match build() {
        Ok(d) => d,
        Err(e) => return t.check(false, format_args!("prefs: build a dictionary: {}", e)),
    };
    let arr = d.array("list");
    t.check(
        d.len() == 6
            && d.get_str("name").as_deref() == Some("libtest")
            && d.get_int("answer") == Some(42)
            && d.get_int("negative") == Some(-7)
            && d.get_bool("enabled", false)
            && d.get_str("answer").is_none()
            && d.get_int("missing").is_none()
            && arr.as_ref().map(|a| a.len() == 3 && a.get_str(0).as_deref() == Some("one") && a.get_int(2) == Some(3) && a.get_int(0).is_none()).unwrap_or(false)
            && d.has_dict("inner"),
        format_args!("prefs: a dictionary of string, numbers, bool, array and dictionary reads back by type"),
    );

    // The file name is a path (a bare name would land in the current
    // directory).
    let w2 = d.write("RAM:libtest_prefs.xml");
    let xml = dos::fs::File::read_to_vec("RAM:libtest_prefs.xml").map(|v| String::from_utf8_lossy(&v).into_owned());
    t.check(
        w2.is_ok() && xml.as_deref().map(|s| s.contains("libtest") && s.contains("42")).unwrap_or(false),
        format_args!("prefs: WritePrefs to a full path writes XML there ({} bytes)", xml.as_ref().map(|s| s.len()).unwrap_or(0)),
    );
    if let Ok(x) = &xml {
        say!("info: prefs: the XML starts {:?}", x.chars().take(120).collect::<String>());
    }

    match PrefsDict::new() {
        Ok(mut back) => {
            let _ = back.set_str("old", "gone after read");
            let r = back.read("RAM:libtest_prefs.xml");
            let arr = back.array("list");
            t.check(
                r.is_ok()
                    && back.get_str("old").is_none()
                    && back.get_str("name").as_deref() == Some("libtest")
                    && back.get_int("answer") == Some(42)
                    && back.get_int("negative") == Some(-7)
                    && back.get_bool("enabled", false)
                    && arr.map(|a| amigaos4::prefs::strings(&a) == ["one", "two"] && a.get_int(2) == Some(3)).unwrap_or(false),
                format_args!("prefs: ReadPrefs clears the dictionary and reads every value back ({:?})", r),
            );
        }
        Err(e) => t.check(false, format_args!("prefs: new: {}", e)),
    }
    let mut d = d;
    let rm = d.remove("answer");
    t.check(rm.is_ok() && d.get_int("answer").is_none() && d.len() == 5 && d.remove("answer").is_err(), format_args!("prefs: remove a key; removing it again fails"));
    drop(d);
    let _ = dos::fs::delete("RAM:libtest_prefs.xml");

    let m0 = avail();
    for _ in 0..200 {
        let _ = build();
    }
    say!("info: prefs: free memory around 200 dictionaries built and dropped: {:+} bytes", avail() as i64 - m0 as i64);
}

// ---------------------------------------------------------------------------
// graphics (off-screen)
// ---------------------------------------------------------------------------

fn run_gfx(t: &mut Tally) {
    use amigaos4::graphics::{BitMap, BitMapOptions, Font, PixFmt, Rect, Region};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IGraphics }.is_null();
    let argb = BitMapOptions { pixel_format: Some(PixFmt::A8R8G8B8), clear: true, user_private: true };
    let mut bm = match BitMap::new(64, 48, 32, &argb) {
        Ok(b) => b,
        Err(e) => return t.check(false, format_args!("gfx: a 64x48 ARGB bitmap: {}", e)),
    };
    say!("info: gfx: ARGB bitmap {}x{} allocated {}x{}, depth {}, RTG {}, format {:?}", bm.width(), bm.height(), bm.allocated_width(), bm.allocated_height(), bm.depth(), bm.is_rtg(), bm.pixel_format());
    t.check(
        null && bm.allocated_width() >= 64 && bm.allocated_height() >= 48 && bm.pixel_format() == Some(PixFmt::A8R8G8B8),
        format_args!("gfx: a user-private ARGB bitmap through the crate's own interface (IGraphics left NULL)"),
    );

    // Colour fills, read back through the API and from memory.
    {
        let mut rp = match bm.rastport() {
            Ok(r) => r,
            Err(e) => return t.check(false, format_args!("gfx: rastport: {}", e)),
        };
        rp.clear_color(0xFF10_2030);
        let corners = [rp.read_color(0, 0), rp.read_color(63, 47), rp.read_color(32, 24)];
        let fill = rp.fill_color(Rect::new(10, 10, 19, 14), 0xFF00_FF00);
        let inside = [rp.read_color(10, 10), rp.read_color(19, 14), rp.read_color(15, 12)];
        let outside = [rp.read_color(9, 10), rp.read_color(20, 14), rp.read_color(15, 15)];
        let rgb = |v: &Result<u32, AmigaError>| v.as_ref().map(|c| c & 0x00FF_FFFF).unwrap_or(1);
        t.check(
            corners.iter().all(|c| rgb(c) == 0x10_2030) && fill.is_ok() && inside.iter().all(|c| rgb(c) == 0x00_FF00) && outside.iter().all(|c| rgb(c) == 0x10_2030),
            format_args!("gfx: SetRastColor and RectFillColor read back with ReadPixelColor, edges exact"),
        );
        let refused = (
            rp.fill_color(Rect::new(60, 40, 64, 47), 0),
            rp.fill(Rect::new(5, 5, 4, 9)),
            rp.line(0, 0, 64, 10),
            rp.ellipse(5, 5, 6, 3),
            rp.read_color(-1, 0),
            rp.write_pixel(0, 48),
        );
        t.check(
            refused.0.is_err() && refused.1.is_err() && refused.2.is_err() && refused.3.is_err() && refused.4.is_err() && refused.5.is_err(),
            format_args!("gfx: a rectangle past the edge or reversed, a line or ellipse leaving the bitmap, pixels outside: all refused"),
        );
    }
    match bm.lock() {
        Some(l) => {
            let bpr = l.bytes_per_row() as usize;
            let b = l.bytes();
            let px = |x: usize, y: usize| [b[bpr * y + x * 4 + 1], b[bpr * y + x * 4 + 2], b[bpr * y + x * 4 + 3]];
            t.check(
                l.pixel_format() == Some(PixFmt::A8R8G8B8) && px(0, 0) == [0x10, 0x20, 0x30] && px(15, 12) == [0, 0xFF, 0] && bpr >= 256,
                format_args!("gfx: LockBitMap: the pixels in memory as A, R, G, B bytes ({} bytes per row)", bpr),
            );
        }
        None => t.check(false, format_args!("gfx: a user-private bitmap could not be locked")),
    }

    // Pixel arrays and conversion.
    {
        let mut rp = match bm.rastport() {
            Ok(r) => r,
            Err(_) => return,
        };
        let mut src = alloc::vec![0u8; 16 * 16 * 4];
        for (k, px) in src.chunks_mut(4).enumerate() {
            let (x, y) = (k % 16, k / 16);
            px.copy_from_slice(&[0xFF, (x * 13) as u8, (y * 7) as u8, (x * y) as u8]);
        }
        let w = rp.write_pixels(Rect::new(5, 5, 20, 20), PixFmt::A8R8G8B8, &src, 64);
        let mut back = alloc::vec![0u8; 16 * 16 * 4];
        let r1 = rp.read_pixels(Rect::new(5, 5, 20, 20), PixFmt::A8R8G8B8, &mut back, 64);
        let mut rgb = alloc::vec![0u8; 16 * 16 * 3];
        let r2 = rp.read_pixels(Rect::new(5, 5, 20, 20), PixFmt::R8G8B8, &mut rgb, 48);
        let same = src.chunks(4).zip(back.chunks(4)).all(|(a, b)| a[1..] == b[1..]);
        let conv = src.chunks(4).zip(rgb.chunks(3)).all(|(a, b)| a[1..] == b[..]);
        t.check(w.is_ok() && r1.is_ok() && r2.is_ok() && same && conv, format_args!("gfx: WritePixelArray 16x16 ARGB, ReadPixelArray back as ARGB and as RGB"));
        let short = alloc::vec![0u8; 16 * 16 * 4 - 1];
        let mut short2 = alloc::vec![0u8; 100];
        t.check(
            rp.write_pixels(Rect::new(0, 0, 15, 15), PixFmt::A8R8G8B8, &short, 64).is_err() && rp.read_pixels(Rect::new(0, 0, 15, 15), PixFmt::A8R8G8B8, &mut short2, 32).is_err(),
            format_args!("gfx: pixel arrays one byte short, or with too small a row pitch, are refused"),
        );
    }

    // Blits.
    match BitMap::new_like(&bm, 32, 32) {
        Ok(mut dst) => {
            let b1 = dst.blit_from(&bm, Rect::new(5, 5, 20, 20), 8, 8);
            let b2 = dst.blit_from(&bm, Rect::new(50, 40, 63, 47), 25, 25);
            let mut a = alloc::vec![0u8; 16 * 16 * 4];
            let mut b = alloc::vec![0u8; 16 * 16 * 4];
            let ra = bm.rastport().and_then(|mut rp| rp.read_pixels(Rect::new(5, 5, 20, 20), PixFmt::A8R8G8B8, &mut a, 64));
            let rb = dst.rastport().and_then(|mut rp| rp.read_pixels(Rect::new(8, 8, 23, 23), PixFmt::A8R8G8B8, &mut b, 64));
            let within = dst.blit_within(Rect::new(8, 8, 23, 23), 10, 9);
            t.check(
                b1.is_ok() && b2.is_err() && ra.is_ok() && rb.is_ok() && a.chunks(4).zip(b.chunks(4)).all(|(x, y)| x[1..] == y[1..]) && within.is_ok(),
                format_args!("gfx: BltBitMap a 16x16 block to a friend bitmap exactly; a blit past the destination's edge refused"),
            );
        }
        Err(e) => t.check(false, format_args!("gfx: a friend bitmap: {}", e)),
    }
    drop(bm);

    // A palette bitmap: pens, lines, text.
    let mut pb = match BitMap::new(80, 24, 8, &BitMapOptions { clear: true, ..Default::default() }) {
        Ok(b) => b,
        Err(e) => return t.check(false, format_args!("gfx: an 8-bit palette bitmap: {}", e)),
    };
    let topaz = Font::open("topaz.font", 8);
    match &topaz {
        Ok(f) => say!("info: gfx: topaz 8: y size {}, x size {}, baseline {}", f.y_size(), f.x_size(), f.baseline()),
        Err(e) => say!("info: gfx: topaz.font: {}", e),
    }
    {
        let mut rp = match pb.rastport() {
            Ok(r) => r,
            Err(_) => return,
        };
        rp.clear(5);
        rp.set_a_pen(3);
        let f = rp.fill(Rect::new(2, 2, 9, 5));
        let pens = (rp.read_pixel(0, 0), rp.read_pixel(2, 2), rp.read_pixel(9, 5), rp.read_pixel(10, 5), rp.read_pixel(79, 23));
        rp.set_a_pen(1);
        let l = rp.line(20, 1, 40, 1);
        let on = (20..=40).all(|x| rp.read_pixel(x, 1) == Ok(Some(1)));
        let off = rp.read_pixel(41, 1) == Ok(Some(5)) && rp.read_pixel(30, 0) == Ok(Some(5)) && rp.read_pixel(30, 2) == Ok(Some(5));
        t.check(
            f.is_ok() && pens == (Ok(Some(5)), Ok(Some(3)), Ok(Some(3)), Ok(Some(5)), Ok(Some(5))) && l.is_ok() && on && off,
            format_args!("gfx: palette bitmap: SetRast 5, RectFill pen 3 to its exact edges, a line in pen 1 with its neighbours untouched"),
        );
        if let Ok(font) = &topaz {
            rp.set_font(font);
            rp.clear(0);
            rp.set_a_pen(1);
            rp.set_draw_mode(0);
            let w = rp.text_width(b"Hello");
            let e = rp.text_extent(b"Hello");
            let tx = rp.text(2, 4, b"Hello");
            let mut set_in = 0;
            let mut set_out = 0;
            for y in 0..24 {
                for x in 0..80 {
                    let v = rp.read_pixel(x, y).ok().flatten().unwrap_or(0);
                    let inside = (2..42).contains(&x) && (4..12).contains(&y);
                    if v != 0 {
                        if inside { set_in += 1 } else { set_out += 1 }
                    }
                }
            }
            let long = rp.text(40, 4, b"Hello world");
            t.check(
                w == Ok(40) && e.map(|x| (x.width, x.height)) == Ok((40, 8)) && tx.is_ok() && set_in > 30 && set_out == 0 && long.is_err(),
                format_args!("gfx: topaz 8: \"Hello\" is 40 pixels; drawn, {} pixels set inside its 40x8 box, none outside; a line too long refused", set_in),
            );
        }
    }
    drop(pb);
    drop(topaz);

    // Regions.
    let reg = (|| -> Result<(i64, Rect, i64, i64, i64, bool), AmigaError> {
        let mut a = Region::new()?;
        a.or_rect(Rect::new(0, 0, 9, 9))?;
        a.or_rect(Rect::new(5, 5, 14, 14))?;
        let union_area = a.area();
        let bounds = a.bounds();
        let mut b = Region::new()?;
        b.or_rect(Rect::new(5, 5, 14, 14))?;
        let mut i = Region::new()?;
        i.or_rect(Rect::new(0, 0, 9, 9))?;
        i.and_region(&b)?;
        let inter = i.area();
        let mut x = Region::new()?;
        x.or_rect(Rect::new(0, 0, 9, 9))?;
        x.xor_region(&b)?;
        let xor = x.area();
        a.clear_rect(Rect::new(0, 0, 14, 4))?;
        let after_clear = a.area();
        x.xor_region(&b)?;
        x.xor_rect(Rect::new(0, 0, 9, 9))?;
        Ok((union_area, bounds, inter, xor, after_clear, x.is_empty()))
    })();
    t.check(
        reg == Ok((175, Rect::new(0, 0, 14, 14), 25, 150, 175 - 50, true)),
        format_args!("gfx: regions: union 175 in a 15x15 box, intersection 25, xor 150, ClearRectRegion takes 50, xor back to empty ({:?})", reg),
    );

    let m0 = avail();
    for k in 0..200u32 {
        if let Ok(mut b) = BitMap::new(32 + k % 32, 16, 32, &argb) {
            if let Ok(mut rp) = b.rastport() {
                let _ = rp.fill_color(Rect::new(0, 0, 15, 15), k);
            }
        }
        if let Ok(mut r) = Region::new() {
            let _ = r.or_rect(Rect::new(0, 0, k as i32, 3));
            let _ = r.xor_rect(Rect::new(2, 2, 5, 9));
        }
        let _ = Font::open("topaz.font", 8);
    }
    say!("info: gfx: free memory around 200 bitmap/rastport/region/font cycles: {:+} bytes", avail() as i64 - m0 as i64);
}

// ---------------------------------------------------------------------------
// intuition: windows, events, requesters
// ---------------------------------------------------------------------------

/// Events of `w` for up to `ms` milliseconds, stopping at the first that
/// `stop` accepts.
fn events_within(w: &amigaos4::intuition::Window<'_>, ms: u64, stop: impl Fn(&amigaos4::intuition::Event) -> bool) -> Vec<amigaos4::intuition::Event> {
    let t0 = amigaos4::arch::timebase();
    let mut v = Vec::new();
    while ms_since(t0) < ms {
        match w.next() {
            Some(m) => {
                let s = stop(&m.event);
                v.push(m.event);
                if s {
                    break;
                }
            }
            None => busy_ms(5),
        }
    }
    v
}

fn run_win(t: &mut Tally) {
    use amigaos4::graphics::Rect;
    use amigaos4::intuition::{idcmp, Event, PubScreen, Requester, Window, WindowOptions};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IIntuition }.is_null();
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("win: lock the default public screen: {}", e)),
    };
    let (sw, sh) = scr.size();
    say!("info: win: default public screen {}x{} \"{}\"", sw, sh, scr.title());
    t.check(null && sw >= 320 && sh >= 200, format_args!("win: the default public screen locked through the crate's own interface"));

    let all = idcmp::CLOSEWINDOW | idcmp::ACTIVEWINDOW | idcmp::INACTIVEWINDOW | idcmp::CHANGEWINDOW | idcmp::NEWSIZE;
    let mut w = match Window::open(&scr, &WindowOptions { title: "libtest window", left: 40, top: 40, inner_width: 320, inner_height: 160, idcmp: all, size_gadget: true, ..Default::default() }) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("win: open a window: {}", e)),
    };
    let (l, tp, ww, wh) = w.bounds();
    let (x0, y0, x1, y1) = w.inner();
    say!("info: win: bounds {:?}, borders {:?}, inner {:?}", (l, tp, ww, wh), w.borders(), (x0, y0, x1, y1));
    t.check((l, tp) == (40, 40) && x1 - x0 + 1 == 320 && y1 - y0 + 1 == 160, format_args!("win: opened at 40,40 with a 320x160 inside"));
    let act = events_within(&w, 2000, |e| *e == Event::Active);
    t.check(act.contains(&Event::Active), format_args!("win: WA_Activate arrives as an ActiveWindow event ({:?})", act));
    t.check(w.set_title("libtest window: retitled").is_ok(), format_args!("win: SetWindowTitles with an owned title"));

    w.change_box(60, 50, 400, 220);
    let ch = events_within(&w, 2000, |e| *e == Event::ChangeWindow);
    let nb = w.bounds();
    t.check(ch.contains(&Event::ChangeWindow) && nb == (60, 50, 400, 220), format_args!("win: ChangeWindowBox is carried out later: ChangeWindow event, then bounds {:?}", nb));

    // Drawing into the window.
    {
        let (x0, y0, x1, y1) = w.inner();
        let font = amigaos4::graphics::Font::open("topaz.font", 8).ok();
        match w.rastport() {
            Ok(mut rp) => {
                let f = rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF20_4080);
                let c = rp.read_color((x0 + x1) / 2, (y0 + y1) / 2);
                let f2 = rp.fill_color(Rect::new(x0 + 10, y0 + 10, x0 + 49, y0 + 29), 0xFFFF_FF00);
                let c2 = (rp.read_color(x0 + 10, y0 + 10), rp.read_color(x0 + 9, y0 + 10));
                if let Some(f) = &font {
                    rp.set_font(f);
                }
                rp.set_a_pen(1);
                let tx = rp.text(x0 + 60, y0 + 40, b"drawn by libtest");
                let out = rp.fill_color(Rect::new(0, 0, 400, 10), 0);
                // The screen may hold fewer bits per colour (16-bit here):
                // compare each channel within 8.
                let near = |v: &Result<u32, AmigaError>, want: u32| {
                    v.as_ref().map(|v| (0..3).all(|k| ((v >> (k * 8)) & 0xFF).abs_diff((want >> (k * 8)) & 0xFF) <= 8)).unwrap_or(false)
                };
                t.check(
                    f.is_ok() && near(&c, 0x204080) && f2.is_ok() && near(&c2.0, 0xFFFF00) && near(&c2.1, 0x204080) && tx.is_ok() && out.is_err(),
                    format_args!("win: drawing into the window's RastPort: fills read back to their edges, text, a fill past the window refused ({:?} {:x?} {:?} {:x?} {:?} {:?})", f, c, f2, c2, tx, out.is_err()),
                );
            }
            Err(e) => t.check(false, format_args!("win: window rastport: {}", e)),
        }
        // The borrowed RastPort put its own font back before `font` closes.
        drop(font);
    }

    // A second window takes the activation.
    match Window::open(&scr, &WindowOptions { title: "libtest second", left: 100, top: 120, inner_width: 120, inner_height: 40, idcmp: all, ..Default::default() }) {
        Ok(w2) => {
            let a2 = events_within(&w2, 2000, |e| *e == Event::Active);
            let i1 = events_within(&w, 1000, |e| *e == Event::Inactive);
            t.check(a2.contains(&Event::Active) && i1.contains(&Event::Inactive), format_args!("win: opening an active second window makes the first inactive"));
        }
        Err(e) => t.check(false, format_args!("win: second window: {}", e)),
    }
    drop(w);


    // A simple-refresh window: covered, uncovered, redrawn through the
    // refresh guard, which reaches only the damaged part.
    {
        use amigaos4::intuition::idcmp as ic;
        let near = |v: &Result<u32, AmigaError>, want: u32| {
            v.as_ref().map(|v| (0..3).all(|k| ((v >> (k * 8)) & 0xFF).abs_diff((want >> (k * 8)) & 0xFF) <= 8)).unwrap_or(false)
        };
        match Window::open(&scr, &WindowOptions { title: "libtest refresh", left: 300, top: 200, inner_width: 200, inner_height: 100, idcmp: ic::REFRESHWINDOW, activate: false, simple_refresh: true, ..Default::default() }) {
            Ok(mut a) => {
                let (x0, y0, x1, y1) = a.inner();
                if let Ok(mut rp) = a.rastport() {
                    let _ = rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF00_C000);
                }
                let _ = events_within(&a, 300, |_| false);
                if let Ok(b) = Window::open(&scr, &WindowOptions { title: "cover", left: 320, top: 220, inner_width: 100, inner_height: 50, idcmp: 0, activate: false, ..Default::default() }) {
                    busy_ms(100);
                    drop(b);
                }
                let ev = events_within(&a, 2000, |e| *e == Event::RefreshWindow);
                {
                    let mut g = a.refresh();
                    if let Ok(mut rp) = g.rastport() {
                        let _ = rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFFC0_0000);
                    };
                }
                let (damaged, kept) = match a.rastport() {
                    Ok(mut rp) => (rp.read_color(30, 40), rp.read_color(x1 - 5, y1 - 5)),
                    Err(e) => (Err(e), Err(AmigaError::NullPointer)),
                };
                let refreshed = ev.contains(&Event::RefreshWindow);
                if !refreshed {
                    say!("info: win: no RefreshWindow after covering a simple-refresh window (the screen keeps window contents: nothing damaged)");
                }
                // With damage, the guard's drawing reaches only it; without,
                // nothing at all.
                t.check(
                    (if refreshed { near(&damaged, 0xC00000) } else { near(&damaged, 0x00C000) }) && near(&kept, 0x00C000),
                    format_args!("win: drawing inside the refresh guard (BeginRefresh/EndRefresh) reaches only damaged parts ({:x?} {:x?}, events {:?})", damaged, kept, ev),
                );
            }
            Err(e) => t.check(false, format_args!("win: simple-refresh window: {}", e)),
        }
    }

    // A requester that does not block, never answered, freed.
    match Requester::open("libtest", "This requester closes by itself.\n100% automatic.", &["OK", "Cancel"]) {
        Ok(r) => {
            let mut none = 0;
            for _ in 0..10 {
                if r.poll().is_none() {
                    none += 1;
                }
                busy_ms(30);
            }
            t.check(none == 10, format_args!("win: a non-blocking requester polled 10 times without an answer, then freed"));
        }
        Err(e) => t.check(false, format_args!("win: BuildEasyRequestArgs: {}", e)),
    }

    let m0 = avail();
    for k in 0..100i32 {
        if let Ok(mut x) = Window::open(&scr, &WindowOptions { title: "libtest cycle", left: 10 + k, top: 30, inner_width: 100, inner_height: 50, idcmp: idcmp::CLOSEWINDOW, activate: false, ..Default::default() }) {
            let _ = x.set_title("cycling");
            if let Ok(mut rp) = x.rastport() {
                let _ = rp.fill_color(Rect::new(5, 15, 50, 30), 0xFF00_0000 | k as u32);
            }
        }
    }
    say!("info: win: free memory around 100 window open/draw/close cycles: {:+} bytes", avail() as i64 - m0 as i64);
}

/// Waits for input injected from the host: keys, a click, the close
/// gadget, then a requester button.
fn run_gui(t: &mut Tally) {
    use amigaos4::intuition::{code, easy_request, idcmp, Event, MenuEntry, MenuStrip, PubScreen, Window, WindowOptions};
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("gui: screen: {}", e)),
    };
    let strip = match MenuStrip::new(&scr, &[
        MenuEntry::Title("Project"),
        MenuEntry::Item { label: "Open...", key: Some('O'), id: 10 },
        MenuEntry::Toggle { label: "Checked", key: Some('K'), id: 20, checked: false },
        MenuEntry::Bar,
        MenuEntry::Item { label: "Quit", key: Some('Q'), id: 99 },
        MenuEntry::Title("Other"),
        MenuEntry::Item { label: "More", key: None, id: 30 },
        MenuEntry::Sub { label: "Sub one", key: None, id: 31 },
    ]) {
        Ok(m) => m,
        Err(e) => return t.check(false, format_args!("gui: menu strip: {}", e)),
    };
    let mut w = match Window::open(&scr, &WindowOptions { title: "libtest gui", left: 100, top: 100, inner_width: 300, inner_height: 150, idcmp: idcmp::CLOSEWINDOW | idcmp::VANILLAKEY | idcmp::MOUSEBUTTONS | idcmp::MENUPICK, ..Default::default() }) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("gui: open: {}", e)),
    };
    t.check(w.set_menu(&strip).is_ok(), format_args!("gui: a GadTools menu strip attached"));
    say!("gui: ready: window at {:?}, inner {:?}", w.bounds(), w.inner());
    let ev = events_within(&w, 90_000, |e| *e == Event::CloseWindow);
    for e in &ev {
        say!("info: gui: {:?}", e);
    }
    let keys: Vec<u8> = ev.iter().filter_map(|e| if let Event::Key(k) = e { Some(*k) } else { None }).collect();
    let click = ev.iter().find_map(|e| if let Event::Button { code: c, x, y } = e { (*c == code::SELECTDOWN).then_some((*x, *y)) } else { None });
    t.check(keys == b"rust", format_args!("gui: typed keys arrive as VanillaKey {:?}", core::str::from_utf8(&keys)));
    t.check(matches!(click, Some((x, y)) if (140..=170).contains(&x) && (60..=90).contains(&y)), format_args!("gui: a click inside arrives with window coordinates {:?}", click));
    t.check(ev.last() == Some(&Event::CloseWindow), format_args!("gui: the close gadget arrives as CloseWindow"));
    let menus: Vec<Event> = ev.iter().filter(|e| matches!(e, Event::Menu { .. })).copied().collect();
    t.check(
        menus == [Event::Menu { id: 10, checked: false }, Event::Menu { id: 20, checked: true }, Event::Menu { id: 20, checked: false }, Event::Menu { id: 99, checked: false }],
        format_args!("gui: Amiga-key shortcuts arrive as Menu events with the entries' ids, the toggle checked then unchecked ({:?})", menus),
    );
    drop(w);
    say!("gui: requester");
    let r = easy_request(None, "libtest", "Pick the middle button", &["Left", "Middle", "Right"]);
    t.check(r == Ok(1), format_args!("gui: EasyRequest gives the button's index from the left ({:?})", r));
}

// ---------------------------------------------------------------------------
// layers: a LayerSet on an own bitmap, a window's clip region
// ---------------------------------------------------------------------------

fn near_rgb(v: &Result<u32, AmigaError>, want: u32) -> bool {
    v.as_ref().map(|v| (0..3).all(|k| ((v >> (k * 8)) & 0xFF).abs_diff((want >> (k * 8)) & 0xFF) <= 8)).unwrap_or(false)
}

fn run_layers(t: &mut Tally) {
    use amigaos4::graphics::{BitMap, BitMapOptions, PixFmt, Rect, Region};
    use amigaos4::intuition::{idcmp, PubScreen, Window, WindowOptions};
    use amigaos4::layers::{LayerSet, Place};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::ILayers }.is_null();
    let argb = BitMapOptions { pixel_format: Some(PixFmt::A8R8G8B8), clear: true, user_private: false };
    let mut bm = match BitMap::new(200, 150, 32, &argb) {
        Ok(b) => b,
        Err(e) => return t.check(false, format_args!("layers: bitmap: {}", e)),
    };
    {
        let set = match LayerSet::new(&mut bm) {
            Ok(s) => s,
            Err(e) => return t.check(false, format_args!("layers: NewLayerInfo: {}", e)),
        };
        let (mut a, mut b) = match (set.create(Rect::new(10, 10, 109, 109), Place::Front), set.create(Rect::new(50, 50, 149, 129), Place::Front)) {
            (Ok(a), Ok(b)) => (a, b),
            (a, b) => return t.check(false, format_args!("layers: CreateLayerA: {:?} {:?}", a.err(), b.err())),
        };
        t.check(null && a.bounds() == Rect::new(10, 10, 109, 109), format_args!("layers: two layers created through the crate's own interface, bounds read back"));
        t.check(set.create(Rect::new(150, 100, 200, 120), Place::Back).is_err(), format_args!("layers: a layer past the bitmap's edge refused"));
        let fb = b.rastport().and_then(|mut rp| rp.fill_color(Rect::new(0, 0, 99, 79), 0xFF00_00C0));
        let fa = a.rastport().and_then(|mut rp| rp.fill_color(Rect::new(0, 0, 99, 99), 0xFFC0_0000));
        let (c20, c60, c120, c5) = (set.read_color(20, 20), set.read_color(60, 60), set.read_color(120, 120), set.read_color(5, 5));
        t.check(
            fa.is_ok() && fb.is_ok() && near_rgb(&c20, 0xC00000) && near_rgb(&c60, 0x0000C0) && near_rgb(&c120, 0x0000C0) && !near_rgb(&c5, 0xC00000) && !near_rgb(&c5, 0x0000C0),
            format_args!("layers: drawing a whole layer reaches only its visible part ({:x?} {:x?} {:x?} {:x?})", c20, c60, c120, c5),
        );
        say!("info: layers: outside every layer the bitmap reads {:x?} (the layer info's own backfill, not the cleared bitmap)", c5);
        t.check(b.is_top_at(60, 60) && !a.is_top_at(60, 60) && a.is_top_at(20, 20) && !a.is_top_at(5, 5) && !b.is_top_at(5, 5), format_args!("layers: WhichLayer finds the top layer at a point"));
        let up = a.to_front();
        let (c60, c120) = (set.read_color(60, 60), set.read_color(120, 120));
        t.check(up.is_ok() && near_rgb(&c60, 0xC00000) && near_rgb(&c120, 0x0000C0) && a.is_top_at(60, 60), format_args!("layers: brought to the front, a smart layer shows what was covered ({:x?} {:x?})", c60, c120));
        let mv = b.move_by(40, 10);
        let c = set.read_color(180, 130);
        t.check(mv.is_ok() && b.bounds() == Rect::new(90, 60, 189, 139) && near_rgb(&c, 0x0000C0) && b.move_by(20, 0).is_err(), format_args!("layers: moved with its contents ({:x?}); a move past the edge refused", c));
        let sz = b.size_by(-50, -40);
        t.check(sz.is_ok() && b.bounds() == Rect::new(90, 60, 139, 99) && b.size_by(0, -40).is_err(), format_args!("layers: sized smaller; a size below 1 row refused"));
        drop(b);
        drop(a);
        let m0 = avail();
        for k in 0..100 {
            if let Ok(mut l) = set.create(Rect::new(k, k / 2, k + 50, k / 2 + 30), if k % 2 == 0 { Place::Front } else { Place::Back }) {
                let _ = l.rastport().and_then(|mut rp| rp.fill_color(Rect::new(0, 0, 50, 30), 0xFF00_8000));
            }
        }
        say!("info: layers: free memory around 100 layer create/draw/delete cycles: {:+} bytes", avail() as i64 - m0 as i64);
    }

    // A window's clip region.
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("layers: screen: {}", e)),
    };
    match Window::open(&scr, &WindowOptions { title: "libtest clip", left: 200, top: 150, inner_width: 200, inner_height: 100, idcmp: idcmp::CLOSEWINDOW, activate: false, ..Default::default() }) {
        Ok(mut w) => {
            let (x0, y0, x1, y1) = w.inner();
            let _ = w.rastport().and_then(|mut rp| rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF00_C000));
            let mut region = match Region::new() {
                Ok(r) => r,
                Err(e) => return t.check(false, format_args!("layers: region: {}", e)),
            };
            let _ = region.or_rect(Rect::new(x0, y0, x0 + 49, y0 + 29));
            let filled = match w.clip(&region) {
                Ok(mut g) => g.rastport().and_then(|mut rp| rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFFC0_00C0)),
                Err(e) => Err(e),
            };
            let (inside, outside) = match w.rastport() {
                Ok(mut rp) => (rp.read_color(x0 + 10, y0 + 10), rp.read_color(x0 + 60, y0 + 10)),
                Err(e) => (Err(e), Err(AmigaError::NullPointer)),
            };
            // After the guard: the whole window again.
            let after = w.rastport().and_then(|mut rp| {
                rp.fill_color(Rect::new(x0, y0, x1, y1), 0xFF00_00C0)?;
                rp.read_color(x0 + 60, y0 + 10)
            });
            t.check(
                filled.is_ok() && near_rgb(&inside, 0xC000C0) && near_rgb(&outside, 0x00C000) && near_rgb(&after, 0x0000C0),
                format_args!("layers: a window's clip region limits drawing, and is gone after the guard ({:x?} {:x?} {:x?})", inside, outside, after),
            );
        }
        Err(e) => t.check(false, format_args!("layers: clip window: {}", e)),
    };
}

// ---------------------------------------------------------------------------
// workbench: AppWindow, AppIcon, AppMenuItem
// ---------------------------------------------------------------------------

fn run_wb(t: &mut Tally) {
    use amigaos4::intuition::{idcmp, PubScreen, Window, WindowOptions};
    use amigaos4::workbench::{AppIcon, AppMenuItem, AppPort};

    // SAFETY: reading the glue's globals (left NULL on purpose).
    let null = unsafe { sys::IWorkbench.is_null() && sys::IIcon.is_null() };
    let port = match AppPort::new() {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("wb: port: {}", e)),
    };
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("wb: screen: {}", e)),
    };
    let mut w = match Window::open(&scr, &WindowOptions { title: "libtest appwindow", left: 300, top: 300, inner_width: 200, inner_height: 80, idcmp: idcmp::CLOSEWINDOW, activate: false, ..Default::default() }) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("wb: window: {}", e)),
    };
    let aw = w.set_app_window(&port, 1);
    let ai = AppIcon::add(&port, 2, "libtest icon");
    let am = AppMenuItem::add(&port, 3, "libtest item");
    t.check(null && aw.is_ok() && ai.is_ok() && am.is_ok(), format_args!("wb: AppWindow, AppIcon and AppMenuItem added through the crate's own interfaces ({:?} {:?} {:?})", aw.as_ref().err(), ai.as_ref().err(), am.as_ref().err()));
    busy_ms(300);
    t.check(port.next().is_none(), format_args!("wb: nothing arrives on the port without the user"));
    drop((ai, am));
    w.clear_app_window();
    drop(w);

    let m0 = avail();
    for k in 0..50u32 {
        if let Ok(mut w) = Window::open(&scr, &WindowOptions { title: "libtest appwindow", left: 300, top: 300, inner_width: 100, inner_height: 40, idcmp: 0, activate: false, ..Default::default() }) {
            let _ = w.set_app_window(&port, k);
            let _i = AppIcon::add(&port, k, "libtest cycle");
            let _m = AppMenuItem::add(&port, k, "libtest cycle");
        }
    }
    say!("info: wb: free memory around 50 AppWindow/AppIcon/AppMenuItem add/remove cycles (with a window each): {:+} bytes", avail() as i64 - m0 as i64);

    // Started from a Shell: no start-up message (and no waiting for one).
    t.check(amigaos4::workbench::WbStartup::take().is_none(), format_args!("wb: WbStartup::take is None from a Shell"));

    // Tool types written into a new icon and read back.
    use amigaos4::dos::fs::{self, File, OpenMode};
    use amigaos4::workbench::{set_tool_types, tool_types};
    let f = "RAM:libtest-tt";
    let made = File::open(f, OpenMode::Create).and_then(|mut h| h.write_all(b"x"));
    let set = set_tool_types(f, &["FIT", "PUBSCREEN=Workbench", "(DONOTWAIT)"]);
    let got = tool_types(f);
    t.check(made.is_ok() && set.is_ok() && got == ["FIT", "PUBSCREEN=Workbench", "(DONOTWAIT)"], format_args!("wb: tool types written into {}.info and read back ({:?} {:?})", f, set, got));
    t.check(tool_types("RAM:libtest-no-such-file").is_empty(), format_args!("wb: no icon, no tool types"));
    let _ = fs::delete("RAM:libtest-tt.info");
    let _ = fs::delete(f);
}

/// The three asl requesters allocated and freed, without showing them.
fn run_asl(t: &mut Tally) {
    use amigaos4::asl::{FileRequester, FontRequester, ScreenModeRequester};
    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IAsl.is_null() };
    let (a, b, c) = (FileRequester::new(), FontRequester::new(), ScreenModeRequester::new());
    t.check(null && a.is_ok() && b.is_ok() && c.is_ok(), format_args!("asl: file, font and screen mode requesters allocated through the crate's own interface"));
    drop((a, b, c));
    let m0 = avail();
    for _ in 0..100 {
        let _ = FileRequester::new();
        let _ = FontRequester::new();
        let _ = ScreenModeRequester::new();
    }
    let d = avail() as i64 - m0 as i64;
    t.check(d.abs() < 16 * 1024, format_args!("asl: 100 allocate/free cycles of each keep free memory ({:+} bytes)", d));
}

/// Waits for the host at each requester (it says which): the file
/// requester's OK (Return), its Cancel (Esc), a typed save name (Return),
/// the font requester's OK, the screen mode requester's OK.
fn run_aslgui(t: &mut Tally) {
    use amigaos4::asl::{Common, FileOptions, FileRequester, FontOptions, FontRequester, ScreenModeOptions, ScreenModeRequester};
    use amigaos4::intuition::{idcmp, PubScreen, Window, WindowOptions};
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("aslgui: screen: {}", e)),
    };
    let w = match Window::open(&scr, &WindowOptions { title: "libtest asl parent", left: 40, top: 40, inner_width: 200, inner_height: 60, idcmp: idcmp::CLOSEWINDOW, ..Default::default() }) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("aslgui: window: {}", e)),
    };
    let mut fr = match FileRequester::new() {
        Ok(r) => r,
        Err(e) => return t.check(false, format_args!("aslgui: requester: {}", e)),
    };
    say!("aslgui: 1 waiting: file requester, press Return");
    let r = fr.request(Some(&w), &FileOptions { common: Common { title: Some("libtest: press Return"), ..Default::default() }, drawer: Some("RAM:"), file: Some("picked.txt"), ..Default::default() });
    t.check(matches!(&r, Ok(Some(c)) if c.paths == ["RAM:picked.txt"] && c.file == "picked.txt" && c.drawer == "RAM:"), format_args!("aslgui: OK gives the drawer, file and joined path ({:?})", r));
    say!("aslgui: 2 waiting: file requester, press Esc");
    let r = fr.request(Some(&w), &FileOptions { common: Common { title: Some("libtest: press Esc"), ..Default::default() }, ..Default::default() });
    t.check(matches!(r, Ok(None)), format_args!("aslgui: Cancel gives None ({:?})", r));
    say!("aslgui: 3 waiting: save requester, type a name and press Return");
    let r = fr.request(Some(&w), &FileOptions { common: Common { title: Some("libtest: type typed.txt, Return"), ..Default::default() }, file: Some(""), save: true, ..Default::default() });
    t.check(matches!(&r, Ok(Some(c)) if c.file == "typed.txt" && c.drawer == "RAM:" && c.paths == ["RAM:typed.txt"]), format_args!("aslgui: save mode keeps the drawer chosen last and returns the typed name ({:?})", r));
    drop(fr);
    say!("aslgui: 4 waiting: font requester, press Return");
    let r = FontRequester::new().and_then(|mut f| f.request(Some(&w), &FontOptions { common: Common { title: Some("libtest: press Return"), ..Default::default() }, size: Some(13), ..Default::default() }));
    t.check(matches!(&r, Ok(Some(f)) if f.name.ends_with(".font") && f.size > 0), format_args!("aslgui: font requester OK gives a font name and size ({:?})", r));
    say!("aslgui: 5 waiting: screen mode requester, press Return");
    let r = ScreenModeRequester::new().and_then(|mut f| f.request(None, &ScreenModeOptions { common: Common { title: Some("libtest: press Return"), ..Default::default() }, do_width: true, do_height: true, ..Default::default() }));
    t.check(matches!(&r, Ok(Some(m)) if m.display_id != 0 && m.display_id != u32::MAX && m.width > 0 && m.height > 0), format_args!("aslgui: screen mode requester OK gives a mode and size ({:?})", r));
}

// Menu ids for the menu tests.
const MN_OPEN: u32 = 101;
const MN_QUIT: u32 = 102;
const MN_FIT: u32 = 103;
const MN_RA: u32 = 104;
const MN_RB: u32 = 105;
const MN_RC: u32 = 106;
const MN_SUB1: u32 = 107;
const MN_SUB2: u32 = 108;
const MN_OFF: u32 = 109;
const MN_CTX: u32 = 110;

/// A strip with each kind of item.
fn menu_strip() -> amigaos4::menu::Strip {
    use amigaos4::menu::{Item, Menu, Strip};
    Strip::new()
        .menu(
            Menu::new("Project")
                .item(Item::action(MN_OPEN, "Open...").key("O"))
                .separator()
                .item(Item::submenu("More", vec![Item::action(MN_SUB1, "First"), Item::action(MN_SUB2, "Second")]))
                .item(Item::action(MN_OFF, "Not now").disabled(true))
                .item(Item::action(MN_QUIT, "Quit").key("Q")),
        )
        .menu(
            Menu::new("View")
                .item(Item::toggle(MN_FIT, "Fit").key("F").checked(true))
                .separator()
                .item(Item::radio(MN_RA, "Small", 1).checked(true))
                .item(Item::radio(MN_RB, "Medium", 1).key("M"))
                .item(Item::radio(MN_RC, "Large", 1)),
        )
}

fn context_strip() -> amigaos4::menu::Strip {
    use amigaos4::menu::{Item, Menu, Strip};
    Strip::new().menu(Menu::new("Context").item(Item::action(MN_CTX, "Context action")).item(Item::toggle(MN_FIT + 100, "Context toggle")))
}

/// Menus without the user: trees built, states by id, a strip and a
/// context menu on a window and on a ReAction window, a pop-up menu, cycles.
fn run_menu(t: &mut Tally) {
    use amigaos4::intuition::{idcmp, PubScreen, Window, WindowOptions};
    use amigaos4::menu::{Item, MenuTree};
    use amigaos4::popup::PopupMenu;
    let tree = match MenuTree::new(&menu_strip()) {
        Ok(m) => m,
        Err(e) => return t.check(false, format_args!("menu: tree: {}", e)),
    };
    t.check(tree.contains(MN_OPEN) && tree.contains(MN_SUB2) && tree.contains(MN_RC) && !tree.contains(999), format_args!("menu: ids found in the tree (MM_FINDID), submenus included"));
    let first = (tree.checked(MN_FIT), tree.checked(MN_RA), tree.checked(MN_RB), tree.disabled(MN_OFF), tree.disabled(MN_OPEN), tree.checked(999));
    t.check(first == (Some(true), Some(true), Some(false), Some(true), Some(false), None), format_args!("menu: starting states read back (MM_GETSTATE) {:?}", first));
    let sets = (tree.set_checked(MN_FIT, false), tree.set_disabled(MN_OFF, false), tree.set_checked(MN_RB, true), tree.set_checked(999, true));
    let second = (tree.checked(MN_FIT), tree.disabled(MN_OFF), tree.checked(MN_RB));
    t.check(sets == (true, true, true, false) && second == (Some(false), Some(false), Some(true)), format_args!("menu: states set (MM_SETSTATE) and read back {:?} {:?}", sets, second));
    t.check(tree.selected().is_empty(), format_args!("menu: nothing selected before any menu session"));
    t.check(MenuTree::new(&amigaos4::menu::Strip::new()).is_err(), format_args!("menu: an empty strip refused"));
    drop(tree);

    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("menu: screen: {}", e)),
    };
    match Window::open(&scr, &WindowOptions { title: "libtest menu", left: 80, top: 80, inner_width: 200, inner_height: 80, idcmp: idcmp::CLOSEWINDOW | idcmp::MENUPICK, activate: false, ..Default::default() }) {
        Ok(mut w) => {
            let s = w.set_menu_strip(&menu_strip());
            let c = w.set_context_menu(&context_strip());
            let st = (w.menu_checked(MN_FIT), w.set_menu_checked(MN_FIT, false), w.menu_checked(MN_FIT), w.menu_checked(MN_FIT + 100), w.set_menu_disabled(MN_QUIT, true));
            t.check(s.is_ok() && c.is_ok() && st == (Some(true), true, Some(false), Some(false), true), format_args!("menu: a menuclass strip (SetMenuStrip) and a context menu (WA_ContextMenuHook) on a window, states by id ({:?} {:?} {:?})", s, c, st));
            let again = w.set_menu_strip(&menu_strip());
            t.check(again.is_ok() && w.menu_checked(MN_FIT) == Some(true), format_args!("menu: the strip replaced by a new one ({:?})", again));
        }
        Err(e) => t.check(false, format_args!("menu: window: {}", e)),
    }

    {
        use amigaos4::reaction::{Gadget, Layout, Window as RaWindow, WindowSpec};
        let strip = menu_strip();
        let root = Layout::vertical().child(Gadget::button(1, "Right-click me").context_menu(context_strip())).child(Gadget::checkbox(2, "Plain"));
        match RaWindow::open(&scr, &WindowSpec { title: "libtest ra menu", position: Some((120, 120)), activate: false, menu: Some(&strip), ..Default::default() }, root) {
            Ok(w) => {
                let st = (w.menu_checked(MN_RA), w.set_menu_checked(MN_RC, true), w.menu_checked(MN_RC), w.menu_checked(MN_FIT + 100), w.set_menu_disabled(MN_OPEN, true));
                t.check(st == (Some(true), true, Some(true), Some(false), true), format_args!("menu: a ReAction window with WINDOW_MenuStrip and a gadget context menu, states by id {:?}", st));
            }
            Err(e) => t.check(false, format_args!("menu: ReAction window: {}", e)),
        }
    }

    let pop = PopupMenu::new(&[Item::action(1, "One"), Item::separator(), Item::toggle(2, "Two").checked(true), Item::submenu("Three", vec![Item::action(3, "Three a")])]);
    t.check(pop.is_ok(), format_args!("menu: a pop-up menu built (popupmenu.class) ({:?})", pop.as_ref().err()));
    drop(pop);

    let m0 = avail();
    let mut ok = 0;
    for _ in 0..50 {
        if MenuTree::new(&menu_strip()).is_ok() && PopupMenu::new(&[Item::action(1, "One"), Item::submenu("Sub", vec![Item::action(2, "Two")])]).is_ok() {
            ok += 1;
        }
    }
    let d = avail() as i64 - m0 as i64;
    t.check(ok == 50 && d.abs() < 32 * 1024, format_args!("menu: 50 tree and pop-up build/drop cycles keep free memory ({} ok, {:+} bytes)", ok, d));
}

/// Menus with the host's input: Amiga-key shortcuts on a window's
/// menuclass strip (Amiga-O, Amiga-F, Amiga-M), then a pop-up menu opened
/// for a click on its second item.
fn run_menugui(t: &mut Tally) {
    use amigaos4::intuition::{idcmp, Event, PubScreen, Window, WindowOptions};
    use amigaos4::menu::Item;
    use amigaos4::popup::PopupMenu;
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("menugui: screen: {}", e)),
    };
    let mut w = match Window::open(&scr, &WindowOptions { title: "libtest menugui", left: 100, top: 100, inner_width: 240, inner_height: 100, idcmp: idcmp::CLOSEWINDOW | idcmp::MENUPICK, ..Default::default() }) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("menugui: window: {}", e)),
    };
    if let Err(e) = w.set_menu_strip(&menu_strip()) {
        return t.check(false, format_args!("menugui: strip: {}", e));
    }
    say!("menugui: waiting for Amiga-O, Amiga-F, Amiga-M");
    let t0 = amigaos4::arch::timebase();
    let mut got: Vec<(u32, bool)> = Vec::new();
    while got.len() < 3 && ms_since(t0) < 120_000 {
        match w.next() {
            Some(m) => {
                say!("info: menugui: {:?}", m.event);
                if let Event::Menu { id, checked } = m.event {
                    got.push((id, checked));
                }
            }
            None => busy_ms(10),
        }
    }
    t.check(got.first() == Some(&(MN_OPEN, false)), format_args!("menugui: Amiga-O is a menu event with Open's id ({:?})", got.first()));
    t.check(got.get(1) == Some(&(MN_FIT, false)) && w.menu_checked(MN_FIT) == Some(false), format_args!("menugui: Amiga-F turns the Fit toggle off, as the event and the tree say ({:?})", got.get(1)));
    t.check(got.get(2) == Some(&(MN_RB, true)) && w.menu_checked(MN_RA) == Some(false), format_args!("menugui: Amiga-M checks Medium and clears Small (MA_MX) ({:?}, Small {:?})", got.get(2), w.menu_checked(MN_RA)));

    let pop = match PopupMenu::new(&[Item::action(201, "Alpha"), Item::action(202, "Beta"), Item::action(203, "Gamma")]) {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("menugui: pop-up: {}", e)),
    };
    say!("menugui: pop-up open: click Beta");
    let r = pop.open(w.as_ptr());
    t.check(r == Some(202), format_args!("menugui: the pop-up's PM_OPEN answers the clicked item's id ({:?})", r));
}

/// The ReAction test window: one gadget of each kind.
fn ra_layout() -> amigaos4::reaction::Layout {
    use amigaos4::reaction::{Gadget, Layout};
    Layout::vertical()
        .child(Gadget::string(1).text("start").max_chars(40).label("Name"))
        .child(Gadget::checkbox(2, "Check me").checked(true))
        .child(Gadget::integer(3).range(0, 50).value(7).label("Count"))
        .child(Gadget::slider(4).range(0, 10).value(3).label("Level"))
        .child(Gadget::chooser(5, &["One", "Two", "Three"]).select(1).label("Pick"))
        .child(Gadget::list(6, &["alpha", "beta", "gamma"]).select(2).min_size(160, 60))
        .child(Gadget::gauge(7).value(40))
        .child(Layout::horizontal().label("Buttons").even().child(Gadget::button(8, "OK")).child(Gadget::button(9, "Push").push()))
}

/// ReAction without the user: a window with each gadget kind, values read
/// back, set and read again, wrong ids and kinds refused, open/close
/// cycles.
fn run_ra(t: &mut Tally) {
    use amigaos4::intuition::PubScreen;
    use amigaos4::reaction::{GadgetKind, Window, WindowSpec};
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("ra: screen: {}", e)),
    };
    let spec = WindowSpec { title: "libtest reaction", position: Some((60, 60)), activate: false, ..Default::default() };
    let w = match Window::open(&scr, &spec, ra_layout()) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("ra: window: {}", e)),
    };
    t.check(!w.as_ptr().is_null() && w.signal_mask() != 0, format_args!("ra: window open through window.class, with a signal ({:#x})", w.signal_mask()));
    t.check(w.kind(6) == Some(GadgetKind::List) && w.kind(8) == Some(GadgetKind::Button) && w.kind(99).is_none(), format_args!("ra: gadget kinds by id"));
    let first = (w.text(1), w.checked(2), w.number(3), w.level(4), w.active(5), w.selected(6), w.level(7), w.checked(9));
    say!("info: ra: first values {:?}", first);
    t.check(
        matches!(&first.0, Ok(s) if s == "start")
            && matches!(first.1, Ok(true))
            && matches!(first.2, Ok(7))
            && matches!(first.3, Ok(3))
            && matches!(first.4, Ok(1))
            && matches!(first.5, Ok(Some(2)))
            && matches!(first.6, Ok(40))
            && matches!(first.7, Ok(false)),
        format_args!("ra: starting values read back (text, check, integer, slider, chooser, list, gauge, push button)"),
    );
    let sets = [w.set_text(1, "changed"), w.set_checked(2, false), w.set_number(3, 42), w.set_level(4, 9), w.set_active(5, 2), w.set_selected(6, Some(0)), w.set_level(7, 75), w.set_checked(9, true)];
    let second = (w.text(1), w.checked(2), w.number(3), w.level(4), w.active(5), w.selected(6), w.level(7), w.checked(9));
    say!("info: ra: set values {:?}", second);
    t.check(
        sets.iter().all(|r| r.is_ok())
            && matches!(&second.0, Ok(s) if s == "changed")
            && matches!(second.1, Ok(false))
            && matches!(second.2, Ok(42))
            && matches!(second.3, Ok(9))
            && matches!(second.4, Ok(2))
            && matches!(second.5, Ok(Some(0)))
            && matches!(second.6, Ok(75))
            && matches!(second.7, Ok(true)),
        format_args!("ra: values set and read back"),
    );
    let _ = w.set_number(3, 500);
    t.check(matches!(w.number(3), Ok(50)), format_args!("ra: an integer set above its range is held at the maximum ({:?})", w.number(3)));
    let d = (w.set_disabled(8, true), w.disabled(8), w.set_disabled(8, false), w.disabled(8));
    t.check(matches!(d, (Ok(()), Ok(true), Ok(()), Ok(false))), format_args!("ra: a button disabled and enabled again ({:?})", d));
    let mut w = w;
    let items = w.set_items(6, &["x", "y", "z", "w"]);
    let after = w.selected(6);
    let sel = w.set_selected(6, Some(3));
    t.check(items.is_ok() && matches!(after, Ok(None)) && sel.is_ok() && matches!(w.selected(6), Ok(Some(3))), format_args!("ra: list rows replaced, nothing selected, then row 3 ({:?} {:?})", items, after));
    t.check(w.number(1).is_err() && w.text(3).is_err() && w.text(99).is_err(), format_args!("ra: wrong kinds and unknown ids refused"));
    let mut ev = 0;
    while let Some(e) = w.next() {
        say!("info: ra: event {:?}", e);
        ev += 1;
    }
    t.check(ev < 10, format_args!("ra: no gadget events without the user ({} other)", ev));
    drop(w);

    let m0 = avail();
    let mut ok = 0;
    for _ in 0..20 {
        if let Ok(mut w) = Window::open(&scr, &spec, ra_layout()) {
            if w.set_items(6, &["a", "b"]).is_ok() {
                ok += 1;
            }
        }
    }
    let delta = avail() as i64 - m0 as i64;
    t.check(ok == 20 && delta.abs() < 64 * 1024, format_args!("ra: 20 open/close cycles with a list replaced keep free memory ({} ok, {:+} bytes)", ok, delta));
}

/// ReAction with the user (or the host's input injection): waits for the
/// check box clicked, text typed into the string and Return, a list row
/// clicked, then Done.
fn run_ragui(t: &mut Tally) {
    use amigaos4::intuition::PubScreen;
    use amigaos4::reaction::{Event, Gadget, Layout, Window, WindowSpec};
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("ragui: screen: {}", e)),
    };
    let root = Layout::vertical()
        .child(Gadget::string(1).label("Type"))
        .child(Gadget::checkbox(2, "Tick me"))
        .child(Gadget::list(4, &["first", "second", "third"]).min_size(200, 70))
        .child(Gadget::button(3, "Done"));
    let w = match Window::open(&scr, &WindowSpec { title: "libtest ragui", position: Some((100, 100)), ..Default::default() }, root) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("ragui: window: {}", e)),
    };
    say!("ragui: waiting at {:?}", unsafe_bounds(w.as_ptr()));
    let t0 = amigaos4::arch::timebase();
    let mut got: Vec<Event> = Vec::new();
    let mut done = false;
    while !done && ms_since(t0) < 180_000 {
        let mut idle = true;
        while let Some(e) = w.next() {
            say!("info: ragui: {:?}", e);
            idle = false;
            done |= matches!(e, Event::Gadget { id: 3, .. } | Event::Close);
            got.push(e);
        }
        if idle {
            busy_ms(10);
        }
    }
    t.check(got.iter().any(|e| matches!(e, Event::Gadget { id: 2, .. })) && matches!(w.checked(2), Ok(true)), format_args!("ragui: the check box clicked is a gadget event and reads ticked"));
    let text = w.text(1);
    t.check(got.iter().any(|e| matches!(e, Event::Gadget { id: 1, .. })) && matches!(&text, Ok(s) if s == "rust"), format_args!("ragui: text typed and Return is a gadget event; the text reads back ({:?})", text));
    let row = got.iter().find_map(|e| match e {
        Event::Gadget { id: 4, code } => Some(*code),
        _ => None,
    });
    t.check(row == Some(1) && matches!(w.selected(4), Ok(Some(1))), format_args!("ragui: the list row clicked arrives as the event's code and reads selected ({:?})", row));
    t.check(done, format_args!("ragui: Done clicked"));
}

/// A window's left, top, width and height.
fn unsafe_bounds(win: *mut sys::Window) -> (i16, i16, i16, i16) {
    if win.is_null() {
        return (0, 0, 0, 0);
    }
    // SAFETY: an open window; LeftEdge, TopEdge, Width and Height are the
    // four WORDs at offsets 4, 6, 8 and 10 (tests/layout/intuition.c).
    unsafe {
        let p = win as *const u8;
        (
            core::ptr::read_unaligned(p.add(4) as *const i16),
            core::ptr::read_unaligned(p.add(6) as *const i16),
            core::ptr::read_unaligned(p.add(8) as *const i16),
            core::ptr::read_unaligned(p.add(10) as *const i16),
        )
    }
}

/// Writes imgview.info beside the test's copy of imgview (a tool icon
/// with tool types) for starting imgview from Workbench.
fn run_imgicon(t: &mut Tally) {
    let r = amigaos4::workbench::set_tool_types("SHARED:imgview/imgview", &["FIT", "PUBSCREEN=Workbench"]);
    t.check(r.is_ok(), format_args!("imgicon: SHARED:imgview/imgview.info written ({:?})", r));
}

/// Talks to a running imgview through application.library: OpenDoc of a
/// test picture, ToFront, then Quit, and checks it has gone.
fn run_appimg(t: &mut Tally) {
    use amigaos4::application::{self as ap, Application, RegisterOptions, Request};
    use amigaos4::timer::{AmigaTimer, TimerUnit, TimerVal};
    let mut tm = match AmigaTimer::open(TimerUnit::MicroHz) {
        Ok(t) => t,
        Err(e) => return t.check(false, format_args!("appimg: timer: {}", e)),
    };
    let me = match Application::register("libtest-appimg", &RegisterOptions::default()) {
        Ok(a) => a,
        Err(e) => return t.check(false, format_args!("appimg: register: {}", e)),
    };
    let target = ap::find("imgview");
    t.check(matches!(target, Ok(Some(_))), format_args!("appimg: imgview found by name ({:?})", target.as_ref().map(|o| o.is_some())));
    let Ok(Some(id)) = target else { return };
    let doc = "SHARED:imgview/pics/d_mid.bmp";
    let r = me.send(id, Request::OpenDoc(doc));
    t.check(r.is_ok(), format_args!("appimg: OpenDoc {} sent ({:?})", doc, r));
    say!("appimg: sent OpenDoc; 8 s for a screenshot");
    let _ = tm.delay(TimerVal::new(8, 0));
    let r = me.send(id, Request::ToFront);
    t.check(r.is_ok(), format_args!("appimg: ToFront sent ({:?})", r));
    let _ = tm.delay(TimerVal::new(2, 0));
    let r = me.send(id, Request::Quit);
    t.check(r.is_ok(), format_args!("appimg: Quit sent ({:?})", r));
    let mut gone = false;
    for _ in 0..50 {
        let _ = tm.delay(TimerVal::new(0, 200_000));
        if matches!(ap::find("imgview"), Ok(None)) {
            gone = true;
            break;
        }
    }
    t.check(gone, format_args!("appimg: imgview unregistered within 10 s of Quit"));
}

/// Waits for the host: a disk icon dropped on the window, the AppIcon
/// double-clicked, a disk icon dropped on the AppIcon (the Tools menu item
/// too, if someone chooses it), then the window closed.
fn run_wbgui(t: &mut Tally) {
    use amigaos4::intuition::{idcmp, Event, PubScreen, Window, WindowOptions};
    use amigaos4::workbench::{AppEvent, AppIcon, AppKind, AppMenuItem, AppPort};
    let port = match AppPort::new() {
        Ok(p) => p,
        Err(e) => return t.check(false, format_args!("wbgui: port: {}", e)),
    };
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return t.check(false, format_args!("wbgui: screen: {}", e)),
    };
    let mut w = match Window::open(&scr, &WindowOptions { title: "libtest drop here", left: 400, top: 250, inner_width: 300, inner_height: 150, idcmp: idcmp::CLOSEWINDOW, ..Default::default() }) {
        Ok(w) => w,
        Err(e) => return t.check(false, format_args!("wbgui: window: {}", e)),
    };
    let (aw, ai, am) = (w.set_app_window(&port, 1), AppIcon::add(&port, 2, "libtest icon"), AppMenuItem::add(&port, 3, "libtest item"));
    if aw.is_err() || ai.is_err() || am.is_err() {
        return t.check(false, format_args!("wbgui: adding the objects failed"));
    }
    say!("wbgui: ready: window at {:?}", w.bounds());
    let t0 = amigaos4::arch::timebase();
    let mut got: Vec<AppEvent> = Vec::new();
    let mut closed = false;
    while ms_since(t0) < 180_000 && !closed {
        let mut idle = true;
        while let Some(e) = port.next() {
            say!("info: wbgui: {:?}", e);
            got.push(e);
            idle = false;
        }
        if let Some(m) = w.next() {
            closed = m.event == Event::CloseWindow;
            idle = false;
        }
        if idle {
            busy_ms(10);
        }
    }
    let win = got.iter().find(|e| e.kind == AppKind::Window);
    t.check(matches!(win, Some(e) if e.id == 1 && e.files.len() == 1 && e.files[0].ends_with(':') && e.x > 0 && e.y > 0), format_args!("wbgui: a disk icon dropped on the AppWindow arrives with its id, the volume as a path and the drop point ({:?})", win));
    let open = got.iter().find(|e| e.kind == AppKind::Icon && e.files.is_empty());
    t.check(matches!(open, Some(e) if e.id == 2 && e.class == 0), format_args!("wbgui: the AppIcon double-clicked arrives as opened, no files ({:?})", open));
    let drop_on = got.iter().find(|e| e.kind == AppKind::Icon && !e.files.is_empty());
    t.check(matches!(drop_on, Some(e) if e.id == 2 && e.files[0].ends_with(':')), format_args!("wbgui: a disk icon dropped on the AppIcon arrives with the volume ({:?})", drop_on));
    // Input injection cannot hold the right button while a screenshot finds
    // the Tools menu, so choosing the AppMenuItem is not injected: reported.
    let menu = got.iter().find(|e| e.kind == AppKind::MenuItem);
    say!("info: wbgui: Tools menu item chosen: {:?}", menu);
    t.check(closed, format_args!("wbgui: closed by the user"));
}

// ---------------------------------------------------------------------------
// datatypes: examine, objects, pixels
// ---------------------------------------------------------------------------

/// The test picture: 4x3, top row first, as 0xRRGGBB.
const DT_PIXELS: [[u32; 4]; 3] = [
    [0xFF0000, 0x00FF00, 0x0000FF, 0xFFFFFF],
    [0x000000, 0x804020, 0x102030, 0xFFFF00],
    [0x555555, 0x00FFFF, 0xFF00FF, 0x010203],
];

/// A 24-bit BMP of DT_PIXELS.
fn dt_bmp() -> Vec<u8> {
    let (w, h) = (4u32, 3u32);
    let row = ((w * 3 + 3) & !3) as usize;
    let size = 54 + row * h as usize;
    let mut v = Vec::with_capacity(size);
    v.extend_from_slice(b"BM");
    v.extend_from_slice(&(size as u32).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&54u32.to_le_bytes());
    for x in [40u32, w, h] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    for x in [0u32, (row * h as usize) as u32, 2835, 2835, 0, 0] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    for y in (0..h as usize).rev() {
        let start = v.len();
        for c in DT_PIXELS[y] {
            v.extend_from_slice(&[c as u8, (c >> 8) as u8, (c >> 16) as u8]);
        }
        while v.len() - start < row {
            v.push(0);
        }
    }
    v
}

fn run_dt(t: &mut Tally) {
    use amigaos4::datatypes::{self as dt, group, DtObject, DtOptions, PixelFormat};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IDataTypes }.is_null();
    let all = dt::list_all();
    let all2 = dt::list_all();
    match &all {
        Ok(v) => {
            let names: Vec<String> = v.iter().map(|d| alloc::format!("{}({})", d.name, d.group_str())).collect();
            say!("info: dt: {} datatypes: {}", v.len(), names.join(" "));
        }
        Err(e) => say!("info: dt: list_all: {}", e),
    }
    t.check(null && matches!((&all, &all2), (Ok(a), Ok(b)) if a.len() >= 3 && a == b), format_args!("dt: every installed datatype listed through the crate's own interface, twice the same"));
    let has = |n: &str| all.as_ref().map(|v| v.iter().any(|d| d.name.eq_ignore_ascii_case(n) || d.base_name.eq_ignore_ascii_case(n))).unwrap_or(false);

    let s = dt::examine("S:Startup-Sequence");
    t.check(matches!(&s, Ok(i) if i.group == group::TEXT), format_args!("dt: S:Startup-Sequence is text ({:?})", s));
    let missing = dt::examine("RAM:no-such-file-libtest");
    t.check(missing == Err(AmigaError::DosError(205)), format_args!("dt: examining a missing file is object-not-found ({:?})", missing));
    let e = dt::dt_string(dt::DTERROR_UNKNOWN_DATATYPE as u32);
    say!("info: dt: GetDTString(2000) = {:?}", e);
    t.check(matches!(&e, Some(s) if !s.is_empty()), format_args!("dt: GetDTString has text for DTERROR_UNKNOWN_DATATYPE"));

    // The text object's contents are the file's.
    let file = amigaos4::dos::fs::File::read_to_vec("S:Startup-Sequence");
    match DtObject::from_file("S:Startup-Sequence", &DtOptions::default()) {
        Ok(o) => {
            let txt = o.text();
            let ty = o.data_type();
            t.check(
                matches!((&txt, &file), (Some(a), Ok(b)) if !a.is_empty() && b.starts_with(&a[..a.len().min(64)])) && matches!(&ty, Some(i) if i.group == group::TEXT),
                format_args!("dt: a text object's buffer is the file's contents, its datatype text ({:?})", ty.map(|i| i.name)),
            );
            say!("info: dt: text object methods: {:x?}", o.methods());
        }
        Err(e) => t.check(false, format_args!("dt: text object: {}", e)),
    }
    let wrong = DtObject::from_file("S:Startup-Sequence", &DtOptions { group: Some(group::PICTURE) });
    t.check(matches!(wrong, Err(AmigaError::DosError(212))), format_args!("dt: asking for a picture from a text file is object-wrong-type ({:?})", wrong.as_ref().err()));

    // A picture made here.
    let bmp = dt_bmp();
    let wrote = amigaos4::dos::fs::File::open("RAM:libtest_dt.bmp", amigaos4::dos::fs::OpenMode::Create).and_then(|mut f| f.write_all(&bmp));
    if !has("bmp") {
        say!("info: dt: no BMP datatype installed: picture checks skipped");
        let _ = amigaos4::dos::fs::delete("RAM:libtest_dt.bmp");
        return;
    }
    let ex = dt::examine("RAM:libtest_dt.bmp");
    let exm = dt::examine_memory(&bmp);
    t.check(wrote.is_ok() && matches!(&ex, Ok(i) if i.group == group::PICTURE) && ex == exm, format_args!("dt: a BMP written here is a picture, from the file and from memory ({:?})", ex.as_ref().map(|i| &i.name)));
    let check_pixels = |o: &DtObject<'_>| -> (Option<amigaos4::datatypes::BitMapHeader>, Result<bool, AmigaError>) {
        let h = o.bitmap_header();
        let mut buf = alloc::vec![0u8; 4 * 4 * 3];
        let r = o.read_pixels(PixelFormat::Argb, &mut buf, 16).map(|_| {
            (0..3).all(|y| (0..4).all(|x| {
                let p = &buf[y * 16 + x * 4..][..4];
                (u32::from(p[1]) << 16 | u32::from(p[2]) << 8 | u32::from(p[3])) == DT_PIXELS[y][x]
            }))
        });
        (h, r)
    };
    match DtObject::from_file("RAM:libtest_dt.bmp", &DtOptions { group: Some(group::PICTURE) }) {
        Ok(o) => {
            let (h, px) = check_pixels(&o);
            say!("info: dt: picture header {:?}, methods {:x?}", h, o.methods());
            t.check(matches!(h, Some(h) if h.width == 4 && h.height == 3), format_args!("dt: the picture's BitMapHeader is 4x3"));
            t.check(px == Ok(true), format_args!("dt: PDTM_READPIXELARRAY gives every pixel of the file as ARGB ({:?})", px));
            let mut small = [0u8; 16];
            t.check(o.read_pixels(PixelFormat::Argb, &mut small, 16).is_err() && o.read_pixels(PixelFormat::Rgb, &mut [0u8; 64], 8).is_err(), format_args!("dt: a pixel buffer or row too small is refused"));
        }
        Err(e) => t.check(false, format_args!("dt: picture object: {}", e)),
    }
    match DtObject::from_memory(&bmp, &DtOptions::default()) {
        Ok(o) => {
            let (h, px) = check_pixels(&o);
            t.check(matches!(h, Some(h) if h.width == 4) && px == Ok(true), format_args!("dt: the same picture from borrowed bytes ({:?})", px));
        }
        Err(e) => t.check(false, format_args!("dt: picture from memory: {}", e)),
    }
    let m0 = avail();
    for _ in 0..100 {
        if let Ok(o) = DtObject::from_memory(&bmp, &DtOptions::default()) {
            let _ = o.bitmap_header();
        }
    }
    say!("info: dt: free memory around 100 picture objects from memory: {:+} bytes", avail() as i64 - m0 as i64);
    let _ = amigaos4::dos::fs::delete("RAM:libtest_dt.bmp");

    // A picture made from pixels, saved as IFF, read back.
    const W: usize = 37;
    const H: usize = 23;
    let mut argb = alloc::vec![0u8; W * H * 4];
    for y in 0..H {
        for x in 0..W {
            let p = &mut argb[(y * W + x) * 4..][..4];
            p.copy_from_slice(&[0xFF, (x * 7) as u8, (y * 11) as u8, ((x ^ y) * 5) as u8]);
        }
    }
    const SAVED: &str = "RAM:libtest_dt.iff";
    let made = DtObject::new_picture(W as u16, H as u16, PixelFormat::Argb, &argb, W * 4);
    let saved = made.as_ref().map_err(|e| *e).and_then(|o| o.save_iff(SAVED));
    let size = amigaos4::dos::fs::metadata(SAVED).map(|m| m.size);
    let back = DtObject::from_file(SAVED, &DtOptions { group: Some(group::PICTURE) }).and_then(|o| {
        let h = o.bitmap_header();
        let mut px = alloc::vec![0u8; W * H * 4];
        o.read_pixels(PixelFormat::Argb, &mut px, W * 4)?;
        Ok((h, o.data_type().map(|d| d.name), px))
    });
    let same = back.as_ref().map(|(_, _, px)| {
        px.chunks(4).zip(argb.chunks(4)).filter(|(a, b)| a[1..] != b[1..]).count()
    });
    say!(
        "info: dt: new picture {}, save_iff {:?} ({:?} bytes), read back as {:?} {:?}, pixels differing {:?}",
        made.is_ok(),
        saved,
        size,
        back.as_ref().ok().map(|b| b.1.clone()),
        back.as_ref().ok().map(|b| b.0.map(|h| (h.width, h.height, h.depth))),
        same
    );
    t.check(
        saved.is_ok()
            && matches!(back, Ok((Some(h), _, _)) if h.width as usize == W && h.height as usize == H && h.depth == 24)
            && same == Ok(0),
        format_args!("dt: a 24-bit picture made from ARGB pixels, saved as IFF and read back the same"),
    );
    drop(made);
    t.check(DtObject::new_picture(4, 4, PixelFormat::Argb, &argb[..16], 16).is_err(), format_args!("dt: new_picture with too few pixels is refused"));
    let _ = amigaos4::dos::fs::delete(SAVED);
}

// ---------------------------------------------------------------------------
// console: RawKeyConvert, a console window
// ---------------------------------------------------------------------------

fn run_con(t: &mut Tally) {
    use amigaos4::console::{raw_key_convert, Console, Unit, WindowSpec};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IConsole }.is_null();
    let a = raw_key_convert(0x20, 0);
    let sa = raw_key_convert(0x20, 1);
    let ret = raw_key_convert(0x44, 0);
    let up = raw_key_convert(0x4C, 0);
    let rel = raw_key_convert(0x20 | 0x80, 0);
    say!("info: con: RawKeyConvert a {:x?} shift-a {:x?} return {:x?} up {:x?} a-released {:x?}", a, sa, ret, up, rel);
    t.check(null && a.as_deref() == Ok(b"a") && sa.as_deref() == Ok(b"A") && ret.as_deref() == Ok(b"\r") && up.as_deref() == Ok(b"\x9bA") && rel.as_deref() == Ok(b""), format_args!("con: RawKeyConvert through console.device opened as a library: a, A, Return, cursor up, nothing for a release"));

    let spec = WindowSpec { title: "libtest console", left: 60, top: 60, width: 480, height: 200, activate: false };
    let mut c = match Console::open(Unit::CharMap, &spec) {
        Ok(c) => c,
        Err(e) => return t.check(false, format_args!("con: open a console window: {}", e)),
    };
    let sz = c.size();
    let w = c.write(b"hello");
    let cur = c.cursor();
    say!("info: con: size {:?}, cursor after \"hello\" {:?}", sz, cur);
    t.check(!c.window_ptr().is_null() && matches!(sz, Ok((r, k)) if r > 3 && k > 20) && w == Ok(5) && cur == Ok((5, 0)), format_args!("con: a console window: its size in characters, \"hello\" written, the cursor after it"));
    let _ = c.write(b"\n\x9b10Cx");
    let cur2 = c.cursor();
    t.check(cur2 == Ok((11, 1)), format_args!("con: CSI sequences move the cursor ({:?})", cur2));
    // A report request answered through CMD_READ: CSI 0 q (window status)
    // gives CSI 1;1;rows;columns SPACE r.
    let _ = c.clear_input();
    let report = |c: &mut Console, seq: &[u8], last: u8| -> Vec<u8> {
        let _ = c.write(seq);
        let mut buf = [0u8; 64];
        let mut got = Vec::new();
        let t0 = amigaos4::arch::timebase();
        while ms_since(t0) < 2000 && got.last() != Some(&last) {
            match c.read(&mut buf, 500) {
                Ok(n) => got.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        got
    };
    let wsr = report(&mut c, b"\x9b0 q", b'r');
    let want: Option<Vec<u8>> = sz.ok().map(|(r, k)| {
        let mut v = alloc::vec![0x9Bu8];
        v.extend_from_slice(alloc::format!("1;1;{};{} r", r, k).as_bytes());
        v
    });
    say!("info: con: window status report {:x?}", wsr);
    t.check(want.as_deref() == Some(&wsr[..]), format_args!("con: CSI 0 q is answered through CMD_READ with the size CD_GETATTRS gives"));
    // CSI 6n (cursor position report) answers with numbers that are not the
    // cursor on console.device 54; CSI 0 q and CD_GETATTRS are right.
    let dsr = report(&mut c, b"\x9b6n", b'R');
    say!("info: con: CSI 6n answered {:?} (the cursor is {:?})", core::str::from_utf8(&dsr[dsr.len().min(1)..]), c.cursor());
    let mut buf = [0u8; 64];

    let t1 = amigaos4::arch::timebase();
    let quiet = c.read(&mut buf, 300);
    let took = ms_since(t1);
    t.check(quiet == Ok(0) && (280..2000).contains(&took), format_args!("con: a read with no input ends after its time limit with nothing ({:?} in {} ms), aborted", quiet, took));
    t.check(c.clear_input().is_ok(), format_args!("con: CMD_CLEAR"));
    drop(c);

    // A console on a window of our own.
    {
        use amigaos4::console::WindowConsole;
        use amigaos4::intuition::{PubScreen, Window, WindowOptions};
        let scr = PubScreen::default_screen();
        match scr.as_ref().map_err(|e| *e).and_then(|scr| {
            Window::open(scr, &WindowOptions { title: "libtest own console", left: 80, top: 80, inner_width: 400, inner_height: 150, activate: false, ..Default::default() })
        }) {
            Ok(w) => {
                let r = WindowConsole::open(Unit::CharMap, &w).and_then(|mut c| {
                    let sz = c.size()?;
                    let n = c.write(b"in our window")?;
                    let cur = c.cursor()?;
                    Ok((c.window_ptr() == w.as_ptr(), sz, n, cur))
                });
                say!("info: con: own window console {:?}", r);
                // The window is still ours and open after the console.
                let still = w.bounds();
                t.check(
                    matches!(r, Ok((true, (rows, cols), 13, (13, 0))) if rows >= 5 && cols >= 30) && still.2 > 0,
                    format_args!("con: a console on our own window: its size from our window, text written, the window left open"),
                );
            }
            Err(e) => t.check(false, format_args!("con: open our own window: {}", e)),
        };
    }

    let m0 = avail();
    for k in 0..30 {
        if let Ok(mut c) = Console::open(Unit::CharMap, &WindowSpec { title: "libtest cycle", left: 20 + k, top: 40, width: 300, height: 100, activate: false }) {
            let _ = c.write(b"cycle\n");
            let _ = c.size();
        }
    }
    for _ in 0..1000 {
        let _ = raw_key_convert(0x21, 0);
    }
    say!("info: con: free memory around 30 console windows and 1000 RawKeyConvert calls: {:+} bytes", avail() as i64 - m0 as i64);
}

/// Waits for input typed by the host into an active console window.
fn run_congui(t: &mut Tally) {
    use amigaos4::console::{Console, Unit, WindowSpec};
    let mut c = match Console::open(Unit::CharMap, &WindowSpec { title: "libtest console input", left: 100, top: 100, width: 480, height: 160, activate: true }) {
        Ok(c) => c,
        Err(e) => return t.check(false, format_args!("congui: open: {}", e)),
    };
    let _ = c.write(b"Type rust and press Return\n");
    say!("congui: ready");
    let mut buf = [0u8; 32];
    let mut got = Vec::new();
    let t0 = amigaos4::arch::timebase();
    while ms_since(t0) < 90_000 && !got.contains(&b'\r') {
        match c.read(&mut buf, 1000) {
            Ok(n) => got.extend_from_slice(&buf[..n]),
            Err(e) => {
                say!("info: congui: read: {}", e);
                break;
            }
        }
    }
    say!("info: congui: read {:x?}", got);
    t.check(got == b"rust\r", format_args!("congui: typed keys read from the console as bytes, Return as CR"));
    let _ = c.write(b"\nthanks\n");
}

// ---------------------------------------------------------------------------
// elf: the libtest binary itself
// ---------------------------------------------------------------------------

fn run_elf(t: &mut Tally) {
    use amigaos4::elf::ElfFile;

    const SELF: &str = "SHARED:libtest/libtest";
    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IElf }.is_null();
    let f = match ElfFile::open(SELF) {
        Ok(f) => f,
        Err(e) => return t.check(false, format_args!("elf: open {}: {}", SELF, e)),
    };
    let h = f.header();
    say!("info: elf: header {:?}", h);
    t.check(null && matches!(h, Ok(h) if h.class == 1 && h.data == 2 && h.machine == 20 && h.section_count as u32 == f.section_count()), format_args!("elf: the libtest binary opened through the crate's own interface: 32-bit big-endian PowerPC"));
    let secs = f.sections();
    let names: Vec<&str> = secs.iter().map(|s| s.name.as_str()).collect();
    say!("info: elf: {} sections: {}", secs.len(), names.join(" "));
    t.check(names.contains(&".text") && names.contains(&".symtab") && names.contains(&".strtab"), format_args!("elf: sections with their names (.text, .symtab, .strtab)"));

    // A section's bytes are the file's bytes at its offset.
    let file = amigaos4::dos::fs::File::read_to_vec(SELF);
    let pick = secs.iter().find(|s| s.name == ".rodata" || s.name == ".comment").cloned();
    match (&pick, &file) {
        (Some(s), Ok(bytes)) => {
            let d = f.section_data(s.index);
            let want = bytes.get(s.offset as usize..(s.offset + s.size) as usize);
            t.check(d.is_some() && d.as_deref() == want, format_args!("elf: section {} ({} bytes) read through GetSection equals the file at its offset", s.name, s.size));
        }
        _ => t.check(false, format_args!("elf: no .rodata/.comment section or file unreadable ({:?})", file.as_ref().err())),
    }

    // The binary is linked stripped: _start is its one global function.
    let main = f.find_symbol("_start");
    say!("info: elf: _start {:?}", main);
    let back = main.as_ref().and_then(|s| f.symbol_at(s.address));
    let text_addr = secs.iter().find(|s| s.name == ".text").map(|s| s.addr).unwrap_or(0);
    t.check(matches!(&main, Some(s) if s.kind == 2 && s.bind == 1 && s.size == 336 && s.value > text_addr && s.address != 0), format_args!("elf: _start found by name: a global function of 336 bytes, its file value inside .text, its loaded address"));
    t.check(back.is_some() && back == main, format_args!("elf: the symbol at _start's loaded address is _start, the same in every field ({:?})", back.as_ref().map(|s| &s.name)));
    t.check(f.find_symbol("no_such_symbol_libtest").is_none(), format_args!("elf: a missing symbol is None"));
    let all = f.symbols();
    let has_main = all.iter().any(|s| Some(s) == main.as_ref());
    // .symtab's entry count, its null entry 0 included (the binary is linked
    // stripped: only a few symbols).
    let n = secs.iter().find(|s| s.name == ".symtab").map(|s| s.size / s.entry_size.max(1)).unwrap_or(0) as usize;
    say!("info: elf: {} symbols scanned, .symtab has {} entries", all.len(), n);
    t.check(n > 1 && all.len() == n && has_main, format_args!("elf: ScanSymbolTable copies every entry of .symtab (the null one too), _start among them with the same value"));

    // The same file from memory.
    if let Ok(bytes) = &file {
        match ElfFile::from_memory(bytes) {
            Ok(m) => {
                // Each handle loads its own copy: the addresses differ.
                let strip = |s: Option<amigaos4::elf::Symbol>| s.map(|mut s| {
                    s.address = 0;
                    s
                });
                let mm = m.find_symbol("_start");
                let back = mm.as_ref().and_then(|s| m.symbol_at(s.address));
                t.check(m.header() == h && strip(mm.clone()) == strip(main.clone()) && back == mm && m.sections() == secs, format_args!("elf: the same image from borrowed bytes gives the same header, sections and symbol (at its own loaded address)"));
            }
            Err(e) => t.check(false, format_args!("elf: from memory: {}", e)),
        }
    }
    drop(f);
    t.check(ElfFile::open("RAM:no-such-elf").is_err() && ElfFile::open("S:Startup-Sequence").is_err() && ElfFile::from_memory(b"not an elf image at all, but long enough to pass the size check....").is_err(), format_args!("elf: a missing file, a text file and bytes that are not ELF are refused"));

    let m0 = avail();
    for _ in 0..20 {
        if let Ok(f) = ElfFile::open(SELF) {
            let _ = f.find_symbol("rust_main");
            let _ = f.sections();
        }
    }
    say!("info: elf: free memory around 20 open/query/close cycles: {:+} bytes", avail() as i64 - m0 as i64);

    // Shared objects. libtest has no C runtime, so it cannot run a real
    // object's start-up code (libz.so crashed in DLOpen); only the failure
    // path is checked here.
    use amigaos4::elf::SharedObject;
    // SAFETY: the object does not exist, so no code is loaded or run.
    t.check(unsafe { SharedObject::open("no_such_object.so") }.is_err(), format_args!("elf: DLOpen of a missing object fails"));
}

// ---------------------------------------------------------------------------
// AmigaInput: devices, queries, polling
// ---------------------------------------------------------------------------

fn run_ain(t: &mut Tally) {
    use amigaos4::input::{self as ain, q, Context};

    // SAFETY: reading the glue's global (left NULL on purpose).
    let null = unsafe { sys::IAmigaInput }.is_null();
    let ctx = match Context::new() {
        Ok(c) => c,
        Err(e) => return t.check(false, format_args!("ain: AIN_CreateContext: {}", e)),
    };
    t.check(null, format_args!("ain: a polling context created through the crate's own interface"));
    let devs = ctx.devices();
    for d in &devs {
        say!("info: ain: device {:?}", d);
    }
    let texts: Vec<String> = (0..=21).map(ain::error_string).collect();
    say!("info: ain: AINERR 3 = {:?}, 11 = {:?}", texts[3], texts[11]);
    t.check(texts.iter().skip(1).all(|s| !s.is_empty()), format_args!("ain: AIN_GetErrorString has text for AINERR 1..21"));
    let bad = ctx.obtain(0xFFFF_FFF0);
    t.check(bad.is_err(), format_args!("ain: an unknown device id is refused ({:?})", bad.as_ref().err()));
    drop(bad);
    if devs.is_empty() {
        say!("info: ain: no AmigaInput devices on this machine: device checks skipped");
        return;
    }
    let agree = devs.iter().all(|d| ctx.query(d.id, q::NUMAXES, 0) == Ok(d.axes) && ctx.query(d.id, q::NUMBUTTONS, 0) == Ok(d.buttons) && ctx.query(d.id, q::NUMHATS, 0) == Ok(d.hats));
    t.check(agree, format_args!("ain: AIN_Query's axis, button and hat counts agree with the enumeration for all {} devices", devs.len()));
    let d0 = devs[0].clone();
    match ctx.obtain(d0.id) {
        Ok(dev) => {
            let st = dev.read();
            say!("info: ain: {} reads {:?}", d0.name, st);
            t.check(matches!(&st, Ok(s) if s.axes.len() == d0.axes as usize && s.buttons.len() == d0.buttons as usize && s.hats.len() == d0.hats as usize), format_args!("ain: {} obtained and read: one value per axis, button and hat", d0.name));
            let twice = ctx.obtain(d0.id);
            let e = ctx.last_error();
            t.check(twice.is_err() && e.code == 11, format_args!("ain: obtaining the same device twice in one context is AINERR_DOUBLEOBTAIN ({:?})", e));
        }
        Err(e) => t.check(false, format_args!("ain: obtain {}: {} ({:?})", d0.name, e, ctx.last_error())),
    }
    drop(ctx);
    let m0 = avail();
    for _ in 0..20 {
        if let Ok(c) = Context::new() {
            if let Ok(d) = c.obtain(d0.id) {
                let _ = d.read();
            }
        }
    }
    say!("info: ain: free memory around 20 context/obtain/read cycles: {:+} bytes", avail() as i64 - m0 as i64);
}

/// Polls the first mouse for 30 s while the host moves the pointer and
/// clicks.
fn run_aingui(t: &mut Tally) {
    use amigaos4::input::{Context, AINDT_MOUSE};
    let ctx = match Context::new() {
        Ok(c) => c,
        Err(e) => return t.check(false, format_args!("aingui: context: {}", e)),
    };
    let Some(m) = ctx.devices().into_iter().find(|d| d.kind == AINDT_MOUSE) else {
        return t.check(false, format_args!("aingui: no mouse device"));
    };
    let dev = match ctx.obtain(m.id) {
        Ok(d) => d,
        Err(e) => return t.check(false, format_args!("aingui: obtain {}: {}", m.name, e)),
    };
    let first = dev.read();
    say!("aingui: ready: {} {:?}", m.name, first);
    let Ok(first) = first else {
        return t.check(false, format_args!("aingui: read: {:?}", ctx.last_error()));
    };
    let (mut axis_moved, mut pressed) = (false, false);
    let t0 = amigaos4::arch::timebase();
    let mut last = first.clone();
    while ms_since(t0) < 30_000 && !(axis_moved && pressed) {
        if let Ok(s) = dev.read() {
            if s != last {
                say!("info: aingui: {:?}", s);
                axis_moved |= s.axes != first.axes;
                pressed |= s.buttons.iter().any(|&b| b != 0);
                last = s;
            }
        }
        busy_ms(20);
    }
    t.check(axis_moved, format_args!("aingui: moving the pointer changes the mouse's axes"));
    t.check(pressed, format_args!("aingui: a held button reads non-zero"));
}

#[no_mangle]
pub extern "C" fn rust_main(args: *const u8, arglen: i32) -> i32 {
    // SAFETY: the Shell's argument string, `arglen` bytes.
    let text = unsafe { core::slice::from_raw_parts(args, arglen.max(0) as usize) };
    let text = core::str::from_utf8(text).unwrap_or("");
    let what = text.split_whitespace().next().unwrap_or("all");

    let mut t = Tally { pass: 0, total: 0 };
    let m0 = avail();
    say!("libtest {}: main task at priority {}", what, task::priority());
    if what == "dtprobe" {
        probe_dt_write();
    }
    if what == "notifyprobe" {
        probe_notify();
    }
    if what == "probe" {
        probe_names();
    }
    if what == "utility" || what == "all" {
        run_utility(&mut t);
    }
    if what == "dos" || what == "all" {
        run_dos(&mut t);
    }
    if what == "process" || what == "dos" || what == "all" {
        run_process(&mut t);
    }
    if what == "args" || what == "dos" || what == "all" {
        run_args(&mut t, what);
    }
    if what == "dos3" || what == "dos" || what == "all" {
        run_dos3(&mut t);
    }
    if what == "dos4" || what == "dos" || what == "all" {
        run_dos4(&mut t);
    }
    if what == "timer" || what == "all" {
        run_timer(&mut t);
    }
    if what == "net" || what == "all" {
        run_net(&mut t);
    }
    if what == "locale" || what == "all" {
        run_locale(&mut t);
    }
    if what == "app" || what == "all" {
        run_app(&mut t);
    }
    if what == "gfx" || what == "all" {
        run_gfx(&mut t);
    }
    if what == "win" || what == "all" {
        run_win(&mut t);
    }
    if what == "gui" {
        run_gui(&mut t);
    }
    if what == "layers" || what == "all" {
        run_layers(&mut t);
    }
    if what == "wb" || what == "all" {
        run_wb(&mut t);
    }
    if what == "wbgui" {
        run_wbgui(&mut t);
    }
    if what == "asl" || what == "all" {
        run_asl(&mut t);
    }
    if what == "aslgui" {
        run_aslgui(&mut t);
    }
    if what == "menu" || what == "all" {
        run_menu(&mut t);
    }
    if what == "menugui" {
        run_menugui(&mut t);
    }
    if what == "ra" || what == "all" {
        run_ra(&mut t);
    }
    if what == "ragui" {
        run_ragui(&mut t);
    }
    if what == "imgicon" {
        run_imgicon(&mut t);
    }
    if what == "appimg" {
        run_appimg(&mut t);
    }
    if what == "dt" || what == "all" {
        run_dt(&mut t);
    }
    if what == "con" || what == "all" {
        run_con(&mut t);
    }
    if what == "congui" {
        run_congui(&mut t);
    }
    if what == "elf" || what == "all" {
        run_elf(&mut t);
    }
    if what == "ain" || what == "all" {
        run_ain(&mut t);
    }
    if what == "aingui" {
        run_aingui(&mut t);
    }
    if what == "pci" || what == "all" {
        run_expansion(&mut t);
        run_pci(&mut t);
    }
    say!("info: free memory over the whole run: {:+} bytes", avail() as i64 - m0 as i64);
    say!("libtest: {}/{} passed", t.pass, t.total);
    if t.pass == t.total && t.total > 0 { 0 } else { 20 }
}
