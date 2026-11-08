//! Layers on a bitmap of the program's own, safe (feature `layers`, with
//! `graphics`).
//!
//! - [`LayerSet`]: NewLayerInfo over a borrowed [`BitMap`], its bounds set
//!   to the bitmap; DisposeLayerInfo on drop.
//! - [`Layer`]: CreateLayerA in front of or behind the others (smart
//!   refresh: what another layer covers is kept and comes back), moved,
//!   sized and restacked inside the bitmap, drawn into through
//!   [`Layer::rastport`] in its own coordinates; DeleteLayer on drop.
//!
//! A window's clip region is set with `intuition::Window::clip`.

use core::marker::PhantomData;

use crate::error::{AmigaError, Result};
use crate::graphics::{BitMap, Rect, RastPort};
use crate::sys::{self, LayersIFace, TagItem, TAG_DONE, TAG_USER};

crate::iface::iface!(iface, LayersIFace, ILayers, b"layers.library\0", b"main\0", 50);

#[cfg_attr(not(feature = "intuition"), allow(dead_code))]
pub(crate) fn layers_iface() -> Result<*mut LayersIFace> {
    iface()
}

// LAYA_* tags (LAYERS_DUMMY = TAG_USER).
const LAYA_MIN_X: u32 = TAG_USER + 30;
const LAYA_MIN_Y: u32 = TAG_USER + 31;
const LAYA_MAX_X: u32 = TAG_USER + 32;
const LAYA_MAX_Y: u32 = TAG_USER + 33;
const LAYA_BITMAP: u32 = TAG_USER + 37;
const LAYA_FLAGS: u32 = TAG_USER + 43;
const LAYA_BEHIND: u32 = TAG_USER + 45;
const LAYERSMART: u32 = 2;
/// struct Layer: rp at 12, bounds (struct Rectangle) at 16.
const LAYER_RP: usize = 12;
const LAYER_BOUNDS: usize = 16;

fn rd16(p: *const u8, off: usize) -> i32 {
    // SAFETY: callers pass a live Layer and an offset from its header
    // (tests/layout/intuition.c).
    unsafe { core::ptr::read_unaligned(p.add(off) as *const i16) as i32 }
}

/// Where a new layer goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Front,
    Back,
}

/// A set of layers on a bitmap; DisposeLayerInfo on drop.
pub struct LayerSet<'b> {
    l: *mut LayersIFace,
    li: *mut sys::Layer_Info,
    bm: *mut sys::BitMap,
    w: i32,
    h: i32,
    _b: PhantomData<&'b mut BitMap>,
}

impl<'b> LayerSet<'b> {
    /// NewLayerInfo for `bitmap`, its bounds the bitmap's.
    pub fn new(bitmap: &'b mut BitMap) -> Result<Self> {
        let l = iface()?;
        let (w, h) = (bitmap.width() as i32, bitmap.height() as i32);
        // SAFETY: an open interface.
        let li = unsafe { ((*l).NewLayerInfo)(l) };
        if li.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        let s = Self { l, li, bm: bitmap.as_ptr(), w, h, _b: PhantomData };
        let mut r: [i16; 4] = [0, 0, (w - 1) as i16, (h - 1) as i16];
        // SAFETY: our layer info and an 8-byte struct Rectangle.
        if unsafe { ((*l).SetLayerInfoBounds)(l, li, r.as_mut_ptr() as *mut sys::Rectangle) } == 0 {
            return Err(AmigaError::InvalidArgument("SetLayerInfoBounds refused the bitmap's size"));
        }
        Ok(s)
    }

    fn inside(&self, r: &Rect) -> bool {
        r.x0 >= 0 && r.y0 >= 0 && r.x1 < self.w && r.y1 < self.h && r.x0 <= r.x1 && r.y0 <= r.y1
    }

    /// CreateLayerA: a smart-refresh layer covering `r` (inclusive),
    /// inside the bitmap.
    pub fn create(&self, r: Rect, place: Place) -> Result<Layer<'_>> {
        if !self.inside(&r) {
            return Err(AmigaError::InvalidArgument("layer outside the bitmap or reversed"));
        }
        let tags = [
            TagItem { ti_Tag: LAYA_MIN_X, ti_Data: r.x0 as u32 },
            TagItem { ti_Tag: LAYA_MIN_Y, ti_Data: r.y0 as u32 },
            TagItem { ti_Tag: LAYA_MAX_X, ti_Data: r.x1 as u32 },
            TagItem { ti_Tag: LAYA_MAX_Y, ti_Data: r.y1 as u32 },
            TagItem { ti_Tag: LAYA_BITMAP, ti_Data: self.bm as u32 },
            TagItem { ti_Tag: LAYA_FLAGS, ti_Data: LAYERSMART },
            TagItem { ti_Tag: LAYA_BEHIND, ti_Data: (place == Place::Back) as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: our layer info, a TAG_DONE-ended list naming our bitmap
        // and a box inside it.
        let layer = unsafe { ((*self.l).CreateLayerA)(self.l, self.li, tags.as_ptr() as *mut TagItem) };
        if layer.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Layer { l: self.l, li: self.li, layer, w: self.w, h: self.h, _s: PhantomData })
    }

    /// The colour at (x, y) of the bitmap itself (what the layers show).
    pub fn read_color(&self, x: i32, y: i32) -> Result<u32> {
        // SAFETY: our borrowed bitmap, `w` by `h`, alive while self is; the
        // RastPort lives only for this call.
        let mut rp = unsafe { RastPort::on_raw(self.bm, self.w, self.h) }?;
        rp.read_color(x, y)
    }
}

impl Drop for LayerSet<'_> {
    fn drop(&mut self) {
        // SAFETY: our layer info, disposed once; its layers borrow self, so
        // they are deleted already.
        unsafe { ((*self.l).DisposeLayerInfo)(self.l, self.li) };
    }
}

/// A layer of a [`LayerSet`]; DeleteLayer on drop.
pub struct Layer<'s> {
    l: *mut LayersIFace,
    li: *mut sys::Layer_Info,
    layer: *mut sys::Layer,
    w: i32,
    h: i32,
    _s: PhantomData<&'s ()>,
}

impl Layer<'_> {
    fn b(&self) -> *const u8 {
        self.layer as *const u8
    }

    /// Where the layer is in the bitmap (inclusive).
    pub fn bounds(&self) -> Rect {
        Rect::new(rd16(self.b(), LAYER_BOUNDS), rd16(self.b(), LAYER_BOUNDS + 2), rd16(self.b(), LAYER_BOUNDS + 4), rd16(self.b(), LAYER_BOUNDS + 6))
    }

    fn fits(&self, r: &Rect) -> Result<()> {
        if r.x0 >= 0 && r.y0 >= 0 && r.x1 < self.w && r.y1 < self.h && r.x0 <= r.x1 && r.y0 <= r.y1 {
            Ok(())
        } else {
            Err(AmigaError::InvalidArgument("the layer would leave the bitmap"))
        }
    }

    /// MoveLayer by (dx, dy), staying inside the bitmap.
    pub fn move_by(&mut self, dx: i32, dy: i32) -> Result<()> {
        let b = self.bounds();
        self.fits(&Rect::new(b.x0 + dx, b.y0 + dy, b.x1 + dx, b.y1 + dy))?;
        // SAFETY: our layer.
        let ok = unsafe { ((*self.l).MoveLayer)(self.l, 0, self.layer, dx, dy) } != 0;
        if ok { Ok(()) } else { Err(AmigaError::AllocationFailed) }
    }

    /// SizeLayer by (dw, dh), staying inside the bitmap and at least 1x1.
    pub fn size_by(&mut self, dw: i32, dh: i32) -> Result<()> {
        let b = self.bounds();
        self.fits(&Rect::new(b.x0, b.y0, b.x1 + dw, b.y1 + dh))?;
        // SAFETY: our layer.
        let ok = unsafe { ((*self.l).SizeLayer)(self.l, 0, self.layer, dw, dh) } != 0;
        if ok { Ok(()) } else { Err(AmigaError::AllocationFailed) }
    }

    /// UpfrontLayer.
    pub fn to_front(&mut self) -> Result<()> {
        // SAFETY: our layer.
        let ok = unsafe { ((*self.l).UpfrontLayer)(self.l, 0, self.layer) } != 0;
        if ok { Ok(()) } else { Err(AmigaError::AllocationFailed) }
    }

    /// BehindLayer.
    pub fn to_back(&mut self) -> Result<()> {
        // SAFETY: our layer.
        let ok = unsafe { ((*self.l).BehindLayer)(self.l, 0, self.layer) } != 0;
        if ok { Ok(()) } else { Err(AmigaError::AllocationFailed) }
    }

    /// WhichLayer: whether this layer is the top one at (x, y) of the
    /// bitmap.
    pub fn is_top_at(&self, x: i32, y: i32) -> bool {
        // SAFETY: our layer info; the result is only compared.
        let top = unsafe { ((*self.l).WhichLayer)(self.l, self.li, x as i16, y as i16) };
        top == self.layer
    }

    /// The layer's RastPort in its own coordinates (0,0 is its top left),
    /// limited to its size; what other layers cover is kept for later.
    pub fn rastport(&mut self) -> Result<RastPort<'_>> {
        let rp: *mut sys::RastPort = {
            // SAFETY: our layer's rp field (tests/layout/intuition.c).
            unsafe { core::ptr::read_unaligned(self.b().add(LAYER_RP) as *const *mut sys::RastPort) }
        };
        let b = self.bounds();
        // SAFETY: our layer's RastPort, valid while the layer is (the
        // result borrows self mutably).
        unsafe { RastPort::borrow_raw(rp, b.width(), b.height()) }
    }
}

impl Drop for Layer<'_> {
    fn drop(&mut self) {
        // SAFETY: our layer, deleted once; no RastPort borrow is left.
        unsafe { ((*self.l).DeleteLayer)(self.l, 0, self.layer) };
    }
}
