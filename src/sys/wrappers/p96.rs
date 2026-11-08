//! IP96 global(s) and convenience wrappers for Picasso96API.library.

use crate::sys::types::*;
use crate::sys::interfaces::p96::*;

#[cfg(target_os = "amigaos")]
extern "C" { pub static IP96: *mut P96IFace; }
#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IP96: *mut P96IFace = core::ptr::null_mut();

#[inline]
pub unsafe fn p96_p96_alloc_bit_map(w: u32, h: u32, depth: u32, flags: u32, friend: *mut BitMap, ptr: APTR) -> *mut BitMap {
    ((*IP96).p96AllocBitMap)(IP96, w, h, depth, flags, friend, ptr)
}
#[inline]
pub unsafe fn p96_p96_free_bit_map(bm: *mut BitMap) { ((*IP96).p96FreeBitMap)(IP96, bm) }
#[inline]
pub unsafe fn p96_p96_get_bit_map_attr(bm: *mut BitMap, attr: u32) -> u32 { ((*IP96).p96GetBitMapAttr)(IP96, bm, attr) }
#[inline]
pub unsafe fn p96_p96_lock_bit_map(bm: *mut BitMap, info: *mut u8, len: u32) -> i32 { ((*IP96).p96LockBitMap)(IP96, bm, info, len) }
#[inline]
pub unsafe fn p96_p96_unlock_bit_map(bm: *mut BitMap, lock: i32) { ((*IP96).p96UnlockBitMap)(IP96, bm, lock) }
#[inline]
pub unsafe fn p96_p96_best_mode_idtag_list(tags: *mut TagItem) -> u32 { ((*IP96).p96BestModeIDTagList)(IP96, tags) }
#[inline]
pub unsafe fn p96_p96_request_mode_idtag_list(tags: *mut TagItem) -> u32 { ((*IP96).p96RequestModeIDTagList)(IP96, tags) }
#[inline]
pub unsafe fn p96_p96_alloc_mode_list_tag_list(tags: *mut TagItem) -> *mut List { ((*IP96).p96AllocModeListTagList)(IP96, tags) }
#[inline]
pub unsafe fn p96_p96_free_mode_list(list: *mut List) { ((*IP96).p96FreeModeList)(IP96, list) }
#[inline]
pub unsafe fn p96_p96_get_mode_idattr(id: u32, attr: u32) -> u32 { ((*IP96).p96GetModeIDAttr)(IP96, id, attr) }
#[inline]
pub unsafe fn p96_p96_open_screen_tag_list(tags: *mut TagItem) -> *mut Screen { ((*IP96).p96OpenScreenTagList)(IP96, tags) }
#[inline]
pub unsafe fn p96_p96_close_screen(screen: *mut Screen) -> u32 { ((*IP96).p96CloseScreen)(IP96, screen) }
#[inline]
pub unsafe fn p96_p96_write_pixel_array(info: *mut RenderInfo, sx: u16, sy: u16, rp: *mut RastPort, dx: u16, dy: u16, w: u16, h: u16) {
    ((*IP96).p96WritePixelArray)(IP96, info, sx, sy, rp, dx, dy, w, h)
}
#[inline]
pub unsafe fn p96_p96_read_pixel_array(info: *mut RenderInfo, sx: u16, sy: u16, rp: *mut RastPort, dx: u16, dy: u16, w: u16, h: u16) {
    ((*IP96).p96ReadPixelArray)(IP96, info, sx, sy, rp, dx, dy, w, h)
}
#[inline]
pub unsafe fn p96_p96_write_pixel(rp: *mut RastPort, x: u16, y: u16, argb: u32) -> u32 { ((*IP96).p96WritePixel)(IP96, rp, x, y, argb) }
#[inline]
pub unsafe fn p96_p96_read_pixel(rp: *mut RastPort, x: u16, y: u16) -> u32 { ((*IP96).p96ReadPixel)(IP96, rp, x, y) }
#[inline]
pub unsafe fn p96_p96_rect_fill(rp: *mut RastPort, x: u16, y: u16, w: u16, h: u16, argb: u32) { ((*IP96).p96RectFill)(IP96, rp, x, y, w, h, argb) }
#[inline]
pub unsafe fn p96_p96_write_true_color_data(info: *mut TrueColorInfo, sx: u16, sy: u16, rp: *mut RastPort, dx: u16, dy: u16, w: u16, h: u16) {
    ((*IP96).p96WriteTrueColorData)(IP96, info, sx, sy, rp, dx, dy, w, h)
}
#[inline]
pub unsafe fn p96_p96_read_true_color_data(info: *mut TrueColorInfo, sx: u16, sy: u16, rp: *mut RastPort, dx: u16, dy: u16, w: u16, h: u16) {
    ((*IP96).p96ReadTrueColorData)(IP96, info, sx, sy, rp, dx, dy, w, h)
}
#[inline]
pub unsafe fn p96_p96_pip_open_tag_list(tags: *mut TagItem) -> *mut Window { ((*IP96).p96PIP_OpenTagList)(IP96, tags) }
#[inline]
pub unsafe fn p96_p96_pip_close(win: *mut Window) -> u32 { ((*IP96).p96PIP_Close)(IP96, win) }
#[inline]
pub unsafe fn p96_p96_pip_set_tag_list(win: *mut Window, tags: *mut TagItem) -> i32 { ((*IP96).p96PIP_SetTagList)(IP96, win, tags) }
#[inline]
pub unsafe fn p96_p96_pip_get_tag_list(win: *mut Window, tags: *mut TagItem) -> i32 { ((*IP96).p96PIP_GetTagList)(IP96, win, tags) }
#[inline]
pub unsafe fn p96_p96_pip_get_imsg(port: *mut MsgPort) -> *mut IntuiMessage { ((*IP96).p96PIP_GetIMsg)(IP96, port) }
#[inline]
pub unsafe fn p96_p96_pip_reply_imsg(msg: *mut IntuiMessage) { ((*IP96).p96PIP_ReplyIMsg)(IP96, msg) }
#[inline]
pub unsafe fn p96_p96_get_rtgdata_tag_list(tags: *mut TagItem) -> i32 { ((*IP96).p96GetRTGDataTagList)(IP96, tags) }
#[inline]
pub unsafe fn p96_p96_get_board_data_tag_list(board: u32, tags: *mut TagItem) -> i32 { ((*IP96).p96GetBoardDataTagList)(IP96, board, tags) }
#[inline]
pub unsafe fn p96_p96_encode_color(palette: APTR, color: u32) -> u32 { ((*IP96).p96EncodeColor)(IP96, palette, color) }
#[inline]
pub unsafe fn p96_p96_lock_bit_map_to_board(bm: *mut BitMap, board: u32, info: *mut u8, len: u32) -> u32 {
    ((*IP96).p96LockBitMapToBoard)(IP96, bm, board, info, len)
}
#[inline]
pub unsafe fn p96_p96_unlock_bit_map_from_board(bm: *mut BitMap, lock: u32) { ((*IP96).p96UnlockBitMapFromBoard)(IP96, bm, lock) }
