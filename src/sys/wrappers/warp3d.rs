//! IWarp3D global(s) and convenience wrappers for warp3d.library.

use crate::sys::types::*;
use crate::sys::interfaces::warp3d::*;

#[cfg(target_os = "amigaos")]
extern "C" { pub static IWarp3D: *mut Warp3DIFace; }
#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IWarp3D: *mut Warp3DIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn warp3d_w3_d_create_context(err: *mut u32, tags: *mut TagItem) -> *mut APTR {
    ((*IWarp3D).W3D_CreateContext)(IWarp3D, err, tags)
}
#[inline]
pub unsafe fn warp3d_w3_d_destroy_context(ctx: *mut APTR) {
    ((*IWarp3D).W3D_DestroyContext)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_get_state(ctx: *mut APTR, state: u32) -> u32 {
    ((*IWarp3D).W3D_GetState)(IWarp3D, ctx, state)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_state(ctx: *mut APTR, state: u32, action: u32) -> u32 {
    ((*IWarp3D).W3D_SetState)(IWarp3D, ctx, state, action)
}
#[inline]
pub unsafe fn warp3d_w3_d_check_driver() -> u32 { ((*IWarp3D).W3D_CheckDriver)(IWarp3D) }
#[inline]
pub unsafe fn warp3d_w3_d_lock_hardware(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_LockHardware)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_un_lock_hardware(ctx: *mut APTR) {
    ((*IWarp3D).W3D_UnLockHardware)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_wait_idle(ctx: *mut APTR) { ((*IWarp3D).W3D_WaitIdle)(IWarp3D, ctx) }
#[inline]
pub unsafe fn warp3d_w3_d_check_idle(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_CheckIdle)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_query(ctx: *mut APTR, query: u32, dest_fmt: u32) -> u32 {
    ((*IWarp3D).W3D_Query)(IWarp3D, ctx, query, dest_fmt)
}
#[inline]
pub unsafe fn warp3d_w3_d_get_tex_fmt_info(ctx: *mut APTR, fmt: u32, dest_fmt: u32) -> u32 {
    ((*IWarp3D).W3D_GetTexFmtInfo)(IWarp3D, ctx, fmt, dest_fmt)
}
#[inline]
pub unsafe fn warp3d_w3_d_alloc_tex_obj(ctx: *mut APTR, err: *mut u32, tags: *mut TagItem) -> *mut APTR {
    ((*IWarp3D).W3D_AllocTexObj)(IWarp3D, ctx, err, tags)
}
#[inline]
pub unsafe fn warp3d_w3_d_free_tex_obj(ctx: *mut APTR, tex: *mut APTR) {
    ((*IWarp3D).W3D_FreeTexObj)(IWarp3D, ctx, tex)
}
#[inline]
pub unsafe fn warp3d_w3_d_release_texture(ctx: *mut APTR, tex: *mut APTR) {
    ((*IWarp3D).W3D_ReleaseTexture)(IWarp3D, ctx, tex)
}
#[inline]
pub unsafe fn warp3d_w3_d_flush_textures(ctx: *mut APTR) {
    ((*IWarp3D).W3D_FlushTextures)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_filter(ctx: *mut APTR, tex: *mut APTR, min: u32, mag: u32) -> u32 {
    ((*IWarp3D).W3D_SetFilter)(IWarp3D, ctx, tex, min, mag)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_tex_env(ctx: *mut APTR, tex: *mut APTR, mode: u32, color: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_SetTexEnv)(IWarp3D, ctx, tex, mode, color)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_wrap_mode(ctx: *mut APTR, tex: *mut APTR, s: u32, t: u32, color: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_SetWrapMode)(IWarp3D, ctx, tex, s, t, color)
}
#[inline]
pub unsafe fn warp3d_w3_d_update_tex_image(ctx: *mut APTR, tex: *mut APTR, image: *mut (), level: i32, pal: *mut u32) -> u32 {
    ((*IWarp3D).W3D_UpdateTexImage)(IWarp3D, ctx, tex, image, level, pal)
}
#[inline]
pub unsafe fn warp3d_w3_d_upload_texture(ctx: *mut APTR, tex: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_UploadTexture)(IWarp3D, ctx, tex)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_line(ctx: *mut APTR, line: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawLine)(IWarp3D, ctx, line)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_point(ctx: *mut APTR, point: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawPoint)(IWarp3D, ctx, point)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_triangle(ctx: *mut APTR, tri: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawTriangle)(IWarp3D, ctx, tri)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_tri_fan(ctx: *mut APTR, fan: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawTriFan)(IWarp3D, ctx, fan)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_tri_strip(ctx: *mut APTR, strip: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawTriStrip)(IWarp3D, ctx, strip)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_alpha_mode(ctx: *mut APTR, mode: u32, ref_val: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_SetAlphaMode)(IWarp3D, ctx, mode, ref_val)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_blend_mode(ctx: *mut APTR, src: u32, dst: u32) -> u32 {
    ((*IWarp3D).W3D_SetBlendMode)(IWarp3D, ctx, src, dst)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_draw_region(ctx: *mut APTR, bm: *mut BitMap, y_off: i32, rect: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_SetDrawRegion)(IWarp3D, ctx, bm, y_off, rect)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_fog_params(ctx: *mut APTR, params: *mut APTR, mode: u32) -> u32 {
    ((*IWarp3D).W3D_SetFogParams)(IWarp3D, ctx, params, mode)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_color_mask(ctx: *mut APTR, r: APTR, g: APTR, b: APTR, a: APTR) -> u32 {
    ((*IWarp3D).W3D_SetColorMask)(IWarp3D, ctx, r, g, b, a)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_stencil_func(ctx: *mut APTR, func: u32, ref_val: u32, mask: u32) -> u32 {
    ((*IWarp3D).W3D_SetStencilFunc)(IWarp3D, ctx, func, ref_val, mask)
}
#[inline]
pub unsafe fn warp3d_w3_d_alloc_zbuffer(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_AllocZBuffer)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_free_zbuffer(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_FreeZBuffer)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_clear_zbuffer(ctx: *mut APTR, val: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_ClearZBuffer)(IWarp3D, ctx, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_read_zpixel(ctx: *mut APTR, x: u32, y: u32, val: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_ReadZPixel)(IWarp3D, ctx, x, y, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_read_zspan(ctx: *mut APTR, x: u32, y: u32, n: u32, val: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_ReadZSpan)(IWarp3D, ctx, x, y, n, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_zcompare_mode(ctx: *mut APTR, mode: u32) -> u32 {
    ((*IWarp3D).W3D_SetZCompareMode)(IWarp3D, ctx, mode)
}
#[inline]
pub unsafe fn warp3d_w3_d_alloc_stencil_buffer(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_AllocStencilBuffer)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_clear_stencil_buffer(ctx: *mut APTR, val: *mut u32) -> u32 {
    ((*IWarp3D).W3D_ClearStencilBuffer)(IWarp3D, ctx, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_fill_stencil_buffer(ctx: *mut APTR, x: u32, y: u32, w: u32, h: u32, depth: u32, data: *mut ()) -> u32 {
    ((*IWarp3D).W3D_FillStencilBuffer)(IWarp3D, ctx, x, y, w, h, depth, data)
}
#[inline]
pub unsafe fn warp3d_w3_d_free_stencil_buffer(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_FreeStencilBuffer)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_read_stencil_pixel(ctx: *mut APTR, x: u32, y: u32, val: *mut u32) -> u32 {
    ((*IWarp3D).W3D_ReadStencilPixel)(IWarp3D, ctx, x, y, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_read_stencil_span(ctx: *mut APTR, x: u32, y: u32, n: u32, val: *mut u32) -> u32 {
    ((*IWarp3D).W3D_ReadStencilSpan)(IWarp3D, ctx, x, y, n, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_logic_op(ctx: *mut APTR, op: u32) -> u32 {
    ((*IWarp3D).W3D_SetLogicOp)(IWarp3D, ctx, op)
}
#[inline]
pub unsafe fn warp3d_w3_d_hint(ctx: *mut APTR, mode: u32, qual: u32) -> u32 {
    ((*IWarp3D).W3D_Hint)(IWarp3D, ctx, mode, qual)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_draw_region_wbm(ctx: *mut APTR, bm: *mut APTR, rect: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_SetDrawRegionWBM)(IWarp3D, ctx, bm, rect)
}
#[inline]
pub unsafe fn warp3d_w3_d_get_driver_state(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_GetDriverState)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_flush(ctx: *mut APTR) -> u32 { ((*IWarp3D).W3D_Flush)(IWarp3D, ctx) }
#[inline]
pub unsafe fn warp3d_w3_d_set_pen_mask(ctx: *mut APTR, mask: u32) -> u32 {
    ((*IWarp3D).W3D_SetPenMask)(IWarp3D, ctx, mask)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_stencil_op(ctx: *mut APTR, sfail: u32, dpfail: u32, dppass: u32) -> u32 {
    ((*IWarp3D).W3D_SetStencilOp)(IWarp3D, ctx, sfail, dpfail, dppass)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_write_mask(ctx: *mut APTR, mask: u32) -> u32 {
    ((*IWarp3D).W3D_SetWriteMask)(IWarp3D, ctx, mask)
}
#[inline]
pub unsafe fn warp3d_w3_d_write_stencil_pixel(ctx: *mut APTR, x: u32, y: u32, val: u32) -> u32 {
    ((*IWarp3D).W3D_WriteStencilPixel)(IWarp3D, ctx, x, y, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_write_stencil_span(ctx: *mut APTR, x: u32, y: u32, n: u32, val: *mut u32, mask: *mut u8) -> u32 {
    ((*IWarp3D).W3D_WriteStencilSpan)(IWarp3D, ctx, x, y, n, val, mask)
}
#[inline]
pub unsafe fn warp3d_w3_d_write_zpixel(ctx: *mut APTR, x: u32, y: u32, val: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_WriteZPixel)(IWarp3D, ctx, x, y, val)
}
#[inline]
pub unsafe fn warp3d_w3_d_write_zspan(ctx: *mut APTR, x: u32, y: u32, n: u32, val: *mut APTR, mask: *mut u8) -> u32 {
    ((*IWarp3D).W3D_WriteZSpan)(IWarp3D, ctx, x, y, n, val, mask)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_current_color(ctx: *mut APTR, color: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_SetCurrentColor)(IWarp3D, ctx, color)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_current_pen(ctx: *mut APTR, pen: u32) -> u32 {
    ((*IWarp3D).W3D_SetCurrentPen)(IWarp3D, ctx, pen)
}
#[inline]
pub unsafe fn warp3d_w3_d_update_tex_sub_image(ctx: *mut APTR, tex: *mut APTR, image: *mut (), level: u32, pal: *mut u32, rect: *mut APTR, src_pitch: u32) -> u32 {
    ((*IWarp3D).W3D_UpdateTexSubImage)(IWarp3D, ctx, tex, image, level, pal, rect, src_pitch)
}
#[inline]
pub unsafe fn warp3d_w3_d_free_all_tex_obj(ctx: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_FreeAllTexObj)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_get_dest_fmt() -> u32 { ((*IWarp3D).W3D_GetDestFmt)(IWarp3D) }
#[inline]
pub unsafe fn warp3d_w3_d_draw_line_strip(ctx: *mut APTR, strip: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawLineStrip)(IWarp3D, ctx, strip)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_line_loop(ctx: *mut APTR, loop_: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawLineLoop)(IWarp3D, ctx, loop_)
}
#[inline]
pub unsafe fn warp3d_w3_d_get_drivers() -> *mut *mut APTR { ((*IWarp3D).W3D_GetDrivers)(IWarp3D) }
#[inline]
pub unsafe fn warp3d_w3_d_query_driver(drv: *mut APTR, query: u32, dest_fmt: u32) -> u32 {
    ((*IWarp3D).W3D_QueryDriver)(IWarp3D, drv, query, dest_fmt)
}
#[inline]
pub unsafe fn warp3d_w3_d_get_driver_tex_fmt_info(drv: *mut APTR, fmt: u32, dest_fmt: u32) -> u32 {
    ((*IWarp3D).W3D_GetDriverTexFmtInfo)(IWarp3D, drv, fmt, dest_fmt)
}
#[inline]
pub unsafe fn warp3d_w3_d_request_mode(tags: *mut TagItem) -> u32 {
    ((*IWarp3D).W3D_RequestMode)(IWarp3D, tags)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_scissor(ctx: *mut APTR, rect: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_SetScissor)(IWarp3D, ctx, rect)
}
#[inline]
pub unsafe fn warp3d_w3_d_flush_frame(ctx: *mut APTR) {
    ((*IWarp3D).W3D_FlushFrame)(IWarp3D, ctx)
}
#[inline]
pub unsafe fn warp3d_w3_d_test_mode(mode: u32) -> *mut APTR {
    ((*IWarp3D).W3D_TestMode)(IWarp3D, mode)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_chroma_test_bounds(ctx: *mut APTR, tex: *mut APTR, rgba_lo: u32, rgba_hi: u32, mode: u32) -> u32 {
    ((*IWarp3D).W3D_SetChromaTestBounds)(IWarp3D, ctx, tex, rgba_lo, rgba_hi, mode)
}
#[inline]
pub unsafe fn warp3d_w3_d_clear_draw_region(ctx: *mut APTR, color: u32) -> u32 {
    ((*IWarp3D).W3D_ClearDrawRegion)(IWarp3D, ctx, color)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_triangle_v(ctx: *mut APTR, tri: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawTriangleV)(IWarp3D, ctx, tri)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_tri_fan_v(ctx: *mut APTR, fan: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawTriFanV)(IWarp3D, ctx, fan)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_tri_strip_v(ctx: *mut APTR, strip: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_DrawTriStripV)(IWarp3D, ctx, strip)
}
#[inline]
pub unsafe fn warp3d_w3_d_get_screenmode_list() -> *mut APTR {
    ((*IWarp3D).W3D_GetScreenmodeList)(IWarp3D)
}
#[inline]
pub unsafe fn warp3d_w3_d_free_screenmode_list(list: *mut APTR) {
    ((*IWarp3D).W3D_FreeScreenmodeList)(IWarp3D, list)
}
#[inline]
pub unsafe fn warp3d_w3_d_best_mode_id(tags: *mut TagItem) -> u32 {
    ((*IWarp3D).W3D_BestModeID)(IWarp3D, tags)
}
#[inline]
pub unsafe fn warp3d_w3_d_vertex_pointer(ctx: *mut APTR, ptr: *mut (), size: i32, mode: u32, stride: u32) -> u32 {
    ((*IWarp3D).W3D_VertexPointer)(IWarp3D, ctx, ptr, size, mode, stride)
}
#[inline]
pub unsafe fn warp3d_w3_d_tex_coord_pointer(ctx: *mut APTR, ptr: *mut (), size: i32, unit: i32, mode: i32, stride: i32, flags: u32) -> u32 {
    ((*IWarp3D).W3D_TexCoordPointer)(IWarp3D, ctx, ptr, size, unit, mode, stride, flags)
}
#[inline]
pub unsafe fn warp3d_w3_d_color_pointer(ctx: *mut APTR, ptr: *mut (), size: i32, mode: u32, stride: u32, flags: u32) -> u32 {
    ((*IWarp3D).W3D_ColorPointer)(IWarp3D, ctx, ptr, size, mode, stride, flags)
}
#[inline]
pub unsafe fn warp3d_w3_d_bind_texture(ctx: *mut APTR, unit: u32, tex: *mut APTR) -> u32 {
    ((*IWarp3D).W3D_BindTexture)(IWarp3D, ctx, unit, tex)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_array(ctx: *mut APTR, prim: u32, base: u32, count: u32) -> u32 {
    ((*IWarp3D).W3D_DrawArray)(IWarp3D, ctx, prim, base, count)
}
#[inline]
pub unsafe fn warp3d_w3_d_draw_elements(ctx: *mut APTR, prim: u32, idx_type: u32, count: u32, indices: *mut ()) -> u32 {
    ((*IWarp3D).W3D_DrawElements)(IWarp3D, ctx, prim, idx_type, count, indices)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_front_face(ctx: *mut APTR, dir: u32) {
    ((*IWarp3D).W3D_SetFrontFace)(IWarp3D, ctx, dir)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_texture_blend(ctx: *mut APTR, tags: *mut TagItem) -> u32 {
    ((*IWarp3D).W3D_SetTextureBlend)(IWarp3D, ctx, tags)
}
#[inline]
pub unsafe fn warp3d_w3_d_secondary_color_pointer(ctx: *mut APTR, ptr: *mut (), size: i32, mode: u32, stride: u32, flags: u32) -> u32 {
    ((*IWarp3D).W3D_SecondaryColorPointer)(IWarp3D, ctx, ptr, size, mode, stride, flags)
}
#[inline]
pub unsafe fn warp3d_w3_d_fog_coord_pointer(ctx: *mut APTR, ptr: *mut (), size: i32, mode: u32, stride: u32) -> u32 {
    ((*IWarp3D).W3D_FogCoordPointer)(IWarp3D, ctx, ptr, size, mode, stride)
}
#[inline]
pub unsafe fn warp3d_w3_d_interleaved_array(ctx: *mut APTR, ptr: *mut (), size: i32, mode: u32, stride: u32) -> u32 {
    ((*IWarp3D).W3D_InterleavedArray)(IWarp3D, ctx, ptr, size, mode, stride)
}
#[inline]
pub unsafe fn warp3d_w3_d_clear_buffers(ctx: *mut APTR, color: *mut APTR, depth: *mut APTR, stencil: *mut u32) -> u32 {
    ((*IWarp3D).W3D_ClearBuffers)(IWarp3D, ctx, color, depth, stencil)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_parameter(ctx: *mut APTR, param: u32, value: *mut ()) -> u32 {
    ((*IWarp3D).W3D_SetParameter)(IWarp3D, ctx, param, value)
}
#[inline]
pub unsafe fn warp3d_w3_d_set_max_anisotropy(ctx: *mut APTR, tex: *mut APTR, max: u32) -> u32 {
    ((*IWarp3D).W3D_SetMaxAnisotropy)(IWarp3D, ctx, tex, max)
}
