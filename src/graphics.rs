//! Off-screen graphics, safe (feature `graphics`; graphics.library 54).
//!
//! - [`BitMap`]: AllocBitMapTagList, FreeBitMap (after WaitBlit) on drop;
//!   its size and pixel format, a [`BitMapLock`] for its memory, and
//!   blits between bitmaps (BltBitMap).
//! - [`RastPort`]: a RastPort of the crate's own on a borrowed bitmap
//!   (InitRastPort). A RastPort without layers does not clip, so every
//!   drawing call here checks its coordinates against the bitmap and
//!   refuses (`InvalidArgument`) what would reach outside it: pens, lines,
//!   rectangles, ellipses, pixels, pixel arrays (Read/WritePixelArray),
//!   text.
//! - [`Font`]: OpenFont / CloseFont; a RastPort borrows the font it uses.
//! - [`Region`]: NewRegion / DisposeRegion and the rectangle and region
//!   operations, with its bounds and rectangles read back.
//!
//! Text is bytes in the font's character set (Latin-1), not UTF-8. Pens
//! mean something only on palette (CLUT) bitmaps; on true-colour ones use
//! the `*_color` calls (ARGB). Windows and layers are not covered here.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::sys::{self, GraphicsIFace, TagItem, APTR, TAG_DONE, TAG_USER};

crate::iface::iface!(iface, GraphicsIFace, IGraphics, b"graphics.library\0", b"main\0", 54);

// graphics/gfx.h, rastport.h
const BMA_HEIGHT: u32 = 0;
const BMA_DEPTH: u32 = 4;
const BMA_WIDTH: u32 = 8;
const BMA_ISRTG: u32 = 16;
const BMA_PIXELFORMAT: u32 = 19;
const BMATAGS_FRIEND: u32 = TAG_USER;
const BMATAGS_PIXEL_FORMAT: u32 = TAG_USER + 2;
const BMATAGS_CLEAR: u32 = TAG_USER + 3;
const BMATAGS_USER_PRIVATE: u32 = TAG_USER + 17;
const LBM_BASE_ADDRESS: u32 = TAG_USER + 1;
const LBM_BYTES_PER_ROW: u32 = TAG_USER + 2;
const LBM_PIXEL_FORMAT: u32 = TAG_USER + 3;

/// struct RastPort: 100 bytes; Layer at 0, BitMap at 4, Font at 52.
/// struct TextFont: tf_YSize at 20, tf_XSize at 24, tf_Baseline at 26.
/// struct Region: bounds at 0, RegionRectangle list at 8; a
/// RegionRectangle: Next at 0, bounds at 8 (tests/layout/graphics.c).
const RASTPORT_SIZE: usize = 100;
const RP_BITMAP: usize = 4;
const RP_FONT: usize = 52;
const TF_YSIZE: usize = 20;
const TF_XSIZE: usize = 24;
const TF_BASELINE: usize = 26;
const RG_RECTS: usize = 8;
const RR_BOUNDS: usize = 8;

/// A pixel format of a bitmap or a pixel array (PIXF_*).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixFmt {
    /// Palette indexes, one byte.
    Clut,
    /// 3 bytes R, G, B.
    R8G8B8,
    /// 4 bytes A, R, G, B.
    A8R8G8B8,
    /// 4 bytes R, G, B, A.
    R8G8B8A8,
    /// 4 bytes B, G, R, A.
    B8G8R8A8,
}

impl PixFmt {
    fn code(self) -> u32 {
        match self {
            PixFmt::Clut => 1,
            PixFmt::R8G8B8 => 2,
            PixFmt::A8R8G8B8 => 6,
            PixFmt::R8G8B8A8 => 8,
            PixFmt::B8G8R8A8 => 9,
        }
    }

    fn from_code(c: u32) -> Option<Self> {
        Some(match c {
            1 => PixFmt::Clut,
            2 => PixFmt::R8G8B8,
            6 => PixFmt::A8R8G8B8,
            8 => PixFmt::R8G8B8A8,
            9 => PixFmt::B8G8R8A8,
            _ => return None,
        })
    }

    pub fn bytes_per_pixel(self) -> usize {
        match self {
            PixFmt::Clut => 1,
            PixFmt::R8G8B8 => 3,
            _ => 4,
        }
    }
}

/// An inclusive rectangle, as RectFill takes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Rect { x0, y0, x1, y1 }
    }

    pub fn width(&self) -> i32 {
        self.x1 - self.x0 + 1
    }

    pub fn height(&self) -> i32 {
        self.y1 - self.y0 + 1
    }

    pub fn area(&self) -> i64 {
        if self.x1 < self.x0 || self.y1 < self.y0 { 0 } else { self.width() as i64 * self.height() as i64 }
    }

    fn raw(&self) -> [i16; 4] {
        [self.x0 as i16, self.y0 as i16, self.x1 as i16, self.y1 as i16]
    }
}

/// How a bitmap is made.
#[derive(Clone, Copy, Debug, Default)]
pub struct BitMapOptions {
    /// A true-colour format (an RTG bitmap); None: a planar/CLUT bitmap of
    /// the given depth.
    pub pixel_format: Option<PixFmt>,
    pub clear: bool,
    /// Memory the program may lock and address directly; it never moves.
    pub user_private: bool,
}

/// Free a registered bitmap (the panic path; see `crate::track`).
unsafe fn release_bitmap(bm: usize, _: usize) {
    if let Ok(g) = iface() {
        // SAFETY: the registry calls this once, for a bitmap still
        // allocated, after any blit has finished.
        unsafe {
            ((*g).WaitBlit)(g);
            ((*g).FreeBitMap)(g, bm as *mut sys::BitMap);
        }
    }
}

/// Close a registered font.
unsafe fn release_font(f: usize, _: usize) {
    if let Ok(g) = iface() {
        // SAFETY: as release_bitmap, for a font still open.
        unsafe { ((*g).CloseFont)(g, f as *mut sys::TextFont) };
    }
}

/// An off-screen bitmap; WaitBlit + FreeBitMap on drop.
pub struct BitMap {
    _t: crate::track::Tracked,
    g: *mut GraphicsIFace,
    bm: *mut sys::BitMap,
    width: u32,
    height: u32,
    _p: PhantomData<*const ()>,
}

impl BitMap {
    /// AllocBitMapTagList(`width`, `height`, `depth`, ...).
    #[track_caller]
    pub fn new(width: u32, height: u32, depth: u32, opt: &BitMapOptions) -> Result<Self> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 || depth == 0 || depth > 32 {
            return Err(AmigaError::InvalidArgument("bitmap size or depth out of range"));
        }
        let g = iface()?;
        let mut tags = Vec::with_capacity(4);
        if let Some(f) = opt.pixel_format {
            tags.push(TagItem { ti_Tag: BMATAGS_PIXEL_FORMAT, ti_Data: f.code() });
        }
        tags.push(TagItem { ti_Tag: BMATAGS_CLEAR, ti_Data: opt.clear as u32 });
        tags.push(TagItem { ti_Tag: BMATAGS_USER_PRIVATE, ti_Data: opt.user_private as u32 });
        tags.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        // SAFETY: an open interface and a TAG_DONE-ended list.
        let bm = unsafe { ((*g).AllocBitMapTagList)(g, width, height, depth, tags.as_mut_ptr() as *mut APTR) };
        if bm.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { g, bm, width, height, _t: crate::track::register("bitmap", bm as usize, 0, release_bitmap), _p: PhantomData })
    }

    /// A bitmap like `friend` (same format), `width` by `height`.
    #[track_caller]
    pub fn new_like(friend: &BitMap, width: u32, height: u32) -> Result<Self> {
        let g = friend.g;
        let tags = [
            TagItem { ti_Tag: BMATAGS_FRIEND, ti_Data: friend.bm as u32 },
            TagItem { ti_Tag: BMATAGS_CLEAR, ti_Data: 1 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: an open interface, a live friend bitmap, a TAG_DONE-ended
        // list.
        let bm = unsafe { ((*g).AllocBitMapTagList)(g, width, height, friend.depth(), tags.as_ptr() as *mut APTR) };
        if bm.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { g, bm, width, height, _t: crate::track::register("bitmap", bm as usize, 0, release_bitmap), _p: PhantomData })
    }

    fn attr(&self, a: u32) -> u32 {
        // SAFETY: our live bitmap.
        unsafe { ((*self.g).GetBitMapAttr)(self.g, self.bm as *mut APTR, a) }
    }

    /// The width asked for (the bitmap itself may be wider: see
    /// [`allocated_width`](Self::allocated_width)).
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// GetBitMapAttr(BMA_WIDTH): possibly rounded up.
    pub fn allocated_width(&self) -> u32 {
        self.attr(BMA_WIDTH)
    }

    pub fn allocated_height(&self) -> u32 {
        self.attr(BMA_HEIGHT)
    }

    pub fn depth(&self) -> u32 {
        self.attr(BMA_DEPTH)
    }

    /// Whether it is an RTG (graphics card style) bitmap.
    pub fn is_rtg(&self) -> bool {
        self.attr(BMA_ISRTG) != 0
    }

    /// Its pixel format, if it is one this module knows.
    pub fn pixel_format(&self) -> Option<PixFmt> {
        PixFmt::from_code(self.attr(BMA_PIXELFORMAT))
    }

    fn check(&self, r: &Rect) -> Result<()> {
        if r.x1 < r.x0 || r.y1 < r.y0 || r.x0 < 0 || r.y0 < 0 || r.x1 >= self.width as i32 || r.y1 >= self.height as i32 {
            return Err(AmigaError::InvalidArgument("rectangle outside the bitmap or reversed"));
        }
        Ok(())
    }

    /// BltBitMap: copy `src`'s rectangle `from` to (`x`, `y`) here.
    pub fn blit_from(&mut self, src: &BitMap, from: Rect, x: i32, y: i32) -> Result<()> {
        src.check(&from)?;
        self.check(&Rect::new(x, y, x + from.width() - 1, y + from.height() - 1))?;
        self.blt(src.bm, from, x, y)
    }

    /// BltBitMap within this bitmap (the areas may overlap).
    pub fn blit_within(&mut self, from: Rect, x: i32, y: i32) -> Result<()> {
        self.check(&from)?;
        self.check(&Rect::new(x, y, x + from.width() - 1, y + from.height() - 1))?;
        let bm = self.bm;
        self.blt(bm, from, x, y)
    }

    fn blt(&mut self, src: *mut sys::BitMap, from: Rect, x: i32, y: i32) -> Result<()> {
        // SAFETY: two live bitmaps (or ours twice) and rectangles checked
        // to lie inside them; minterm 0xC0 = copy, all planes.
        unsafe {
            ((*self.g).BltBitMap)(
                self.g,
                src as *mut APTR,
                from.x0 as i16,
                from.y0 as i16,
                self.bm,
                x as i16,
                y as i16,
                from.width() as i16,
                from.height() as i16,
                0xC0,
                0xFF,
                core::ptr::null_mut(),
            )
        };
        Ok(())
    }

    /// LockBitMapTagList: the bitmap's memory, while the lock lives (keep
    /// it short: screen switching waits). None for bitmaps that cannot be
    /// locked (planar ones).
    pub fn lock(&mut self) -> Option<BitMapLock<'_>> {
        let (mut base, mut bpr, mut fmt) = (0u32, 0u32, 0u32);
        let tags = [
            TagItem { ti_Tag: LBM_BASE_ADDRESS, ti_Data: &mut base as *mut u32 as u32 },
            TagItem { ti_Tag: LBM_BYTES_PER_ROW, ti_Data: &mut bpr as *mut u32 as u32 },
            TagItem { ti_Tag: LBM_PIXEL_FORMAT, ti_Data: &mut fmt as *mut u32 as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our bitmap; the tags point at u32s on this stack.
        let h = unsafe { ((*self.g).LockBitMapTagList)(self.g, self.bm, tags.as_ptr() as *mut TagItem) };
        if h.is_null() || base == 0 || bpr == 0 {
            if !h.is_null() {
                // SAFETY: the handle just obtained, released once.
                unsafe { ((*self.g).UnlockBitMap)(self.g, h) };
            }
            return None;
        }
        let rows = self.allocated_height();
        Some(BitMapLock { g: self.g, h, base: base as *mut u8, len: bpr as usize * rows as usize, bpr, fmt: PixFmt::from_code(fmt), _b: PhantomData })
    }

    /// The bitmap for raw calls (and the crate's layers), valid while
    /// self is.
    /// Give up the bitmap (to an owner that frees it, such as
    /// picture.datatype's PDTA_BitMap): unregistered, not freed.
    pub fn into_raw(self) -> *mut sys::BitMap {
        let me = core::mem::ManuallyDrop::new(self);
        // SAFETY: `me` is never used or dropped again; its registration is
        // read out once and dropped (unregistered).
        drop(unsafe { core::ptr::read(&me._t) });
        me.bm
    }

    pub fn as_ptr(&self) -> *mut sys::BitMap {
        self.bm
    }

    /// A RastPort drawing into this bitmap.
    pub fn rastport(&mut self) -> Result<RastPort<'_>> {
        RastPort::new(self)
    }
}

impl Drop for BitMap {
    fn drop(&mut self) {
        // SAFETY: our bitmap, freed once after any blit has finished.
        unsafe {
            ((*self.g).WaitBlit)(self.g);
            ((*self.g).FreeBitMap)(self.g, self.bm);
        }
    }
}

/// A locked bitmap's memory; UnlockBitMap on drop.
pub struct BitMapLock<'b> {
    g: *mut GraphicsIFace,
    h: APTR,
    base: *mut u8,
    len: usize,
    bpr: u32,
    fmt: Option<PixFmt>,
    _b: PhantomData<&'b mut BitMap>,
}

impl BitMapLock<'_> {
    pub fn bytes_per_row(&self) -> u32 {
        self.bpr
    }

    pub fn pixel_format(&self) -> Option<PixFmt> {
        self.fmt
    }

    /// The pixel memory: bytes_per_row × the bitmap's rows.
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: the locked bitmap's memory, `len` bytes, valid until the
        // unlock (drop), not drawn into meanwhile (we borrow the bitmap
        // mutably).
        unsafe { core::slice::from_raw_parts(self.base, self.len) }
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: as above, borrowed mutably.
        unsafe { core::slice::from_raw_parts_mut(self.base, self.len) }
    }
}

impl Drop for BitMapLock<'_> {
    fn drop(&mut self) {
        // SAFETY: the lock handle we got, released once.
        unsafe { ((*self.g).UnlockBitMap)(self.g, self.h) };
    }
}

/// An open font; CloseFont on drop.
pub struct Font {
    _t: crate::track::Tracked,
    g: *mut GraphicsIFace,
    f: *mut sys::TextFont,
    _p: PhantomData<*const ()>,
}

impl Font {
    /// OpenFont: a ROM or loaded font ("topaz.font", 8). The nearest match
    /// is returned: check [`y_size`](Self::y_size).
    #[track_caller]
    pub fn open(name: &str, size: u16) -> Result<Self> {
        let g = iface()?;
        let n = c_copy(name.as_bytes())?;
        // struct TextAttr: name, YSize, Style, Flags (8 bytes).
        let mut ta = [0u32; 2];
        ta[0] = n.as_ptr() as u32;
        ta[1] = (size as u32) << 16;
        // SAFETY: an open interface and a TextAttr with a NUL-terminated
        // name, alive across the call.
        let f = unsafe { ((*g).OpenFont)(g, ta.as_mut_ptr() as *mut APTR) };
        if f.is_null() { Err(AmigaError::NullPointer) } else { Ok(Self { g, f, _t: crate::track::register("font", f as usize, 0, release_font), _p: PhantomData }) }
    }

    fn u16_at(&self, off: usize) -> u16 {
        // SAFETY: a field of our open TextFont at its header offset.
        unsafe { core::ptr::read((self.f as *const u8).add(off) as *const u16) }
    }

    pub fn y_size(&self) -> u16 {
        self.u16_at(TF_YSIZE)
    }

    /// The nominal width (exact for fixed-width fonts).
    pub fn x_size(&self) -> u16 {
        self.u16_at(TF_XSIZE)
    }

    pub fn baseline(&self) -> u16 {
        self.u16_at(TF_BASELINE)
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        // SAFETY: our font, closed once; no RastPort of ours uses it any
        // more (they borrow it).
        unsafe { ((*self.g).CloseFont)(self.g, self.f) };
    }
}

/// TextExtent's result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extent {
    pub width: u16,
    pub height: u16,
    /// Relative to the pen position (y negative above the baseline).
    pub bounds: Rect,
}

/// Where a RastPort lives.
#[cfg_attr(not(any(feature = "intuition", feature = "layers")), allow(dead_code))]
enum Store {
    /// The crate's own, on a borrowed bitmap.
    Own(Box<[u32; RASTPORT_SIZE / 4]>),
    /// A window's (or another owner's); its font is put back on drop.
    Borrowed { rp: *mut sys::RastPort, old_font: *mut sys::TextFont },
}

/// A RastPort: the crate's own drawing into a borrowed bitmap
/// ([`BitMap::rastport`]), or a window's, borrowed for `'a`
/// (`intuition::Window::rastport`). Fonts it uses are borrowed for `'a`
/// too; a borrowed RastPort gets its old font back on drop.
pub struct RastPort<'a> {
    g: *mut GraphicsIFace,
    store: Store,
    w: i32,
    h: i32,
    _b: PhantomData<&'a mut BitMap>,
    _f: PhantomData<&'a Font>,
}

impl<'a> RastPort<'a> {
    fn new(bm: &'a mut BitMap) -> Result<Self> {
        // SAFETY: our borrowed bitmap, valid and ours for 'a.
        unsafe { Self::on_raw(bm.bm, bm.width as i32, bm.height as i32) }
    }

    /// The crate's own RastPort on a bitmap it does not own.
    ///
    /// # Safety
    /// `bm` is a live bitmap at least `width` by `height` that stays valid
    /// for `'a`.
    pub(crate) unsafe fn on_raw(bm: *mut sys::BitMap, width: i32, height: i32) -> Result<Self> {
        let g = iface()?;
        let mut rp = Box::new([0u32; RASTPORT_SIZE / 4]);
        // SAFETY: a RastPort-sized, aligned block; InitRastPort fills it,
        // and its BitMap (at 4, which no call sets) is the caller's bitmap.
        unsafe {
            ((*g).InitRastPort)(g, rp.as_mut_ptr() as *mut sys::RastPort);
            core::ptr::write((rp.as_mut_ptr() as *mut u8).add(RP_BITMAP) as *mut *mut sys::BitMap, bm);
        }
        Ok(Self { g, store: Store::Own(rp), w: width, h: height, _b: PhantomData, _f: PhantomData })
    }

    /// Borrow another owner's RastPort, drawing limited to `width` by
    /// `height` from its origin.
    ///
    /// # Safety
    /// `rp` is a live, initialised RastPort that stays valid, and is used
    /// by no one else, for `'a`.
    #[cfg_attr(not(any(feature = "intuition", feature = "layers")), allow(dead_code))]
    pub(crate) unsafe fn borrow_raw(rp: *mut sys::RastPort, width: i32, height: i32) -> Result<Self> {
        let g = iface()?;
        // SAFETY: the caller's contract: a live RastPort; its font pointer
        // (at 52) is saved to be put back.
        let old_font = unsafe { core::ptr::read_unaligned((rp as *const u8).add(RP_FONT) as *const *mut sys::TextFont) };
        Ok(Self { g, store: Store::Borrowed { rp, old_font }, w: width.max(0), h: height.max(0), _b: PhantomData, _f: PhantomData })
    }

    fn p(&mut self) -> *mut sys::RastPort {
        match &mut self.store {
            Store::Own(b) => b.as_mut_ptr() as *mut sys::RastPort,
            Store::Borrowed { rp, .. } => *rp,
        }
    }

    fn p_const(&self) -> *const u8 {
        match &self.store {
            Store::Own(b) => b.as_ptr() as *const u8,
            Store::Borrowed { rp, .. } => *rp as *const u8,
        }
    }

    /// The drawable width and height.
    pub fn size(&self) -> (i32, i32) {
        (self.w, self.h)
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        (0..self.w).contains(&x) && (0..self.h).contains(&y)
    }

    fn check(&self, r: &Rect) -> Result<()> {
        if r.x1 < r.x0 || r.y1 < r.y0 || !self.inside(r.x0, r.y0) || !self.inside(r.x1, r.y1) {
            return Err(AmigaError::InvalidArgument("rectangle outside the bitmap or reversed"));
        }
        Ok(())
    }

    fn point(&self, x: i32, y: i32) -> Result<()> {
        if self.inside(x, y) { Ok(()) } else { Err(AmigaError::InvalidArgument("point outside the bitmap")) }
    }

    /// SetAPen.
    pub fn set_a_pen(&mut self, pen: u8) {
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort.
        unsafe { ((*g).SetAPen)(g, p, pen) };
    }

    /// SetBPen.
    pub fn set_b_pen(&mut self, pen: u8) {
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort.
        unsafe { ((*g).SetBPen)(g, p, pen) };
    }

    /// SetDrMd (JAM1 0, JAM2 1, COMPLEMENT 2, INVERSVID 4).
    pub fn set_draw_mode(&mut self, mode: u8) {
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort.
        unsafe { ((*g).SetDrMd)(g, p, mode & 7) };
    }

    /// SetRast: the whole bitmap to pen `pen`.
    pub fn clear(&mut self, pen: u8) {
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort on a live bitmap.
        unsafe { ((*g).SetRast)(g, p, pen) };
    }

    /// SetRastColor: the whole bitmap to ARGB `color`.
    pub fn clear_color(&mut self, color: u32) {
        let (g, p) = (self.g, self.p());
        // SAFETY: as above.
        unsafe { ((*g).SetRastColor)(g, p, color) };
    }

    /// RectFill with the A pen.
    pub fn fill(&mut self, r: Rect) -> Result<()> {
        self.check(&r)?;
        let [a, b, c, d] = r.raw();
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort and an ordered rectangle inside the bitmap.
        unsafe { ((*g).RectFill)(g, p, a, b, c, d) };
        Ok(())
    }

    /// RectFillColor with ARGB `color`.
    pub fn fill_color(&mut self, r: Rect, color: u32) -> Result<()> {
        self.check(&r)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: as in fill.
        unsafe { ((*g).RectFillColor)(g, p, r.x0 as u32, r.y0 as u32, r.x1 as u32, r.y1 as u32, color) };
        Ok(())
    }

    /// Move + Draw: a line in the A pen between two points of the bitmap.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<()> {
        self.point(x0, y0)?;
        self.point(x1, y1)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort; both ends inside the bitmap, so the line
        // (inside their bounding box) is too.
        unsafe {
            ((*g).Move)(g, p, x0 as i16, y0 as i16);
            ((*g).Draw)(g, p, x1 as i16, y1 as i16);
        }
        Ok(())
    }

    /// DrawEllipse (not clipped on a RastPort without layers: its whole
    /// box must be inside).
    pub fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32) -> Result<()> {
        if rx < 0 || ry < 0 {
            return Err(AmigaError::InvalidArgument("negative radius"));
        }
        self.check(&Rect::new(cx - rx, cy - ry, cx + rx, cy + ry))?;
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort; the ellipse's box is inside the bitmap.
        unsafe { ((*g).DrawEllipse)(g, p, cx as i16, cy as i16, rx as i16, ry as i16) };
        Ok(())
    }

    /// WritePixel in the A pen.
    pub fn write_pixel(&mut self, x: i32, y: i32) -> Result<()> {
        self.point(x, y)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort and a point inside the bitmap.
        let r = unsafe { ((*g).WritePixel)(g, p, x as i16, y as i16) };
        if r == 0 { Ok(()) } else { Err(AmigaError::InvalidArgument("WritePixel failed")) }
    }

    /// ReadPixel: the pen at (x, y) (None when it cannot be read).
    pub fn read_pixel(&mut self, x: i32, y: i32) -> Result<Option<u32>> {
        self.point(x, y)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: as above.
        let v = unsafe { ((*g).ReadPixel)(g, p, x as i16, y as i16) };
        Ok((v >= 0).then_some(v as u32))
    }

    /// WritePixelColor (ARGB).
    pub fn write_color(&mut self, x: i32, y: i32, color: u32) -> Result<()> {
        self.point(x, y)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: as above.
        unsafe { ((*g).WritePixelColor)(g, p, x as u32, y as u32, color) };
        Ok(())
    }

    /// ReadPixelColor (ARGB).
    pub fn read_color(&mut self, x: i32, y: i32) -> Result<u32> {
        self.point(x, y)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: as above.
        Ok(unsafe { ((*g).ReadPixelColor)(g, p, x as u32, y as u32) })
    }

    fn check_array(&self, r: &Rect, fmt: PixFmt, len: usize, bpr: usize) -> Result<()> {
        self.check(r)?;
        let row = r.width() as usize * fmt.bytes_per_pixel();
        if bpr < row || len < bpr * (r.height() as usize - 1) + row || bpr > u32::MAX as usize {
            return Err(AmigaError::InvalidArgument("pixel array too small for the rectangle"));
        }
        Ok(())
    }

    /// ReadPixelArray: the rectangle `r` into `dst` as `fmt`, rows
    /// `bytes_per_row` apart.
    pub fn read_pixels(&mut self, r: Rect, fmt: PixFmt, dst: &mut [u8], bytes_per_row: usize) -> Result<()> {
        self.check_array(&r, fmt, dst.len(), bytes_per_row)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort, a rectangle inside the bitmap and a buffer
        // checked to hold it in this format and pitch.
        unsafe {
            ((*g).ReadPixelArray)(g, p, r.x0 as u32, r.y0 as u32, dst.as_mut_ptr(), 0, 0, bytes_per_row as u32, fmt.code() as usize as APTR, r.width() as u32, r.height() as u32)
        };
        Ok(())
    }

    /// WritePixelArray: `src` (as `fmt`, rows `bytes_per_row` apart) into
    /// the rectangle `r`.
    pub fn write_pixels(&mut self, r: Rect, fmt: PixFmt, src: &[u8], bytes_per_row: usize) -> Result<()> {
        self.check_array(&r, fmt, src.len(), bytes_per_row)?;
        let (g, p) = (self.g, self.p());
        // SAFETY: as in read_pixels; the source is only read (the SDK types
        // it non-const).
        unsafe {
            ((*g).WritePixelArray)(g, src.as_ptr() as *mut u8, 0, 0, bytes_per_row as u32, fmt.code() as usize as APTR, p, r.x0 as u32, r.y0 as u32, r.width() as u32, r.height() as u32)
        };
        Ok(())
    }

    /// SetFont: draw text in `font` (borrowed for the RastPort's life).
    pub fn set_font(&mut self, font: &'a Font) {
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort and a font that outlives it ('a).
        unsafe { ((*g).SetFont)(g, p, font.f) };
    }

    fn font_metric(&self, off: usize) -> u16 {
        // SAFETY: the RastPort's font pointer (at 52): the default font from
        // InitRastPort or one borrowed for 'a; a u16 at its header offset.
        unsafe {
            let f = core::ptr::read_unaligned(self.p_const().add(RP_FONT) as *const *const u8);
            if f.is_null() { 0 } else { core::ptr::read(f.add(off) as *const u16) }
        }
    }

    /// TextLength: the width of `text` in pixels.
    pub fn text_width(&mut self, text: &[u8]) -> Result<u32> {
        if text.len() > u16::MAX as usize {
            return Err(AmigaError::InvalidArgument("text too long"));
        }
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort and `text.len()` readable bytes.
        Ok(unsafe { ((*g).TextLength)(g, p, text.as_ptr(), text.len() as u16) } as u16 as u32)
    }

    /// TextExtent.
    pub fn text_extent(&mut self, text: &[u8]) -> Result<Extent> {
        if text.len() > u16::MAX as usize {
            return Err(AmigaError::InvalidArgument("text too long"));
        }
        let mut te = [0u16; 6];
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort, the text, and a 12-byte TextExtent to fill.
        unsafe { ((*g).TextExtent)(g, p, text.as_ptr(), text.len() as u16, te.as_mut_ptr() as *mut sys::TextExtent) };
        Ok(Extent {
            width: te[0],
            height: te[1],
            bounds: Rect::new(te[2] as i16 as i32, te[3] as i16 as i32, te[4] as i16 as i32, te[5] as i16 as i32),
        })
    }

    /// Text with its top-left corner at (x, y), in the A pen; the whole
    /// line must fit in the bitmap.
    pub fn text(&mut self, x: i32, y: i32, text: &[u8]) -> Result<()> {
        let width = self.text_width(text)? as i32;
        let (ys, base) = (self.font_metric(TF_YSIZE) as i32, self.font_metric(TF_BASELINE) as i32);
        if ys == 0 || width == 0 && !text.is_empty() {
            return Err(AmigaError::InvalidArgument("no font"));
        }
        self.check(&Rect::new(x, y, x + width.max(1) - 1, y + ys - 1))?;
        let (g, p) = (self.g, self.p());
        // SAFETY: our RastPort; the text's box (width by font height from
        // the top) lies inside the bitmap; the bytes are read only.
        unsafe {
            ((*g).Move)(g, p, x as i16, (y + base) as i16);
            ((*g).Text)(g, p, text.as_ptr(), text.len() as u16);
        }
        Ok(())
    }
}

impl Drop for RastPort<'_> {
    fn drop(&mut self) {
        if let Store::Borrowed { rp, old_font } = self.store {
            if !old_font.is_null() {
                // SAFETY: the borrowed RastPort, still valid ('a); its own
                // font goes back before any font of ours can be closed.
                unsafe { ((*self.g).SetFont)(self.g, rp, old_font) };
            }
        }
    }
}

/// A clip region; DisposeRegion on drop.
pub struct Region {
    g: *mut GraphicsIFace,
    r: *mut sys::Region,
    _p: PhantomData<*const ()>,
}

impl Region {
    pub fn new() -> Result<Self> {
        let g = iface()?;
        // SAFETY: an open interface.
        let r = unsafe { ((*g).NewRegion)(g) };
        if r.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(Self { g, r, _p: PhantomData }) }
    }

    fn rect_op(&mut self, r: Rect, f: unsafe extern "C" fn(*mut GraphicsIFace, *mut sys::Region, *mut APTR) -> u32) -> Result<()> {
        if r.x1 < r.x0 || r.y1 < r.y0 {
            return Err(AmigaError::InvalidArgument("reversed rectangle"));
        }
        let mut raw = r.raw();
        // SAFETY: our region and an 8-byte struct Rectangle.
        let ok = unsafe { f(self.g, self.r, raw.as_mut_ptr() as *mut APTR) } != 0;
        if ok { Ok(()) } else { Err(AmigaError::AllocationFailed) }
    }

    /// The region for raw calls (and clip regions), valid while self is.
    pub fn as_ptr(&self) -> *mut sys::Region {
        self.r
    }

    /// OrRectRegion: self |= r.
    pub fn or_rect(&mut self, r: Rect) -> Result<()> {
        // SAFETY: reading the vtable of an open interface.
        let f = unsafe { (*self.g).OrRectRegion };
        self.rect_op(r, f)
    }

    /// XorRectRegion.
    pub fn xor_rect(&mut self, r: Rect) -> Result<()> {
        // SAFETY: as above.
        let f = unsafe { (*self.g).XorRectRegion };
        self.rect_op(r, f)
    }

    /// ClearRectRegion: self -= r.
    pub fn clear_rect(&mut self, r: Rect) -> Result<()> {
        // SAFETY: as above.
        let f = unsafe { (*self.g).ClearRectRegion };
        self.rect_op(r, f)
    }

    /// AndRectRegion: self &= r (cannot fail).
    pub fn and_rect(&mut self, r: Rect) -> Result<()> {
        if r.x1 < r.x0 || r.y1 < r.y0 {
            return Err(AmigaError::InvalidArgument("reversed rectangle"));
        }
        let mut raw = r.raw();
        // SAFETY: our region and a struct Rectangle.
        unsafe { ((*self.g).AndRectRegion)(self.g, self.r, raw.as_mut_ptr() as *mut APTR) };
        Ok(())
    }

    fn region_op(&mut self, other: &Region, f: unsafe extern "C" fn(*mut GraphicsIFace, *mut APTR, *mut sys::Region) -> u32) -> Result<()> {
        // SAFETY: two live regions; the result goes into the second
        // argument (ours), the first is only read.
        let ok = unsafe { f(self.g, other.r as *mut APTR, self.r) } != 0;
        if ok { Ok(()) } else { Err(AmigaError::AllocationFailed) }
    }

    /// self |= other (OrRegionRegion(other, self)).
    pub fn or_region(&mut self, other: &Region) -> Result<()> {
        // SAFETY: reading the vtable of an open interface.
        let f = unsafe { (*self.g).OrRegionRegion };
        self.region_op(other, f)
    }

    /// self &= other.
    pub fn and_region(&mut self, other: &Region) -> Result<()> {
        // SAFETY: as above.
        let f = unsafe { (*self.g).AndRegionRegion };
        self.region_op(other, f)
    }

    /// self ^= other.
    pub fn xor_region(&mut self, other: &Region) -> Result<()> {
        // SAFETY: as above.
        let f = unsafe { (*self.g).XorRegionRegion };
        self.region_op(other, f)
    }

    /// ClearRegion: empty it.
    pub fn clear(&mut self) {
        // SAFETY: our region.
        unsafe { ((*self.g).ClearRegion)(self.g, self.r) };
    }

    /// The bounding box (meaningless when empty).
    pub fn bounds(&self) -> Rect {
        // SAFETY: our region's bounds (struct Rectangle at 0).
        let b = unsafe { core::ptr::read(self.r as *const [i16; 4]) };
        Rect::new(b[0] as i32, b[1] as i32, b[2] as i32, b[3] as i32)
    }

    /// Its rectangles, in absolute coordinates (each RegionRectangle's
    /// bounds are relative to the region's).
    pub fn rects(&self) -> Vec<Rect> {
        let b = self.bounds();
        let mut v = Vec::new();
        // SAFETY: our region's RegionRectangle list (at 8), walked by its
        // Next pointers (at 0), each with bounds at 8; not changed while
        // we hold &self.
        unsafe {
            let mut rr = core::ptr::read((self.r as *const u8).add(RG_RECTS) as *const *const u8);
            while !rr.is_null() && v.len() < 100_000 {
                let r = core::ptr::read(rr.add(RR_BOUNDS) as *const [i16; 4]);
                v.push(Rect::new(b.x0 + r[0] as i32, b.y0 + r[1] as i32, b.x0 + r[2] as i32, b.y0 + r[3] as i32));
                rr = core::ptr::read(rr as *const *const u8);
            }
        }
        v
    }

    /// The area covered (its rectangles do not overlap).
    pub fn area(&self) -> i64 {
        self.rects().iter().map(|r| r.area()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.area() == 0
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: our region, disposed once (no layer holds it: this
        // module installs no clip regions).
        unsafe { ((*self.g).DisposeRegion)(self.g, self.r) };
    }
}
