//! ILZMA global(s) and convenience wrappers for lzma.library.

use crate::sys::types::*;
use crate::sys::interfaces::lzma::*;

#[cfg(target_os = "amigaos")]
extern "C" { pub static ILZMA: *mut LZMAIFace; }
#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ILZMA: *mut LZMAIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn lzma_lzma_code(a: *mut APTR, b: APTR) -> APTR { ((*ILZMA).lzma_code)(ILZMA, a, b) }
#[inline]
pub unsafe fn lzma_lzma_end(a: *mut APTR) { ((*ILZMA).lzma_end)(ILZMA, a) }
#[inline]
pub unsafe fn lzma_lzma_memusage(a: *const APTR) -> APTR { ((*ILZMA).lzma_memusage)(ILZMA, a) }
#[inline]
pub unsafe fn lzma_lzma_memlimit_get(a: *const APTR) -> APTR { ((*ILZMA).lzma_memlimit_get)(ILZMA, a) }
#[inline]
pub unsafe fn lzma_lzma_memlimit_set(a: *mut APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_memlimit_set)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_block_header_size(a: *mut APTR) -> APTR {
    ((*ILZMA).lzma_block_header_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_block_header_encode(a: *const APTR, b: *mut APTR) -> APTR {
    ((*ILZMA).lzma_block_header_encode)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_block_header_decode(a: *mut APTR, b: *mut APTR, c: *const APTR) -> APTR {
    ((*ILZMA).lzma_block_header_decode)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_block_compressed_size(a: *mut APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_block_compressed_size)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_block_unpadded_size(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_block_unpadded_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_block_total_size(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_block_total_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_block_encoder(a: *mut APTR, b: *mut APTR) -> APTR {
    ((*ILZMA).lzma_block_encoder)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_block_decoder(a: *mut APTR, b: *mut APTR) -> APTR {
    ((*ILZMA).lzma_block_decoder)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_block_buffer_bound(a: u32) -> u32 {
    ((*ILZMA).lzma_block_buffer_bound)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_block_buffer_encode(
    a: *mut APTR, b: *mut APTR, c: *const APTR, d: u32,
    e: *mut APTR, f: *mut u32, g: u32,
) -> APTR {
    ((*ILZMA).lzma_block_buffer_encode)(ILZMA, a, b, c, d, e, f, g)
}
#[inline]
pub unsafe fn lzma_lzma_block_buffer_decode(
    a: *mut APTR, b: *mut APTR, c: *const APTR, d: *mut u32, e: u32,
    f: *mut APTR, g: *mut u32, h: u32,
) -> APTR {
    ((*ILZMA).lzma_block_buffer_decode)(ILZMA, a, b, c, d, e, f, g, h)
}
#[inline]
pub unsafe fn lzma_lzma_check_is_supported(a: APTR) -> APTR {
    ((*ILZMA).lzma_check_is_supported)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_check_size(a: APTR) -> APTR { ((*ILZMA).lzma_check_size)(ILZMA, a) }
#[inline]
pub unsafe fn lzma_lzma_crc32(a: *const APTR, b: u32, c: APTR) -> APTR {
    ((*ILZMA).lzma_crc32)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_crc64(a: *const APTR, b: u32, c: APTR) -> APTR {
    ((*ILZMA).lzma_crc64)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_get_check(a: *const APTR) -> APTR { ((*ILZMA).lzma_get_check)(ILZMA, a) }
#[inline]
pub unsafe fn lzma_lzma_easy_encoder_memusage(a: APTR) -> APTR {
    ((*ILZMA).lzma_easy_encoder_memusage)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_easy_decoder_memusage(a: APTR) -> APTR {
    ((*ILZMA).lzma_easy_decoder_memusage)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_easy_encoder(a: *mut APTR, b: APTR, c: APTR) -> APTR {
    ((*ILZMA).lzma_easy_encoder)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_easy_buffer_encode(
    a: APTR, b: APTR, c: *mut APTR, d: *const APTR, e: u32,
    f: *mut APTR, g: *mut u32, h: u32,
) -> APTR {
    ((*ILZMA).lzma_easy_buffer_encode)(ILZMA, a, b, c, d, e, f, g, h)
}
#[inline]
pub unsafe fn lzma_lzma_stream_encoder(a: *mut APTR, b: *const APTR, c: APTR) -> APTR {
    ((*ILZMA).lzma_stream_encoder)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_alone_encoder(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_alone_encoder)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_stream_buffer_bound(a: u32) -> u32 {
    ((*ILZMA).lzma_stream_buffer_bound)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_stream_buffer_encode(
    a: *mut APTR, b: APTR, c: *mut APTR, d: *const APTR, e: u32,
    f: *mut APTR, g: *mut u32, h: u32,
) -> APTR {
    ((*ILZMA).lzma_stream_buffer_encode)(ILZMA, a, b, c, d, e, f, g, h)
}
#[inline]
pub unsafe fn lzma_lzma_stream_decoder(a: *mut APTR, b: APTR, c: APTR) -> APTR {
    ((*ILZMA).lzma_stream_decoder)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_auto_decoder(a: *mut APTR, b: APTR, c: APTR) -> APTR {
    ((*ILZMA).lzma_auto_decoder)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_alone_decoder(a: *mut APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_alone_decoder)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_stream_buffer_decode(
    a: *mut APTR, b: APTR, c: *mut APTR, d: *const APTR, e: *mut u32, f: u32,
    g: *mut APTR, h: *mut u32, i: u32,
) -> APTR {
    ((*ILZMA).lzma_stream_buffer_decode)(ILZMA, a, b, c, d, e, f, g, h, i)
}
#[inline]
pub unsafe fn lzma_lzma_filter_encoder_is_supported(a: APTR) -> APTR {
    ((*ILZMA).lzma_filter_encoder_is_supported)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_filter_decoder_is_supported(a: APTR) -> APTR {
    ((*ILZMA).lzma_filter_decoder_is_supported)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_filters_copy(a: *const APTR, b: *mut APTR, c: *mut APTR) -> APTR {
    ((*ILZMA).lzma_filters_copy)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_raw_encoder_memusage(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_raw_encoder_memusage)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_raw_decoder_memusage(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_raw_decoder_memusage)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_raw_encoder(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_raw_encoder)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_raw_decoder(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_raw_decoder)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_filters_update(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_filters_update)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_raw_buffer_encode(
    a: *const APTR, b: *mut APTR, c: *const APTR, d: u32,
    e: *mut APTR, f: *mut u32, g: u32,
) -> APTR {
    ((*ILZMA).lzma_raw_buffer_encode)(ILZMA, a, b, c, d, e, f, g)
}
#[inline]
pub unsafe fn lzma_lzma_raw_buffer_decode(
    a: *const APTR, b: *mut APTR, c: *const APTR, d: *mut u32, e: u32,
    f: *mut APTR, g: *mut u32, h: u32,
) -> APTR {
    ((*ILZMA).lzma_raw_buffer_decode)(ILZMA, a, b, c, d, e, f, g, h)
}
#[inline]
pub unsafe fn lzma_lzma_properties_size(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_properties_size)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_properties_encode(a: *const APTR, b: *mut APTR) -> APTR {
    ((*ILZMA).lzma_properties_encode)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_properties_decode(a: *mut APTR, b: *mut APTR, c: *const APTR, d: u32) -> APTR {
    ((*ILZMA).lzma_properties_decode)(ILZMA, a, b, c, d)
}
#[inline]
pub unsafe fn lzma_lzma_filter_flags_size(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_filter_flags_size)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_filter_flags_encode(a: *const APTR, b: *mut APTR, c: *mut u32, d: u32) -> APTR {
    ((*ILZMA).lzma_filter_flags_encode)(ILZMA, a, b, c, d)
}
#[inline]
pub unsafe fn lzma_lzma_filter_flags_decode(
    a: *mut APTR, b: *mut APTR, c: *const APTR, d: *mut u32, e: u32,
) -> APTR {
    ((*ILZMA).lzma_filter_flags_decode)(ILZMA, a, b, c, d, e)
}
#[inline]
pub unsafe fn lzma_lzma_physmem() -> APTR { ((*ILZMA).lzma_physmem)(ILZMA) }
#[inline]
pub unsafe fn lzma_lzma_index_memusage(a: APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_index_memusage)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_memused(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_memused)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_init(a: *mut APTR) -> *mut APTR {
    ((*ILZMA).lzma_index_init)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_end(a: *mut APTR, b: *mut APTR) {
    ((*ILZMA).lzma_index_end)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_append(a: *mut APTR, b: *mut APTR, c: APTR, d: APTR) -> APTR {
    ((*ILZMA).lzma_index_append)(ILZMA, a, b, c, d)
}
#[inline]
pub unsafe fn lzma_lzma_index_stream_flags(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_stream_flags)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_checks(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_checks)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_stream_padding(a: *mut APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_index_stream_padding)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_stream_count(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_stream_count)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_block_count(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_block_count)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_size(a: *const APTR) -> APTR { ((*ILZMA).lzma_index_size)(ILZMA, a) }
#[inline]
pub unsafe fn lzma_lzma_index_stream_size(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_stream_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_total_size(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_total_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_file_size(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_file_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_uncompressed_size(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_uncompressed_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_iter_init(a: *mut APTR, b: *const APTR) {
    ((*ILZMA).lzma_index_iter_init)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_iter_rewind(a: *mut APTR) {
    ((*ILZMA).lzma_index_iter_rewind)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_index_iter_next(a: *mut APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_index_iter_next)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_iter_locate(a: *mut APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_index_iter_locate)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_cat(a: *mut APTR, b: *mut APTR, c: *mut APTR) -> APTR {
    ((*ILZMA).lzma_index_cat)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_index_dup(a: *const APTR, b: *mut APTR) -> *mut APTR {
    ((*ILZMA).lzma_index_dup)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_encoder(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_encoder)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_decoder(a: *mut APTR, b: *mut *mut APTR, c: APTR) -> APTR {
    ((*ILZMA).lzma_index_decoder)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_index_buffer_encode(
    a: *const APTR, b: *mut APTR, c: *mut u32, d: u32,
) -> APTR {
    ((*ILZMA).lzma_index_buffer_encode)(ILZMA, a, b, c, d)
}
#[inline]
pub unsafe fn lzma_lzma_index_buffer_decode(
    a: *mut *mut APTR, b: *mut APTR, c: *mut APTR, d: *const APTR, e: *mut u32, f: u32,
) -> APTR {
    ((*ILZMA).lzma_index_buffer_decode)(ILZMA, a, b, c, d, e, f)
}
#[inline]
pub unsafe fn lzma_lzma_index_hash_init(a: *mut APTR, b: *mut APTR) -> *mut APTR {
    ((*ILZMA).lzma_index_hash_init)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_hash_end(a: *mut APTR, b: *mut APTR) {
    ((*ILZMA).lzma_index_hash_end)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_index_hash_append(a: *mut APTR, b: APTR, c: APTR) -> APTR {
    ((*ILZMA).lzma_index_hash_append)(ILZMA, a, b, c)
}
#[inline]
pub unsafe fn lzma_lzma_index_hash_decode(
    a: *mut APTR, b: *const APTR, c: *mut u32, d: u32,
) -> APTR {
    ((*ILZMA).lzma_index_hash_decode)(ILZMA, a, b, c, d)
}
#[inline]
pub unsafe fn lzma_lzma_index_hash_size(a: *const APTR) -> APTR {
    ((*ILZMA).lzma_index_hash_size)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_mf_is_supported(a: APTR) -> APTR {
    ((*ILZMA).lzma_mf_is_supported)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_mode_is_supported(a: APTR) -> APTR {
    ((*ILZMA).lzma_mode_is_supported)(ILZMA, a)
}
#[inline]
pub unsafe fn lzma_lzma_lzma_preset(a: *mut APTR, b: APTR) -> APTR {
    ((*ILZMA).lzma_lzma_preset)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_stream_header_encode(a: *const APTR, b: *mut APTR) -> APTR {
    ((*ILZMA).lzma_stream_header_encode)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_stream_footer_encode(a: *const APTR, b: *mut APTR) -> APTR {
    ((*ILZMA).lzma_stream_footer_encode)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_stream_header_decode(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_stream_header_decode)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_stream_footer_decode(a: *mut APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_stream_footer_decode)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_stream_flags_compare(a: *const APTR, b: *const APTR) -> APTR {
    ((*ILZMA).lzma_stream_flags_compare)(ILZMA, a, b)
}
#[inline]
pub unsafe fn lzma_lzma_version_number() -> APTR { ((*ILZMA).lzma_version_number)(ILZMA) }
#[inline]
pub unsafe fn lzma_lzma_version_string() -> *const APTR { ((*ILZMA).lzma_version_string)(ILZMA) }
#[inline]
pub unsafe fn lzma_lzma_vli_encode(
    a: APTR, b: *mut u32, c: *mut APTR, d: *mut u32, e: u32,
) -> APTR {
    ((*ILZMA).lzma_vli_encode)(ILZMA, a, b, c, d, e)
}
#[inline]
pub unsafe fn lzma_lzma_vli_decode(
    a: *mut APTR, b: *mut u32, c: *const APTR, d: *mut u32, e: u32,
) -> APTR {
    ((*ILZMA).lzma_vli_decode)(ILZMA, a, b, c, d, e)
}
#[inline]
pub unsafe fn lzma_lzma_vli_size(a: APTR) -> APTR { ((*ILZMA).lzma_vli_size)(ILZMA, a) }
#[inline]
pub unsafe fn lzma_lzma_get_progress(a: *mut APTR, b: *mut APTR, c: *mut APTR) {
    ((*ILZMA).lzma_get_progress)(ILZMA, a, b, c)
}
