//! imgview: a picture viewer for AmigaOS 4.1, written with the amigaos4
//! crate.
//!
//!   imgview [FILE] [FIT/S] [SMOOTH/S] [PUBSCREEN/K]
//!
//! Shows any picture a datatype can read (PNG, JPEG, GIF, ILBM, BMP,
//! TIFF, ...) in a window on a public screen. Started from Workbench too:
//! as a tool (its icon's tool types FIT, SMOOTH and PUBSCREEN=name) or as
//! a project icon's default tool.
//!
//! - Project > Open... (o): asl's file requester.
//! - Drop a file icon on the window to open it (AppWindow), or send it as
//!   a document through application.library (OpenDoc).
//! - Next / previous picture in the same directory: space or n, backspace
//!   or p (menu Project). The ends beep.
//! - View > Fit to window (f): shrink pictures larger than the window.
//! - View > Zoom in / out / Actual size (+, -, 1): 10% to 800%.
//! - View > Smooth scaling (s): area averaging when shrinking, bilinear
//!   when enlarging (otherwise nearest pixel).
//! - Cursor keys scroll a picture larger than the window (shift: a page).
//! - q, Esc, the close gadget, CTRL-C, or Quit from AmiDock end it.
//!
//! The window's place and size and the fit and smooth settings are kept in
//! ENV:imgview.xml (and ENVARC:).

#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;
use core::panic::PanicInfo;

use amigaos4::allocator::SharedExecAllocator;
use amigaos4::application::{Application, Event as AppEvent, RegisterOptions};
use amigaos4::asl::{Common, FileOptions, FileRequester};
use amigaos4::datatypes::{self, group, DtObject, DtOptions, PixelFormat};
use amigaos4::dos::{self, args::Args, fs};
use amigaos4::error::AmigaError;
use amigaos4::graphics::{PixFmt, Rect};
use amigaos4::intuition::{easy_request, PubScreen};
use amigaos4::menu::{Item, Menu, Strip};
use amigaos4::reaction::{Event, Gadget, Layout, ToolButton, Window, WindowSpec};
use amigaos4::prefs::PrefsDict;
use amigaos4::sys::SIGBREAKF_CTRL_C;
use amigaos4::workbench::WbStartup;

#[global_allocator]
static ALLOCATOR: SharedExecAllocator = SharedExecAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}

const VERSION: &str = "imgview 0.4 (5.11.2026)";
/// Pictures above this many pixels are refused (4 bytes each in memory).
const MAX_PIXELS: u64 = 64 * 1024 * 1024;
/// A scaled copy above this many pixels is not made.
const MAX_SCALED: u64 = 16 * 1024 * 1024;
const BACKGROUND: u32 = 0xFF50_5050;
const PREFS_ENV: &str = "ENV:imgview.xml";
const PREFS_ENVARC: &str = "ENVARC:imgview.xml";
/// The zoom steps, in percent.
const ZOOMS: [u32; 12] = [10, 25, 33, 50, 67, 100, 150, 200, 300, 400, 600, 800];

// Menu ids.
const M_NEXT: u32 = 1;
const M_PREV: u32 = 2;
const M_ABOUT: u32 = 3;
const M_QUIT: u32 = 4;
const M_OPEN: u32 = 5;
const M_FIT: u32 = 10;
const M_SMOOTH: u32 = 11;
const M_ZOOM_IN: u32 = 12;
const M_ZOOM_OUT: u32 = 13;
const M_ACTUAL: u32 = 14;
const M_LOCK: u32 = 15;

// Raw key codes and qualifiers.
const KEY_UP: u16 = 0x4C;
const KEY_DOWN: u16 = 0x4D;
const KEY_RIGHT: u16 = 0x4E;
const KEY_LEFT: u16 = 0x4F;
const QUAL_SHIFT: u16 = 0x0003;
const SCROLL_STEP: i32 = 32;
/// The space gadget the picture is drawn in.
const PIC: u16 = 1;
/// The toolbar under the picture; a click's code is the menu item's id.
const BAR: u16 = 2;

/// A toolbar button for menu item `id`: the AISS image `image` from an
/// Images drawer beside the program, else from the system's toolbar
/// images (TBImages:, where AISS installs), else Filer's (it has the
/// arrows); else `text`. Its help bubble is the menu item's name.
fn tool(id: u32, text: &str, image: &str, menu: &str) -> ToolButton {
    ToolButton::new(id as u16, text)
        .hint(menu)
        .image(&format!("PROGDIR:Images/{}", image))
        .image(&format!("TBImages:{}", image))
        .image(&format!("SYS:Utilities/Filer/images/{}", image))
}

fn say(args: core::fmt::Arguments) {
    if let Ok(mut o) = dos::stdout() {
        let _ = o.write_fmt(args);
        let _ = o.write_str("\n");
    }
}

/// A decoded picture: ARGB, 4 bytes per pixel, rows `w * 4` apart.
struct Picture {
    path: String,
    name: String,
    w: u32,
    h: u32,
    argb: Vec<u8>,
    kind: String,
}

/// Why a file could not be shown, in words.
fn reason(e: &AmigaError, name: &str) -> String {
    match e {
        AmigaError::DosError(c) if *c >= datatypes::DTERROR_UNKNOWN_DATATYPE => {
            datatypes::dt_string(*c as u32).map(|s| s.replace("%s", name)).unwrap_or_else(|| format!("datatypes error {}", c))
        }
        AmigaError::DosError(212) => "not a picture".to_string(),
        AmigaError::DosError(c) => dos::fault(*c),
        other => format!("{}", other),
    }
}

fn split_path(path: &str) -> (&str, &str) {
    let cut = path.rfind(['/', ':']).map(|i| i + 1).unwrap_or(0);
    (&path[..cut], &path[cut..])
}

fn load(path: &str) -> Result<Picture, String> {
    let name = split_path(path).1.to_string();
    let o = DtObject::from_file(path, &DtOptions { group: Some(group::PICTURE) }).map_err(|e| reason(&e, &name))?;
    let h = o.bitmap_header().ok_or("the datatype gave no size")?;
    let (w, ht) = (h.width as u32, h.height as u32);
    if w == 0 || ht == 0 {
        return Err("the picture is empty".into());
    }
    if w as u64 * ht as u64 > MAX_PIXELS {
        return Err(format!("{} x {} is too large (at most {} megapixels)", w, ht, MAX_PIXELS >> 20));
    }
    let mut argb = Vec::new();
    argb.try_reserve_exact(w as usize * ht as usize * 4).map_err(|_| "not enough memory".to_string())?;
    argb.resize(w as usize * ht as usize * 4, 0);
    o.read_pixels(PixelFormat::Argb, &mut argb, w as usize * 4).map_err(|e| format!("reading the pixels: {}", e))?;
    let kind = o.data_type().map(|d| d.name).unwrap_or_default();
    Ok(Picture { path: path.into(), name, w, h: ht, argb, kind })
}

/// `p` scaled to `tw` x `th`: nearest pixel, or smooth (an area average
/// when shrinking, bilinear when enlarging). None without the memory.
fn scale(p: &Picture, tw: u32, th: u32, smooth: bool) -> Option<Vec<u8>> {
    let (w, h) = (p.w as usize, p.h as usize);
    let (tw, th) = (tw as usize, th as usize);
    let src = &p.argb;
    let mut out = Vec::new();
    out.try_reserve_exact(tw * th * 4).ok()?;
    if !smooth {
        for y in 0..th {
            let sy = y * h / th;
            let row = &src[sy * w * 4..][..w * 4];
            for x in 0..tw {
                let sx = x * w / tw;
                out.extend_from_slice(&row[sx * 4..sx * 4 + 4]);
            }
        }
    } else if tw <= w && th <= h {
        // Each target pixel averages the source box it covers.
        let xs: Vec<(usize, usize)> = (0..tw).map(|x| (x * w / tw, ((x + 1) * w / tw).max(x * w / tw + 1))).collect();
        for y in 0..th {
            let (y0, y1) = (y * h / th, ((y + 1) * h / th).max(y * h / th + 1));
            for &(x0, x1) in &xs {
                let mut sum = [0u32; 4];
                for sy in y0..y1 {
                    let row = &src[(sy * w + x0) * 4..(sy * w + x1) * 4];
                    for px in row.chunks_exact(4) {
                        for c in 0..4 {
                            sum[c] += px[c] as u32;
                        }
                    }
                }
                let n = ((y1 - y0) * (x1 - x0)) as u32;
                for s in sum {
                    out.push(((s + n / 2) / n) as u8);
                }
            }
        }
    } else {
        // Bilinear, sampling at pixel centres, 8-bit weights.
        let pos = |t: usize, tn: usize, n: usize| -> (usize, usize, u32) {
            let f = (((2 * t + 1) * n * 256) / (2 * tn)) as i64 - 128;
            let f = f.clamp(0, ((n - 1) * 256) as i64) as usize;
            let i0 = f >> 8;
            (i0, (i0 + 1).min(n - 1), (f & 255) as u32)
        };
        let xs: Vec<(usize, usize, u32)> = (0..tw).map(|x| pos(x, tw, w)).collect();
        for y in 0..th {
            let (y0, y1, wy) = pos(y, th, h);
            let (r0, r1) = (&src[y0 * w * 4..][..w * 4], &src[y1 * w * 4..][..w * 4]);
            for &(x0, x1, wx) in &xs {
                for c in 0..4 {
                    let top = r0[x0 * 4 + c] as u32 * (256 - wx) + r0[x1 * 4 + c] as u32 * wx;
                    let bot = r1[x0 * 4 + c] as u32 * (256 - wx) + r1[x1 * 4 + c] as u32 * wx;
                    out.push(((top * (256 - wy) + bot * wy + (1 << 15)) >> 16) as u8);
                }
            }
        }
    }
    Some(out)
}

/// What the window shows.
struct View {
    pic: Option<Picture>,
    /// Fit to window and Actual size are independent: fit alone scales
    /// to the window both ways; actual alone shows 100%; both show 100%
    /// unless the picture is larger than the window, which it then fits;
    /// neither shows the zoom.
    fit: bool,
    actual: bool,
    smooth: bool,
    /// Percent, when not fitting.
    zoom: u32,
    scroll: (i32, i32),
    /// The scaled copy and the (width, height, smooth) it was made for.
    scaled: Option<((u32, u32, bool), Vec<u8>)>,
}

impl View {
    /// The picture's area: the space gadget's render box (window
    /// coordinates, inclusive); empty while iconified.
    fn area(w: &Window<'_>) -> (i32, i32, i32, i32) {
        w.render_box(PIC).unwrap_or((0, 0, -1, -1))
    }

    fn inner_size(w: &Window<'_>) -> (u32, u32) {
        let (x0, y0, x1, y1) = Self::area(w);
        ((x1 - x0 + 1).max(0) as u32, (y1 - y0 + 1).max(0) as u32)
    }

    /// The size the picture is shown at in an `aw` x `ah` window.
    fn target(&self, aw: u32, ah: u32) -> Option<(u32, u32)> {
        let p = self.pic.as_ref()?;
        if self.fit {
            // The picture scaled to the window, larger or smaller, keeping
            // its proportions (with actual size on: only ever smaller).
            if aw == 0 || ah == 0 || (p.w, p.h) == (aw, ah) || (self.actual && p.w <= aw && p.h <= ah) {
                return Some((p.w, p.h));
            }
            return Some(if p.w as u64 * ah as u64 > p.h as u64 * aw as u64 {
                (aw, ((p.h as u64 * aw as u64) / p.w as u64).max(1) as u32)
            } else {
                (((p.w as u64 * ah as u64) / p.h as u64).max(1) as u32, ah)
            });
        }
        if self.actual {
            return Some((p.w, p.h));
        }
        Some(Self::zoomed(p, self.zoom))
    }

    fn zoomed(p: &Picture, zoom: u32) -> (u32, u32) {
        ((p.w as u64 * zoom as u64 / 100).max(1) as u32, (p.h as u64 * zoom as u64 / 100).max(1) as u32)
    }

    /// The pixels to show: (width, height, data); scaled first if needed.
    fn source(&mut self, win: &Window<'_>, aw: u32, ah: u32) -> Option<(u32, u32, &[u8])> {
        let (tw, th) = self.target(aw, ah)?;
        let p = self.pic.as_ref()?;
        if (tw, th) == (p.w, p.h) {
            return Some((p.w, p.h, p.argb.as_slice()));
        }
        let key = (tw, th, self.smooth);
        if !matches!(&self.scaled, Some((k, _)) if *k == key) {
            win.set_busy(true);
            self.scaled = scale(p, tw, th, self.smooth).map(|d| (key, d));
            win.set_busy(false);
        }
        match &self.scaled {
            Some(((w, h, _), d)) => Some((*w, *h, d.as_slice())),
            None => Some((p.w, p.h, p.argb.as_slice())),
        }
    }

    fn title(&self, aw: u32, ah: u32) -> String {
        match (&self.pic, self.target(aw, ah)) {
            (Some(p), Some((tw, _))) => {
                let pct = (tw as u64 * 100 + p.w as u64 / 2) / p.w as u64;
                let pct = if pct == 100 { String::new() } else { format!(", {}%", pct.max(1)) };
                format!("imgview: {} ({} x {} {}{})", p.name, p.w, p.h, p.kind, pct)
            }
            _ => "imgview: drop a picture here, or Open (Amiga-O)".into(),
        }
    }

    fn draw(&mut self, w: &mut Window<'_>) {
        let (x0, y0, x1, y1) = Self::area(w);
        let (aw, ah) = Self::inner_size(w);
        if aw == 0 || ah == 0 {
            return;
        }
        let scroll = self.scroll;
        let Some((sw, sh, data)) = self.source(w, aw, ah) else {
            if let Ok(mut rp) = w.rastport() {
                let _ = rp.fill_color(Rect::new(x0, y0, x1, y1), BACKGROUND);
                rp.set_a_pen(1);
                let _ = rp.text(x0 + 8, y0 + 20, b"Drop a picture on this window");
            }
            return;
        };
        // Where the picture goes: centred when smaller, scrolled when larger.
        let place = |avail: u32, size: u32, at: i32, scroll: i32| -> (i32, i32, i32) {
            if size <= avail {
                (at + (avail - size) as i32 / 2, 0, size as i32)
            } else {
                (at, scroll.clamp(0, (size - avail) as i32), avail as i32)
            }
        };
        let (ox, sx, dw) = place(aw, sw, x0, scroll.0);
        let (oy, sy, dh) = place(ah, sh, y0, scroll.1);
        let Ok(mut rp) = w.rastport() else { return };
        // The background around the picture.
        let around = [
            Rect::new(x0, y0, x1, oy - 1),
            Rect::new(x0, oy + dh, x1, y1),
            Rect::new(x0, oy, ox - 1, oy + dh - 1),
            Rect::new(ox + dw, oy, x1, oy + dh - 1),
        ];
        for r in around {
            if r.x1 >= r.x0 && r.y1 >= r.y0 {
                let _ = rp.fill_color(r, BACKGROUND);
            }
        }
        let start = (sy as usize * sw as usize + sx as usize) * 4;
        let _ = rp.write_pixels(Rect::new(ox, oy, ox + dw - 1, oy + dh - 1), PixFmt::A8R8G8B8, &data[start..], sw as usize * 4);
        // The scroll position, clamped to the picture.
        self.scroll = (sx, sy);
    }

    fn scroll_by(&mut self, dx: i32, dy: i32) {
        self.scroll = (self.scroll.0.saturating_add(dx).max(0), self.scroll.1.saturating_add(dy).max(0));
    }
}

/// The next (`step` 1) or previous (-1) picture in `path`'s directory, by
/// name; None at the end.
fn neighbour(path: &str, step: i32) -> Option<String> {
    let lock = fs::Lock::new(path, fs::LockMode::Shared).ok()?;
    let name = lock.metadata().ok()?.name;
    let dir = lock.parent().ok()??;
    let dir_path = dir.path().ok()?;
    let mut names: Vec<String> = dir.read_dir().ok()?.filter_map(|e| e.ok()).filter(|m| m.is_file()).map(|m| m.name).collect();
    names.sort_by_key(|n| n.to_ascii_lowercase());
    let at = names.iter().position(|n| *n == name)? as i32;
    let mut k = at + step;
    while k >= 0 && (k as usize) < names.len() {
        let full = dos::add_part(&dir_path, &names[k as usize]).ok()?;
        if matches!(datatypes::examine(&full), Ok(i) if i.group == group::PICTURE) {
            return Some(full);
        }
        k += step;
    }
    None
}

/// Window place and size, and the view settings, from the prefs.
struct Settings {
    bounds: Option<(i32, i32, i32, i32)>,
    fit: bool,
    actual: bool,
    smooth: bool,
    /// The window keeps its size; the picture scrolls in it.
    lock: bool,
}

fn read_settings() -> Settings {
    let mut s = Settings { bounds: None, fit: false, actual: false, smooth: false, lock: false };
    let Ok(mut d) = PrefsDict::new() else { return s };
    if d.read(PREFS_ENV).is_err() {
        return s;
    }
    s.fit = d.get_bool("fit", false);
    s.actual = d.get_bool("actual", false);
    s.lock = d.get_bool("lock", false);
    s.smooth = d.get_bool("smooth", false);
    if let (Some(l), Some(t), Some(w), Some(h)) = (d.get_int("left"), d.get_int("top"), d.get_int("width"), d.get_int("height")) {
        if w >= 80 && h >= 60 {
            s.bounds = Some((l, t, w, h));
        }
    }
    s
}

fn write_settings(bounds: (i32, i32, i32, i32), fit: bool, actual: bool, smooth: bool, lock: bool) {
    let Ok(mut d) = PrefsDict::new() else { return };
    let (l, t, w, h) = bounds;
    let ok = d.set_int("left", l).is_ok()
        && d.set_int("top", t).is_ok()
        && d.set_int("width", w).is_ok()
        && d.set_int("height", h).is_ok()
        && d.set_bool("fit", fit).is_ok()
        && d.set_bool("actual", actual).is_ok()
        && d.set_bool("lock", lock).is_ok()
        && d.set_bool("smooth", smooth).is_ok();
    if ok {
        let _ = d.write(PREFS_ENV);
        let _ = d.write(PREFS_ENVARC);
    }
}

/// Size the window for a `pw` x `ph` view, inside the screen: the picture
/// area's size plus what the window has around it (borders, layout
/// spacing).
/// window.class lays a changed window out later: the main loop redraws
/// the gadgets and the picture when it sees the new picture area.
fn size_for(w: &Window<'_>, screen: (i32, i32), pw: u32, ph: u32) {
    let (Some((l, t, ow0, oh0)), Some((x0, y0, x1, y1))) = (w.bounds(), w.render_box(PIC)) else { return };
    let (ew, eh) = (ow0 - (x1 - x0 + 1), oh0 - (y1 - y0 + 1));
    // Never below the window's least size (the toolbar's width).
    let (mw, mh) = w.min_size().unwrap_or((160, 80));
    let ow = (pw.min(i32::MAX as u32 / 2) as i32 + ew).max(160).max(mw).min(screen.0);
    let oh = (ph.min(i32::MAX as u32 / 2) as i32 + eh).max(80).max(mh).min(screen.1);
    let left = l.min(screen.0 - ow).max(0);
    let top = t.min(screen.1 - oh).max(0);
    if (left, top, ow, oh) != (l, t, ow0, oh0) {
        w.change_box(left, top, ow, oh);
    }
}

struct Viewer<'w, 's> {
    win: &'w mut Window<'s>,
    /// The picture area last drawn into (window.class lays out later than
    /// a resize is asked for: a changed box means draw again).
    drawn: (i32, i32, i32, i32),
    /// The toolbar's Fit, 1:1 and Lock buttons as last set (None: set them).
    pressed: Option<(bool, bool, bool)>,
    /// Lock window size: the window is never resized for the picture.
    lock: bool,
    view: View,
    screen: (i32, i32),
    files: Option<FileRequester>,
    beep: &'w dyn Fn(),
}

impl Viewer<'_, '_> {
    fn show_error(&self, path: &str, why: &str) {
        let _ = easy_request(Some(self.win), "imgview", &format!("Cannot show {}:\n{}", path, why), &["OK"]);
    }

    fn open(&mut self, path: &str) {
        self.win.set_busy(true);
        let r = load(path);
        self.win.set_busy(false);
        match r {
            Ok(p) => {
                if !self.view.fit {
                    let (w, h) = View::zoomed(&p, self.view.zoom);
                    self.size_window(w, h);
                }
                self.view.pic = Some(p);
                self.view.scaled = None;
                self.view.scroll = (0, 0);
                self.refresh();
            }
            Err(why) => self.show_error(path, &why),
        }
    }

    /// Project > Open...: asl's file requester, kept for the next time
    /// (it remembers its drawer and size).
    fn choose(&mut self) {
        if self.files.is_none() {
            match FileRequester::new() {
                Ok(r) => self.files = Some(r),
                Err(e) => return self.show_error("a file requester", &format!("{}", e)),
            }
        }
        let (drawer, file) = match &self.view.pic {
            Some(p) => {
                let (d, f) = split_path(&p.path);
                (Some(d.to_string()), Some(f.to_string()))
            }
            None => (None, None),
        };
        let Some(fr) = self.files.as_mut() else { return };
        let r = fr.request(Some(self.win), &FileOptions {
            common: Common { title: Some("Open a picture"), ..Default::default() },
            drawer: drawer.as_deref(),
            file: file.as_deref(),
            ok_text: Some("Open"),
            reject_icons: true,
            ..Default::default()
        });
        match r {
            Ok(Some(c)) if !c.file.is_empty() => {
                if let Some(p) = c.paths.first() {
                    self.open(p);
                }
            }
            Ok(_) => {}
            Err(e) => self.show_error("a file requester", &format!("{}", e)),
        }
    }

    fn step(&mut self, dir: i32) {
        let Some(cur) = self.view.pic.as_ref().map(|p| p.path.clone()) else { return (self.beep)() };
        match neighbour(&cur, dir) {
            Some(next) => self.open(&next),
            None => (self.beep)(),
        }
    }

    fn set_fit(&mut self, on: bool) {
        self.view.fit = on;
        // The View menu's check mark follows keys and zooming.
        self.win.set_menu_checked(M_FIT, on);
        if !on {
            // Back to the zoom (100% unless zoomed), the window sized for it.
            if let Some(p) = &self.view.pic {
                let (w, h) = View::zoomed(p, self.view.zoom);
                self.view.scroll = (0, 0);
                self.size_window(w, h);
            }
        }
        self.refresh();
    }

    /// Actual size on: 100% (unless fit shrinks a picture larger than the
    /// window), the window sized for it; off: back to the zoom.
    fn set_actual(&mut self, on: bool) {
        self.view.actual = on;
        self.win.set_menu_checked(M_ACTUAL, on);
        if on {
            self.view.zoom = 100;
        }
        if !self.view.fit {
            if let Some(p) = &self.view.pic {
                let (w, h) = View::zoomed(p, self.view.zoom);
                self.view.scroll = (0, 0);
                self.size_window(w, h);
            }
        }
        self.refresh();
    }

    /// Size the window for a `w` x `h` view, unless its size is locked.
    fn size_window(&self, w: u32, h: u32) {
        if !self.lock {
            size_for(self.win, self.screen, w, h);
        }
    }

    /// Lock window size: on, the window keeps its size and the picture
    /// scrolls in it; off, the window is sized for the picture again from
    /// the next zoom, fit, actual size or picture on (turning it off does
    /// not resize the window itself).
    fn set_lock(&mut self, on: bool) {
        self.lock = on;
        self.win.set_menu_checked(M_LOCK, on);
        self.refresh();
    }

    fn set_smooth(&mut self, on: bool) {
        self.view.smooth = on;
        self.win.set_menu_checked(M_SMOOTH, on);
        self.refresh();
    }

    /// Zoom one step in (1) or out (-1), or to 100% (0), from the size
    /// shown now; fit and actual size stop.
    fn zoom(&mut self, dir: i32) {
        let (aw, ah) = View::inner_size(self.win);
        let Some(p) = self.view.pic.as_ref() else { return (self.beep)() };
        let (cw, ch) = self.view.target(aw, ah).unwrap_or((p.w, p.h));
        // The zoom shown now: the step itself when zoomed (working it back
        // from the scaled width truncated 33% of 320 = 105 px to 32%, so
        // "the next step up" was 33% again and zooming in stuck), else the
        // fitted size's percentage, rounded.
        let now = if !self.view.fit && !self.view.actual {
            self.view.zoom
        } else {
            ((cw as u64 * 100 + p.w as u64 / 2) / p.w as u64) as u32
        };
        let next = match dir {
            0 => Some(100),
            d if d > 0 => ZOOMS.iter().copied().find(|&z| z > now),
            _ => ZOOMS.iter().rev().copied().find(|&z| z < now),
        };
        let Some(z) = next else { return (self.beep)() };
        let (nw, nh) = View::zoomed(p, z);
        if nw as u64 * nh as u64 > MAX_SCALED {
            return (self.beep)();
        }
        // Keep the middle of what is shown in the middle.
        let mid = |s: i32, avail: u32, old: u32, new: u32| -> i32 {
            let c = (s as i64 + (avail.min(old) / 2) as i64) * new as i64 / old.max(1) as i64;
            (c - (avail / 2) as i64).clamp(0, i32::MAX as i64) as i32
        };
        self.view.scroll = (mid(self.view.scroll.0, aw, cw, nw), mid(self.view.scroll.1, ah, ch, nh));
        self.view.zoom = z;
        if self.view.fit {
            self.view.fit = false;
            self.win.set_menu_checked(M_FIT, false);
        }
        if self.view.actual {
            self.view.actual = false;
            self.win.set_menu_checked(M_ACTUAL, false);
        }
        self.size_window(nw, nh);
        self.refresh();
    }

    /// The border scrollers: the shown picture's size, the area's size and
    /// the scroll position (in pixels).
    fn update_scrollers(&self) {
        let (aw, ah) = View::inner_size(self.win);
        let (tw, th) = self.view.target(aw, ah).unwrap_or((0, 0));
        self.win.set_scroll(false, tw, aw, self.view.scroll.0.max(0) as u32);
        self.win.set_scroll(true, th, ah, self.view.scroll.1.max(0) as u32);
    }

    fn refresh(&mut self) {
        // Fit and 1:1 stay pressed in while their settings are on.
        let want = (self.view.fit, self.view.actual, self.lock);
        if self.pressed != Some(want) {
            let _ = self.win.set_tool_selected(BAR, M_FIT as u16, want.0);
            let _ = self.win.set_tool_selected(BAR, M_ACTUAL as u16, want.1);
            let _ = self.win.set_tool_selected(BAR, M_LOCK as u16, want.2);
            self.pressed = Some(want);
        }
        self.drawn = View::area(self.win);
        self.view.draw(self.win);
        self.update_scrollers();
        let (aw, ah) = View::inner_size(self.win);
        let _ = self.win.set_title(&self.view.title(aw, ah));
    }
}

/// The View menu's items (in the menu bar and in the context menu).
fn view_menu(title: &str, fit: bool, actual: bool, smooth: bool, lock: bool) -> Menu {
    Menu::new(title)
        .item(Item::toggle(M_FIT, "Fit to window").key("F").checked(fit))
        .item(Item::toggle(M_ACTUAL, "Actual size").key("1").checked(actual))
        .item(Item::toggle(M_LOCK, "Lock window size").key("L").checked(lock))
        .item(Item::toggle(M_SMOOTH, "Smooth scaling").key("S").checked(smooth))
        .separator()
        .item(Item::action(M_ZOOM_IN, "Zoom in").key("+"))
        .item(Item::action(M_ZOOM_OUT, "Zoom out").key("-"))
}

/// The options from the command line, or from the tool's icon.
struct Options {
    file: Option<String>,
    fit: bool,
    smooth: bool,
    pubscreen: Option<String>,
}

fn options(wb: Option<&WbStartup>) -> Result<Options, String> {
    match wb {
        Some(wb) => {
            let mut o = Options { file: wb.files().into_iter().next(), fit: false, smooth: false, pubscreen: None };
            for t in wb.tool_types() {
                let (k, v) = t.split_once('=').unwrap_or((&t, ""));
                match k.trim().to_ascii_uppercase().as_str() {
                    "FIT" => o.fit = true,
                    "SMOOTH" => o.smooth = true,
                    "PUBSCREEN" if !v.trim().is_empty() => o.pubscreen = Some(v.trim().to_string()),
                    _ => {}
                }
            }
            Ok(o)
        }
        None => {
            let a = Args::from_command_line("FILE,FIT/S,SMOOTH/S,PUBSCREEN/K").map_err(|e| format!("{} (usage: imgview [FILE] [FIT] [SMOOTH] [PUBSCREEN=name])", e))?;
            Ok(Options {
                file: a.text("FILE").ok().flatten().map(String::from),
                fit: a.switch("FIT").unwrap_or(false),
                smooth: a.switch("SMOOTH").unwrap_or(false),
                pubscreen: a.text("PUBSCREEN").ok().flatten().map(String::from),
            })
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_main(_args: *const u8, _len: i32) -> i32 {
    // First: from Workbench, the start-up message (replied when `wb` is
    // dropped, last, as rust_main returns).
    let wb = WbStartup::take();
    let from_wb = wb.is_some();
    // Errors before the window go to the Shell, or to a requester when
    // there is no Shell.
    let fail = |msg: &str| -> i32 {
        if from_wb {
            let _ = easy_request(None, "imgview", msg, &["OK"]);
        } else {
            say(format_args!("imgview: {}", msg));
        }
        20
    };
    let opts = match options(wb.as_ref()) {
        Ok(o) => o,
        Err(e) => return fail(&e),
    };
    let mut settings = read_settings();
    settings.fit |= opts.fit;
    settings.smooth |= opts.smooth;
    let screen = match &opts.pubscreen {
        Some(n) => PubScreen::lock(n),
        None => PubScreen::default_screen(),
    };
    let screen = match screen {
        Ok(s) => s,
        Err(e) => return fail(&format!("no public screen: {}", e)),
    };
    let ssize = screen.size();

    // The menus (menuclass): the window's strip, and a context menu shown
    // by the right button over the picture with the same ids.
    let strip = Strip::new()
        .menu(
            Menu::new("Project")
                .item(Item::action(M_OPEN, "Open...").key("O"))
                .separator()
                .item(Item::action(M_NEXT, "Next picture").key("N"))
                .item(Item::action(M_PREV, "Previous picture").key("P"))
                .separator()
                .item(Item::action(M_ABOUT, "About...").key("?"))
                .item(Item::action(M_QUIT, "Quit").key("Q")),
        )
        .menu(view_menu("View", settings.fit, settings.actual, settings.smooth, settings.lock));
    let context = Strip::new().menu(
        view_menu("imgview", settings.fit, settings.actual, settings.smooth, settings.lock)
            .separator()
            .item(Item::action(M_OPEN, "Open..."))
            .item(Item::action(M_NEXT, "Next picture"))
            .item(Item::action(M_PREV, "Previous picture")),
    );

    // A ReAction window (window.class, as tequila builds its own): the
    // picture is drawn in a space gadget, which carries the context menu;
    // the window has the menu strip, an iconify gadget, takes dropped
    // files and puts its version in the screen title.
    // The picture, and under it a toolbar like a player's: the hints are
    // the menu items' names.
    let bar = alloc::vec![
        tool(M_PREV, "<", "nav_west", "Previous picture"),
        tool(M_NEXT, ">", "nav_east", "Next picture"),
        ToolButton::separator(),
        tool(M_ZOOM_OUT, "-", "zoom_out", "Zoom out"),
        tool(M_ZOOM_IN, "+", "zoom_in", "Zoom in"),
        tool(M_ACTUAL, "1:1", "zoom_100", "Actual size").toggle(settings.actual),
        tool(M_FIT, "Fit", "fit", "Fit to window").toggle(settings.fit),
        tool(M_LOCK, "Lock", "lock", "Lock window size").toggle(settings.lock),
        ToolButton::separator(),
        tool(M_OPEN, "Open", "open", "Open..."),
    ];
    let root = Layout::vertical()
        .child(Gadget::space(PIC).min_size(160, 100).context_menu(context))
        .child(Gadget::toolbar(BAR, bar).weight(0));
    let spec = WindowSpec {
        title: "imgview",
        screen_title: Some(VERSION),
        position: settings.bounds.map(|b| (b.0, b.1)),
        centre_on_mouse: true,
        menu: Some(&strip),
        iconify: true,
        drops: true,
        scrollers: true,
        ..Default::default()
    };
    let mut win = match Window::open(&screen, &spec, root) {
        Ok(w) => w,
        Err(e) => return fail(&format!("cannot open a window: {}", e)),
    };
    // The saved outer size.
    if let Some((left, top, width, height)) = settings.bounds {
        win.change_box(left, top, width.min(ssize.0), height.min(ssize.1));
    }

    let app = Application::register("imgview", &RegisterOptions { url_identifier: Some("amigaos4.rust"), description: Some("Picture viewer written in Rust"), ..Default::default() }).ok();
    let beep = || screen.beep();
    let appmask = app.as_ref().map(|a| a.signal_mask()).unwrap_or(0);
    let winmask = win.signal_mask();
    let started = match &wb {
        None => "Started from a Shell.".to_string(),
        Some(w) => format!("Started from Workbench; code freed by {}.", if w.code_handed_to_dos() { "DOS" } else { "Workbench" }),
    };

    let mut v = Viewer {
        win: &mut win,
        drawn: (0, 0, -1, -1),
        pressed: None,
        lock: settings.lock,
        view: View { pic: None, fit: settings.fit, actual: settings.actual, smooth: settings.smooth, zoom: 100, scroll: (0, 0), scaled: None },
        screen: ssize,
        files: None,
        beep: &beep,
    };
    // window.class lays the window out inside WM_HANDLEINPUT: like tequila,
    // draw only once the window's events are handled, so the picture's
    // area has its laid-out size (the first picture too).
    let mut redraw = true;
    let mut pending = opts.file.clone();
    let mut quit = false;
    let mut got = 0u32;
    loop {
        if got & SIGBREAKF_CTRL_C != 0 {
            quit = true;
        }
        while let Some(e) = v.win.next() {
            match e {
                Event::Close => quit = true,
                // The window is back, or has a new size: draw it again.
                Event::NewSize | Event::Uniconified => redraw = true,
                Event::Dropped => {
                    // A picture dropped on the window: bring the window
                    // forward and make it the active one, then show it.
                    v.win.to_front();
                    if let Some(f) = v.win.dropped().first() {
                        v.open(f);
                    }
                }
                Event::Key(k) => match k {
                    b'q' | b'Q' | 27 => quit = true,
                    b' ' | b'n' | b'N' => v.step(1),
                    8 | b'p' | b'P' => v.step(-1),
                    b'o' | b'O' => v.choose(),
                    b'f' | b'F' => v.set_fit(!v.view.fit),
                    b's' | b'S' => v.set_smooth(!v.view.smooth),
                    b'+' | b'=' => v.zoom(1),
                    b'-' => v.zoom(-1),
                    b'1' => v.set_actual(!v.view.actual),
                    b'l' | b'L' => {
                        let on = !v.lock;
                        v.set_lock(on);
                    }
                    _ => {}
                },
                Event::RawKey { code, qualifier } if code & 0x80 == 0 => {
                    let (aw, ah) = View::inner_size(v.win);
                    let (px, py) = if qualifier & QUAL_SHIFT != 0 { (aw as i32, ah as i32) } else { (SCROLL_STEP, SCROLL_STEP) };
                    let moved = match code {
                        KEY_UP => Some((0, -py)),
                        KEY_DOWN => Some((0, py)),
                        KEY_LEFT => Some((-px, 0)),
                        KEY_RIGHT => Some((px, 0)),
                        _ => None,
                    };
                    if let Some((dx, dy)) = moved {
                        v.view.scroll_by(dx, dy);
                        v.view.draw(v.win);
                        v.update_scrollers();
                    }
                }
                // A border scroller moved: show that part of the picture.
                Event::Scroll { vertical, top } => {
                    if vertical {
                        v.view.scroll.1 = top as i32;
                    } else {
                        v.view.scroll.0 = top as i32;
                    }
                    v.view.draw(v.win);
                }
                Event::Gadget { id: BAR, code } => {
                    // A click flips a toggle button itself: set both
                    // again from the view after it.
                    v.pressed = None;
                    match code as u32 {
                    M_OPEN => v.choose(),
                    M_NEXT => v.step(1),
                    M_PREV => v.step(-1),
                    M_FIT => {
                        let on = !v.view.fit;
                        v.set_fit(on);
                    }
                    M_ACTUAL => {
                        let on = !v.view.actual;
                        v.set_actual(on);
                    }
                    M_LOCK => {
                        let on = !v.lock;
                        v.set_lock(on);
                    }
                    M_ZOOM_IN => v.zoom(1),
                    M_ZOOM_OUT => v.zoom(-1),
                    _ => {}
                    }
                    v.refresh();
                }
                Event::Menu { id, checked, .. } => match id {
                    M_OPEN => v.choose(),
                    M_NEXT => v.step(1),
                    M_PREV => v.step(-1),
                    M_ABOUT => {
                        let _ = easy_request(Some(v.win), "About imgview", &format!("{}\n\nA picture viewer written in Rust\nwith the amigaos4 crate.\nPictures are read by datatypes.\n\n{}", VERSION, started), &["OK"]);
                    }
                    M_QUIT => quit = true,
                    // A pick ticks the menu it came from; set_* ticks the
                    // other (the strip or the context menu) too.
                    M_FIT => v.set_fit(checked),
                    M_SMOOTH => v.set_smooth(checked),
                    M_ZOOM_IN => v.zoom(1),
                    M_ZOOM_OUT => v.zoom(-1),
                    M_ACTUAL => v.set_actual(checked),
                    M_LOCK => v.set_lock(checked),
                    _ => {}
                },
                _ => {}
            }
        }
        if let Some(a) = &app {
            while let Some((_, e)) = a.next() {
                match e {
                    AppEvent::Quit | AppEvent::ForceQuit => quit = true,
                    AppEvent::ToFront => {
                        v.win.uniconify();
                        v.win.to_front();
                        v.refresh();
                    }
                    AppEvent::OpenDoc(path) => v.open(&path),
                    _ => {}
                }
            }
        }
        if let Some(f) = pending.take() {
            // open() draws, and a resize it asks for brings a NewSize.
            v.open(&f);
        } else if redraw || View::area(v.win) != v.drawn {
            // The window's new layout is done: its gadgets moved (a
            // shrinking window gets no refresh for them), then the picture.
            // (A draw made before the layout may have used the old picture
            // area; this puts both right. Waiting for the layout instead hung
            // when Intuition left the size as it was: no layout came.)
            if View::area(v.win) != v.drawn {
                v.win.redraw_gadgets();
            }
            v.refresh();
        }
        redraw = false;
        if quit {
            break;
        }
        got = amigaos4::signal::wait(winmask | appmask | SIGBREAKF_CTRL_C);
    }
    let (fit, actual, smooth, lock) = (v.view.fit, v.view.actual, v.view.smooth, v.lock);
    drop(v);
    // While iconified the window has no box: keep the one saved before.
    if let Some(b) = win.bounds().or(settings.bounds) {
        write_settings(b, fit, actual, smooth, lock);
    }
    drop(win);
    0
}
