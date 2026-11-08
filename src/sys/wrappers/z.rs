//! IZ global(s) and convenience wrappers.
//!
//! Hand-written binding for z.library — zlib (deflate/inflate)
//! compression. Mirrors the standard zlib C API: low-level stream
//! Compress/Inflate, plus Compress/Uncompress one-shot variants,
//! Adler-32/CRC-32 checksums, and the GZIP-style Inflate-back API.

use crate::sys::types::*;
use crate::sys::interfaces::z::*;

// ---- IZ (ZIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IZ: *mut ZIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IZ: *mut ZIFace = core::ptr::null_mut();

// ── Deflate stream ───────────────────────────────────────────

#[inline]
pub unsafe fn z_deflate_init(strm: *mut APTR, level: i32) -> i32 { ((*IZ).DeflateInit)(IZ, strm, level) }
#[inline]
pub unsafe fn z_deflate(strm: *mut APTR, flush: i32) -> i32 { ((*IZ).Deflate)(IZ, strm, flush) }
#[inline]
pub unsafe fn z_deflate_end(strm: *mut APTR) -> i32 { ((*IZ).DeflateEnd)(IZ, strm) }
#[inline]
pub unsafe fn z_deflate_init2(
    strm: *mut APTR, level: i32, method: i32, window_bits: i32, mem_level: i32, strategy: i32,
) -> i32 {
    ((*IZ).DeflateInit2)(IZ, strm, level, method, window_bits, mem_level, strategy)
}
#[inline]
pub unsafe fn z_deflate_set_dictionary(strm: *mut APTR, dict: *const u8, dict_length: u32) -> i32 { ((*IZ).DeflateSetDictionary)(IZ, strm, dict, dict_length) }
#[inline]
pub unsafe fn z_deflate_copy(dest: *mut APTR, source: *mut APTR) -> i32 { ((*IZ).DeflateCopy)(IZ, dest, source) }
#[inline]
pub unsafe fn z_deflate_reset(strm: *mut APTR) -> i32 { ((*IZ).DeflateReset)(IZ, strm) }
#[inline]
pub unsafe fn z_deflate_params(strm: *mut APTR, level: i32, strategy: i32) -> i32 { ((*IZ).DeflateParams)(IZ, strm, level, strategy) }
#[inline]
pub unsafe fn z_deflate_tune(strm: *mut APTR, good_length: i32, max_lazy: i32, nice_length: i32, max_chain: i32) -> i32 {
    ((*IZ).DeflateTune)(IZ, strm, good_length, max_lazy, nice_length, max_chain)
}
#[inline]
pub unsafe fn z_deflate_bound(strm: *mut APTR, source_length: u32) -> u32 { ((*IZ).DeflateBound)(IZ, strm, source_length) }
#[inline]
pub unsafe fn z_deflate_prime(strm: *mut APTR, bits: i32, value: i32) -> i32 { ((*IZ).DeflatePrime)(IZ, strm, bits, value) }
#[inline]
pub unsafe fn z_deflate_set_header(strm: *mut APTR, head: *mut APTR) -> i32 { ((*IZ).DeflateSetHeader)(IZ, strm, head) }

// ── Inflate stream ───────────────────────────────────────────

#[inline]
pub unsafe fn z_inflate_init(strm: *mut APTR) -> i32 { ((*IZ).InflateInit)(IZ, strm) }
#[inline]
pub unsafe fn z_inflate(strm: *mut APTR, flush: i32) -> i32 { ((*IZ).Inflate)(IZ, strm, flush) }
#[inline]
pub unsafe fn z_inflate_end(strm: *mut APTR) -> i32 { ((*IZ).InflateEnd)(IZ, strm) }
#[inline]
pub unsafe fn z_inflate_init2(strm: *mut APTR, window_bits: i32) -> i32 { ((*IZ).InflateInit2)(IZ, strm, window_bits) }
#[inline]
pub unsafe fn z_inflate_set_dictionary(strm: *mut APTR, dict: *const u8, dict_length: u32) -> i32 { ((*IZ).InflateSetDictionary)(IZ, strm, dict, dict_length) }
#[inline]
pub unsafe fn z_inflate_sync(strm: *mut APTR) -> i32 { ((*IZ).InflateSync)(IZ, strm) }
#[inline]
pub unsafe fn z_inflate_copy(dest: *mut APTR, source: *mut APTR) -> i32 { ((*IZ).InflateCopy)(IZ, dest, source) }
#[inline]
pub unsafe fn z_inflate_reset(strm: *mut APTR) -> i32 { ((*IZ).InflateReset)(IZ, strm) }
#[inline]
pub unsafe fn z_inflate_prime(strm: *mut APTR, bits: i32, value: i32) -> i32 { ((*IZ).InflatePrime)(IZ, strm, bits, value) }
#[inline]
pub unsafe fn z_inflate_get_header(strm: *mut APTR, head: *mut APTR) -> i32 { ((*IZ).InflateGetHeader)(IZ, strm, head) }
#[inline]
pub unsafe fn z_inflate_get_dictionary(strm: *mut APTR, dict: *mut u8, dict_length: *mut u32) -> i32 { ((*IZ).InflateGetDictionary)(IZ, strm, dict, dict_length) }
#[inline]
pub unsafe fn z_inflate_reset2(strm: *mut APTR, window_bits: i32) -> i32 { ((*IZ).InflateReset2)(IZ, strm, window_bits) }
#[inline]
pub unsafe fn z_inflate_validate(strm: *mut APTR, check: i32) -> i32 { ((*IZ).InflateValidate)(IZ, strm, check) }

// ── Inflate-back (gzip-style chunked) ────────────────────────

#[inline]
pub unsafe fn z_inflate_back_init(strm: *mut APTR, window_bits: i32, window: *mut u8) -> i32 { ((*IZ).InflateBackInit)(IZ, strm, window_bits, window) }
#[inline]
pub unsafe fn z_inflate_back(
    strm: *mut APTR, in_fn: APTR, in_desc: *mut (), out_fn: APTR, out_desc: *mut (),
) -> i32 {
    ((*IZ).InflateBack)(IZ, strm, in_fn, in_desc, out_fn, out_desc)
}
#[inline]
pub unsafe fn z_inflate_back_end(strm: *mut APTR) -> i32 { ((*IZ).InflateBackEnd)(IZ, strm) }

// ── Checksums ────────────────────────────────────────────────

#[inline]
pub unsafe fn z_adler32(adler: u32, buf: *const u8, len: u32) -> u32 { ((*IZ).Adler32)(IZ, adler, buf, len) }
#[inline]
pub unsafe fn z_adler32_combine(adler1: u32, adler2: u32, len2: i32) -> u32 { ((*IZ).Adler32Combine)(IZ, adler1, adler2, len2) }
#[inline]
pub unsafe fn z_crc32(crc: u32, buf: *const u8, len: u32) -> u32 { ((*IZ).CRC32)(IZ, crc, buf, len) }
#[inline]
pub unsafe fn z_crc32_combine(crc1: u32, crc2: u32, len2: i32) -> u32 { ((*IZ).CRC32Combine)(IZ, crc1, crc2, len2) }

// ── One-shot compress / uncompress ───────────────────────────

#[inline]
pub unsafe fn z_zlib_version() -> CONST_STRPTR { ((*IZ).ZlibVersion)(IZ) }
#[inline]
pub unsafe fn z_compress(dest: APTR, dest_len: *mut u32, source: CONST_APTR, source_len: u32) -> i32 { ((*IZ).Compress)(IZ, dest, dest_len, source, source_len) }
#[inline]
pub unsafe fn z_uncompress(dest: APTR, dest_len: *mut u32, source: CONST_APTR, source_len: u32) -> i32 { ((*IZ).Uncompress)(IZ, dest, dest_len, source, source_len) }
#[inline]
pub unsafe fn z_compress2(dest: APTR, dest_len: *mut u32, source: CONST_APTR, source_len: u32, level: i32) -> i32 {
    ((*IZ).Compress2)(IZ, dest, dest_len, source, source_len, level)
}
#[inline]
pub unsafe fn z_compress_bound(source_len: u32) -> u32 { ((*IZ).CompressBound)(IZ, source_len) }
#[inline]
pub unsafe fn z_uncompress2(dest: APTR, dest_len: *mut u32, source: CONST_APTR, source_len: *mut u32) -> i32 {
    ((*IZ).Uncompress2)(IZ, dest, dest_len, source, source_len)
}

// ── Error string ─────────────────────────────────────────────

#[inline]
pub unsafe fn z_zerror(error: i32) -> CONST_STRPTR { ((*IZ).ZError)(IZ, error) }
