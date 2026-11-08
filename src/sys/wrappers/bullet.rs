//! IBullet global(s) and convenience wrappers.
//!
//! Hand-written binding for the outline-font scaling engine that
//! diskfont.library uses for TrueType-style rendering. Engines manage
//! glyph caches per-typeface; SetInfo/ObtainInfo/ReleaseInfo configure
//! the engine for a given font.

use crate::sys::types::*;
use crate::sys::interfaces::bullet::*;

// ---- IBullet (BulletIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IBullet: *mut BulletIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IBullet: *mut BulletIFace = core::ptr::null_mut();

// ── Engine lifecycle ─────────────────────────────────────────

#[inline]
pub unsafe fn bullet_open_engine() -> *mut GlyphEngine {
    ((*IBullet).OpenEngine)(IBullet)
}

#[inline]
pub unsafe fn bullet_close_engine(engine: *mut GlyphEngine) {
    ((*IBullet).CloseEngine)(IBullet, engine)
}

// ── Engine info (set / obtain / release) ─────────────────────

#[inline]
pub unsafe fn bullet_set_info_a(engine: *mut GlyphEngine, tag_list: *const TagItem) -> u32 {
    ((*IBullet).SetInfoA)(IBullet, engine, tag_list)
}

#[inline]
pub unsafe fn bullet_obtain_info_a(engine: *mut GlyphEngine, tag_list: *const TagItem) -> u32 {
    ((*IBullet).ObtainInfoA)(IBullet, engine, tag_list)
}

#[inline]
pub unsafe fn bullet_release_info_a(engine: *mut GlyphEngine, tag_list: *const TagItem) -> u32 {
    ((*IBullet).ReleaseInfoA)(IBullet, engine, tag_list)
}

// ── Glyph rendering ──────────────────────────────────────────

#[inline]
pub unsafe fn bullet_get_glyph_map(engine: *mut GlyphEngine, glyph_code: u32, map: *mut *mut GlyphMap) -> u32 {
    ((*IBullet).GetGlyphMap)(IBullet, engine, glyph_code, map)
}
