//! asl.library's standard requesters, safe (feature `asl`, with `dos` and
//! `intuition`).
//!
//! - [`FileRequester`]: choose a file (or several, or a drawer) to open or
//!   save. The requester is kept between calls, so it remembers the
//!   drawer, the file and its own place and size, as Amiga programs do.
//! - [`FontRequester`]: choose a font and size.
//! - [`ScreenModeRequester`]: choose a display mode (and size and depth).
//!
//! Each is AllocAslRequest with no tags, then AslRequest with the options
//! of one call; FreeAslRequest on drop. A request blocks its caller until
//! the user answers; with a parent window that window is put to sleep (a
//! busy pointer, no input) for that time. The answers are copied out.
//! `Ok(None)` is the user's Cancel (or asl.library failing to open the
//! requester, which it reports the same way).

use alloc::string::String;
use alloc::vec::Vec;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::intuition::ParentWindow;
use crate::sys::{AslIFace, TagItem, APTR, TAG_DONE};

crate::iface::iface!(iface, AslIFace, IAsl, b"asl.library\0", b"main\0", 50);

// AllocAslRequest types.
const ASL_FILE_REQUEST: u32 = 0;
const ASL_FONT_REQUEST: u32 = 1;
const ASL_SCREEN_MODE_REQUEST: u32 = 2;

// Tags (libraries/asl.h, ASL_TB = TAG_USER + 0x80000). The three kinds
// share the numbers of the common ones.
const ASL_TB: u32 = 0x8008_0000;
const TITLE_TEXT: u32 = ASL_TB + 1;
const WINDOW: u32 = ASL_TB + 2;
const INITIAL_LEFT_EDGE: u32 = ASL_TB + 3;
const INITIAL_TOP_EDGE: u32 = ASL_TB + 4;
const INITIAL_WIDTH: u32 = ASL_TB + 5;
const INITIAL_HEIGHT: u32 = ASL_TB + 6;
const PUB_SCREEN_NAME: u32 = ASL_TB + 41;
const SLEEP_WINDOW: u32 = ASL_TB + 43;
const ASLFR_INITIAL_FILE: u32 = ASL_TB + 8;
const ASLFR_INITIAL_DRAWER: u32 = ASL_TB + 9;
const ASLFR_INITIAL_PATTERN: u32 = ASL_TB + 10;
const ASLFR_POSITIVE_TEXT: u32 = ASL_TB + 18;
const ASLFR_DO_SAVE_MODE: u32 = ASL_TB + 44;
const ASLFR_DO_MULTI_SELECT: u32 = ASL_TB + 45;
const ASLFR_DO_PATTERNS: u32 = ASL_TB + 46;
const ASLFR_DRAWERS_ONLY: u32 = ASL_TB + 47;
const ASLFR_REJECT_ICONS: u32 = ASL_TB + 60;
const ASLFO_INITIAL_NAME: u32 = ASL_TB + 10;
const ASLFO_INITIAL_SIZE: u32 = ASL_TB + 11;
const ASLFO_MIN_HEIGHT: u32 = ASL_TB + 16;
const ASLFO_MAX_HEIGHT: u32 = ASL_TB + 17;
const ASLFO_FIXED_WIDTH_ONLY: u32 = ASL_TB + 48;
const ASLSM_INITIAL_DISPLAY_ID: u32 = ASL_TB + 100;
const ASLSM_DO_WIDTH: u32 = ASL_TB + 109;
const ASLSM_DO_HEIGHT: u32 = ASL_TB + 110;
const ASLSM_DO_DEPTH: u32 = ASL_TB + 111;

// struct FileRequester, FontRequester (fo_Attr = struct TextAttr),
// ScreenModeRequester; struct WBArg (tests/layout/asl.c).
const FR_FILE: usize = 4;
const FR_DRAWER: usize = 8;
const FR_NUM_ARGS: usize = 32;
const FR_ARG_LIST: usize = 36;
const WBARG_SIZE: usize = 8;
const WBARG_NAME: usize = 4;
const FO_NAME: usize = 8;
const FO_YSIZE: usize = 12;
const FO_STYLE: usize = 14;
const FO_FLAGS: usize = 15;
const SM_DISPLAY_ID: usize = 0;
const SM_DISPLAY_WIDTH: usize = 4;
const SM_DISPLAY_HEIGHT: usize = 8;
const SM_DISPLAY_DEPTH: usize = 12;
const SM_OVERSCAN_TYPE: usize = 14;
const SM_AUTO_SCROLL: usize = 16;

fn rd<T: Copy>(p: *const u8, off: usize) -> T {
    // SAFETY: callers pass a live requester (or WBArg) and a field offset
    // from its header (tests/layout/asl.c).
    unsafe { core::ptr::read_unaligned(p.add(off) as *const T) }
}

fn text(p: *const u8) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: a NUL-terminated string the requester owns, copied.
    String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned()
}

/// The tags of one request and the C strings they point at, kept alive
/// together until AslRequest returns.
struct Tags {
    items: Vec<TagItem>,
    strings: Vec<Vec<u8>>,
}

impl Tags {
    fn new() -> Self {
        Self { items: Vec::new(), strings: Vec::new() }
    }

    fn val(&mut self, tag: u32, data: u32) {
        self.items.push(TagItem { ti_Tag: tag, ti_Data: data });
    }

    fn flag(&mut self, tag: u32, on: bool) {
        self.val(tag, on as u32);
    }

    fn text(&mut self, tag: u32, s: Option<&str>) -> Result<()> {
        if let Some(s) = s {
            let c = c_copy(s.as_bytes())?;
            // The Vec's heap block does not move when `strings` grows.
            self.val(tag, c.as_ptr() as u32);
            self.strings.push(c);
        }
        Ok(())
    }

    /// The options every kind has.
    fn common(&mut self, win: Option<&dyn ParentWindow>, c: &Common<'_>) -> Result<()> {
        self.text(TITLE_TEXT, c.title)?;
        if let Some(w) = win.map(|w| w.window_ptr()).filter(|w| !w.is_null()) {
            self.val(WINDOW, w as u32);
            self.flag(SLEEP_WINDOW, true);
        }
        self.text(PUB_SCREEN_NAME, c.pub_screen)?;
        if let Some((l, t, w, h)) = c.bounds {
            self.val(INITIAL_LEFT_EDGE, l as u32);
            self.val(INITIAL_TOP_EDGE, t as u32);
            self.val(INITIAL_WIDTH, w as u32);
            self.val(INITIAL_HEIGHT, h as u32);
        }
        Ok(())
    }

    fn done(mut self) -> Self {
        self.val(TAG_DONE, 0);
        self
    }
}

/// What every requester takes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Common<'a> {
    /// The requester window's title.
    pub title: Option<&'a str>,
    /// The public screen to open on when there is no parent window
    /// (default: the default public screen).
    pub pub_screen: Option<&'a str>,
    /// Place and size: left, top, width, height (default: asl's own, or
    /// where the user left it last time).
    pub bounds: Option<(i16, i16, i16, i16)>,
}

/// One allocated requester; FreeAslRequest on drop.
struct Req {
    i: *mut AslIFace,
    p: APTR,
}

impl Req {
    fn new(kind: u32) -> Result<Self> {
        let i = iface()?;
        // SAFETY: an open interface, a request type from asl.h, no tags.
        let p = unsafe { ((*i).AllocAslRequest)(i, kind, core::ptr::null()) };
        if p.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(Self { i, p }) }
    }

    /// AslRequest with `tags`: true when the user chose (not Cancel).
    fn run(&mut self, tags: &Tags) -> bool {
        // SAFETY: our requester, used by this task only (&mut), and a
        // TAG_DONE-ended list whose strings and window outlive the call
        // (borrowed by our caller for it).
        unsafe { ((*self.i).AslRequest)(self.i, self.p, tags.items.as_ptr()) != 0 }
    }

    fn at(&self) -> *const u8 {
        self.p as *const u8
    }
}

impl Drop for Req {
    fn drop(&mut self) {
        // SAFETY: our requester, not showing (AslRequest has returned),
        // freed once.
        unsafe { ((*self.i).FreeAslRequest)(self.i, self.p) };
    }
}

/// How to show a [`FileRequester`].
#[derive(Clone, Copy, Debug, Default)]
pub struct FileOptions<'a> {
    pub common: Common<'a>,
    /// The drawer to start in (default: the one the user chose last).
    pub drawer: Option<&'a str>,
    /// The file name to start with.
    pub file: Option<&'a str>,
    /// An AmigaDOS pattern of the files to show, e.g. `#?.(png|jpg)`,
    /// with a pattern field the user can change.
    pub pattern: Option<&'a str>,
    /// The text of the OK button.
    pub ok_text: Option<&'a str>,
    /// A save requester (a name that does not exist yet may be typed).
    pub save: bool,
    /// Several files may be chosen.
    pub multi_select: bool,
    /// Only drawers are shown: choose a drawer.
    pub drawers_only: bool,
    /// .info files are not shown.
    pub reject_icons: bool,
}

/// What the user chose in a [`FileRequester`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileChoice {
    /// The drawer, as typed or chosen (may be relative or empty).
    pub drawer: String,
    /// The file name (empty when only a drawer was chosen).
    pub file: String,
    /// The full paths: the drawer joined with each file chosen (one, or
    /// several with `multi_select`; the drawer alone when no file was).
    pub paths: Vec<String>,
}

/// asl's file requester, kept between requests.
pub struct FileRequester {
    req: Req,
}

impl FileRequester {
    pub fn new() -> Result<Self> {
        Ok(Self { req: Req::new(ASL_FILE_REQUEST)? })
    }

    /// Show the requester and wait for the user. `parent`: the window to
    /// open on and put to sleep meanwhile.
    pub fn request(&mut self, parent: Option<&dyn ParentWindow>, o: &FileOptions<'_>) -> Result<Option<FileChoice>> {
        let mut t = Tags::new();
        t.common(parent, &o.common)?;
        t.text(ASLFR_INITIAL_DRAWER, o.drawer)?;
        t.text(ASLFR_INITIAL_FILE, o.file)?;
        if o.pattern.is_some() {
            t.text(ASLFR_INITIAL_PATTERN, o.pattern)?;
            t.flag(ASLFR_DO_PATTERNS, true);
        }
        t.text(ASLFR_POSITIVE_TEXT, o.ok_text)?;
        t.flag(ASLFR_DO_SAVE_MODE, o.save);
        t.flag(ASLFR_DO_MULTI_SELECT, o.multi_select);
        t.flag(ASLFR_DRAWERS_ONLY, o.drawers_only);
        t.flag(ASLFR_REJECT_ICONS, o.reject_icons);
        let t = t.done();
        if !self.req.run(&t) {
            return Ok(None);
        }
        let p = self.req.at();
        let drawer = text(rd(p, FR_DRAWER));
        let file = text(rd(p, FR_FILE));
        let join = |name: &str| if name.is_empty() { drawer.clone() } else { crate::dos::add_part(&drawer, name).unwrap_or_else(|_| String::from(name)) };
        let mut paths = Vec::new();
        let n = rd::<i32>(p, FR_NUM_ARGS).clamp(0, 65536) as usize;
        let list: *const u8 = rd(p, FR_ARG_LIST);
        if o.multi_select && n > 0 && !list.is_null() {
            for k in 0..n {
                // SAFETY: element k of the requester's fr_NumArgs WBArgs.
                let a = unsafe { list.add(k * WBARG_SIZE) };
                paths.push(join(&text(rd(a, WBARG_NAME))));
            }
        } else {
            paths.push(join(&file));
        }
        Ok(Some(FileChoice { drawer: drawer.clone(), file, paths }))
    }
}

/// How to show a [`FontRequester`].
#[derive(Clone, Copy, Debug, Default)]
pub struct FontOptions<'a> {
    pub common: Common<'a>,
    /// The font to start with, e.g. `DejaVu Sans.font`.
    pub name: Option<&'a str>,
    /// Its size.
    pub size: Option<u16>,
    /// The sizes offered (default: asl's 5 to 24).
    pub min_height: Option<u16>,
    pub max_height: Option<u16>,
    /// Only fixed-width fonts.
    pub fixed_width_only: bool,
}

/// A font chosen in a [`FontRequester`] (a struct TextAttr's contents).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontChoice {
    pub name: String,
    pub size: u16,
    pub style: u8,
    pub flags: u8,
}

/// asl's font requester, kept between requests.
pub struct FontRequester {
    req: Req,
}

impl FontRequester {
    pub fn new() -> Result<Self> {
        Ok(Self { req: Req::new(ASL_FONT_REQUEST)? })
    }

    pub fn request(&mut self, parent: Option<&dyn ParentWindow>, o: &FontOptions<'_>) -> Result<Option<FontChoice>> {
        let mut t = Tags::new();
        t.common(parent, &o.common)?;
        t.text(ASLFO_INITIAL_NAME, o.name)?;
        if let Some(s) = o.size {
            t.val(ASLFO_INITIAL_SIZE, s as u32);
        }
        if let Some(s) = o.min_height {
            t.val(ASLFO_MIN_HEIGHT, s as u32);
        }
        if let Some(s) = o.max_height {
            t.val(ASLFO_MAX_HEIGHT, s as u32);
        }
        t.flag(ASLFO_FIXED_WIDTH_ONLY, o.fixed_width_only);
        let t = t.done();
        if !self.req.run(&t) {
            return Ok(None);
        }
        let p = self.req.at();
        Ok(Some(FontChoice { name: text(rd(p, FO_NAME)), size: rd(p, FO_YSIZE), style: rd(p, FO_STYLE), flags: rd(p, FO_FLAGS) }))
    }
}

/// How to show a [`ScreenModeRequester`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenModeOptions<'a> {
    pub common: Common<'a>,
    /// The mode to start with.
    pub display_id: Option<u32>,
    /// Width, height and depth fields too.
    pub do_width: bool,
    pub do_height: bool,
    pub do_depth: bool,
}

/// A mode chosen in a [`ScreenModeRequester`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenMode {
    pub display_id: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u16,
    pub overscan: u16,
    pub auto_scroll: bool,
}

/// asl's screen mode requester, kept between requests.
pub struct ScreenModeRequester {
    req: Req,
}

impl ScreenModeRequester {
    pub fn new() -> Result<Self> {
        Ok(Self { req: Req::new(ASL_SCREEN_MODE_REQUEST)? })
    }

    pub fn request(&mut self, parent: Option<&dyn ParentWindow>, o: &ScreenModeOptions<'_>) -> Result<Option<ScreenMode>> {
        let mut t = Tags::new();
        t.common(parent, &o.common)?;
        if let Some(id) = o.display_id {
            t.val(ASLSM_INITIAL_DISPLAY_ID, id);
        }
        t.flag(ASLSM_DO_WIDTH, o.do_width);
        t.flag(ASLSM_DO_HEIGHT, o.do_height);
        t.flag(ASLSM_DO_DEPTH, o.do_depth);
        let t = t.done();
        if !self.req.run(&t) {
            return Ok(None);
        }
        let p = self.req.at();
        Ok(Some(ScreenMode {
            display_id: rd(p, SM_DISPLAY_ID),
            width: rd(p, SM_DISPLAY_WIDTH),
            height: rd(p, SM_DISPLAY_HEIGHT),
            depth: rd(p, SM_DISPLAY_DEPTH),
            overscan: rd(p, SM_OVERSCAN_TYPE),
            auto_scroll: rd::<u16>(p, SM_AUTO_SCROLL) != 0,
        }))
    }
}

// Not wrapped: AbortAslRequest / ActivateAslRequest (a requester run from
// another task), AslControl, hooks (filter functions), the obsolete
// pre-V36 calls.
