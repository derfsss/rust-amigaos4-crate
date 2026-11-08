//! ICyberGfx global(s) and convenience wrappers for cybergraphics.library.

use crate::sys::types::*;
use crate::sys::interfaces::cybergfx::*;

#[cfg(target_os = "amigaos")]
extern "C" { pub static ICyberGfx: *mut CyberGfxIFace; }
#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ICyberGfx: *mut CyberGfxIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn cybergfx_is_cyber_mode_id(id: u32) -> u32 { ((*ICyberGfx).IsCyberModeID)(ICyberGfx, id) }
#[inline]
pub unsafe fn cybergfx_best_cmode_idtag_list(tags: *mut TagItem) -> u32 { ((*ICyberGfx).BestCModeIDTagList)(ICyberGfx, tags) }
#[inline]
pub unsafe fn cybergfx_cmode_request_tag_list(req: APTR, tags: *mut TagItem) -> u32 { ((*ICyberGfx).CModeRequestTagList)(ICyberGfx, req, tags) }
#[inline]
pub unsafe fn cybergfx_alloc_cmode_list_tag_list(tags: *mut TagItem) -> *mut List { ((*ICyberGfx).AllocCModeListTagList)(ICyberGfx, tags) }
#[inline]
pub unsafe fn cybergfx_free_cmode_list(list: *mut List) { ((*ICyberGfx).FreeCModeList)(ICyberGfx, list) }
#[inline]
pub unsafe fn cybergfx_scale_pixel_array(
    src: APTR, src_w: u16, src_h: u16, src_mod: u16, rp: *mut RastPort, dst_x: i16, dst_y: i16,
    dst_w: u16, dst_h: u16, fmt: u8,
) -> i32 { ((*ICyberGfx).ScalePixelArray)(ICyberGfx, src, src_w, src_h, src_mod, rp, dst_x, dst_y, dst_w, dst_h, fmt) }
#[inline]
pub unsafe fn cybergfx_get_cyber_map_attr(bm: *mut BitMap, attr: u32) -> u32 { ((*ICyberGfx).GetCyberMapAttr)(ICyberGfx, bm, attr) }
#[inline]
pub unsafe fn cybergfx_get_cyber_idattr(attr: u32, id: u32) -> u32 { ((*ICyberGfx).GetCyberIDAttr)(ICyberGfx, attr, id) }
#[inline]
pub unsafe fn cybergfx_read_rgbpixel(rp: *mut RastPort, x: i16, y: i16) -> u32 { ((*ICyberGfx).ReadRGBPixel)(ICyberGfx, rp, x, y) }
#[inline]
pub unsafe fn cybergfx_write_rgbpixel(rp: *mut RastPort, x: i16, y: i16, argb: u32) -> i32 { ((*ICyberGfx).WriteRGBPixel)(ICyberGfx, rp, x, y, argb) }
#[inline]
pub unsafe fn cybergfx_read_pixel_array(
    dst: APTR, dst_x: i16, dst_y: i16, dst_mod: u16, rp: *mut RastPort, src_x: i16, src_y: i16,
    w: u16, h: u16, fmt: u8,
) -> u32 { ((*ICyberGfx).ReadPixelArray)(ICyberGfx, dst, dst_x, dst_y, dst_mod, rp, src_x, src_y, w, h, fmt) }
#[inline]
pub unsafe fn cybergfx_write_pixel_array(
    src: APTR, src_x: i16, src_y: i16, src_mod: u16, rp: *mut RastPort, dst_x: i16, dst_y: i16,
    w: u16, h: u16, fmt: u8,
) -> u32 { ((*ICyberGfx).WritePixelArray)(ICyberGfx, src, src_x, src_y, src_mod, rp, dst_x, dst_y, w, h, fmt) }
#[inline]
pub unsafe fn cybergfx_move_pixel_array(src_x: i16, src_y: i16, rp: *mut RastPort, dst_x: i16, dst_y: i16, w: u16, h: u16) -> u32 {
    ((*ICyberGfx).MovePixelArray)(ICyberGfx, src_x, src_y, rp, dst_x, dst_y, w, h)
}
#[inline]
pub unsafe fn cybergfx_invert_pixel_array(rp: *mut RastPort, x: i16, y: i16, w: u16, h: u16) -> u32 {
    ((*ICyberGfx).InvertPixelArray)(ICyberGfx, rp, x, y, w, h)
}
#[inline]
pub unsafe fn cybergfx_fill_pixel_array(rp: *mut RastPort, x: i16, y: i16, w: u16, h: u16, argb: u32) -> u32 {
    ((*ICyberGfx).FillPixelArray)(ICyberGfx, rp, x, y, w, h, argb)
}
#[inline]
pub unsafe fn cybergfx_do_cdraw_method_tag_list(hook: *mut Hook, rp: *mut RastPort, tags: *mut TagItem) {
    ((*ICyberGfx).DoCDrawMethodTagList)(ICyberGfx, hook, rp, tags)
}
#[inline]
pub unsafe fn cybergfx_cvideo_ctrl_tag_list(vp: *mut ViewPort, tags: *mut TagItem) {
    ((*ICyberGfx).CVideoCtrlTagList)(ICyberGfx, vp, tags)
}
#[inline]
pub unsafe fn cybergfx_lock_bit_map_tag_list(bm: APTR, tags: *mut TagItem) -> APTR { ((*ICyberGfx).LockBitMapTagList)(ICyberGfx, bm, tags) }
#[inline]
pub unsafe fn cybergfx_un_lock_bit_map(handle: APTR) { ((*ICyberGfx).UnLockBitMap)(ICyberGfx, handle) }
#[inline]
pub unsafe fn cybergfx_un_lock_bit_map_tag_list(handle: APTR, tags: *mut TagItem) { ((*ICyberGfx).UnLockBitMapTagList)(ICyberGfx, handle, tags) }
#[inline]
pub unsafe fn cybergfx_extract_color(rp: *mut RastPort, bm: *mut BitMap, color: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
    ((*ICyberGfx).ExtractColor)(ICyberGfx, rp, bm, color, x, y, w, h)
}
#[inline]
pub unsafe fn cybergfx_write_lutpixel_array(
    src: APTR, src_x: i16, src_y: i16, src_mod: u16, rp: *mut RastPort, ctab: APTR,
    dst_x: i16, dst_y: i16, w: u16, h: u16, fmt: u8,
) -> u32 { ((*ICyberGfx).WriteLUTPixelArray)(ICyberGfx, src, src_x, src_y, src_mod, rp, ctab, dst_x, dst_y, w, h, fmt) }
#[inline]
pub unsafe fn cybergfx_write_pixel_array_alpha(
    src: APTR, src_x: i16, src_y: i16, src_mod: i16, rp: *mut RastPort, dst_x: i16, dst_y: i16,
    w: u16, h: u16, alpha: u32,
) -> u32 { ((*ICyberGfx).WritePixelArrayAlpha)(ICyberGfx, src, src_x, src_y, src_mod, rp, dst_x, dst_y, w, h, alpha) }
#[inline]
pub unsafe fn cybergfx_blt_template_alpha(src: APTR, src_x: i16, src_y: i16, rp: *mut RastPort, dst_x: i16, dst_y: i16, w: u16, h: u16) {
    ((*ICyberGfx).BltTemplateAlpha)(ICyberGfx, src, src_x, src_y, rp, dst_x, dst_y, w, h)
}
