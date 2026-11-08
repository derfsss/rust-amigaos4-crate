//! IBZip2 global(s) and convenience wrappers.
//!
//! Hand-written binding for bzip2.library — Burrows-Wheeler block-
//! sort compression. Mirrors the standard libbz2 C API surface
//! (low-level Compress/Decompress + stream-style bzRead/bzWrite +
//! fopen-style bzopen/bzdopen).

use crate::sys::types::*;
use crate::sys::interfaces::bzip2::*;

// ---- IBZip2 (BZip2IFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IBZip2: *mut BZip2IFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IBZip2: *mut BZip2IFace = core::ptr::null_mut();

// ── Version ──────────────────────────────────────────────────

#[inline]
pub unsafe fn bzip2_bz2_bzlib_version() -> *const APTR {
    ((*IBZip2).BZ2_bzlibVersion)(IBZip2)
}

// ── Low-level stream compression API ─────────────────────────

#[inline]
pub unsafe fn bzip2_bz2_bz_compress_init(
    strm: *mut APTR, block_size_100k: i32, verbosity: i32, work_factor: i32,
) -> i32 {
    ((*IBZip2).BZ2_bzCompressInit)(IBZip2, strm, block_size_100k, verbosity, work_factor)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_compress(strm: *mut APTR, action: i32) -> i32 {
    ((*IBZip2).BZ2_bzCompress)(IBZip2, strm, action)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_compress_end(strm: *mut APTR) -> i32 {
    ((*IBZip2).BZ2_bzCompressEnd)(IBZip2, strm)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_decompress_init(strm: *mut APTR, verbosity: i32, small: i32) -> i32 {
    ((*IBZip2).BZ2_bzDecompressInit)(IBZip2, strm, verbosity, small)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_decompress(strm: *mut APTR) -> i32 {
    ((*IBZip2).BZ2_bzDecompress)(IBZip2, strm)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_decompress_end(strm: *mut APTR) -> i32 {
    ((*IBZip2).BZ2_bzDecompressEnd)(IBZip2, strm)
}

// ── Buffer-to-buffer one-shot compression ────────────────────

#[inline]
pub unsafe fn bzip2_bz2_bz_buff_to_buff_compress(
    dest: *mut APTR, dest_len: *mut APTR, source: *mut APTR, source_len: APTR,
    block_size_100k: i32, verbosity: i32, work_factor: i32,
) -> i32 {
    ((*IBZip2).BZ2_bzBuffToBuffCompress)(
        IBZip2, dest, dest_len, source, source_len, block_size_100k, verbosity, work_factor,
    )
}

#[inline]
pub unsafe fn bzip2_bz2_bz_buff_to_buff_decompress(
    dest: *mut APTR, dest_len: *mut APTR, source: *mut APTR, source_len: APTR,
    small: i32, verbosity: i32,
) -> i32 {
    ((*IBZip2).BZ2_bzBuffToBuffDecompress)(
        IBZip2, dest, dest_len, source, source_len, small, verbosity,
    )
}

// ── Stream-style read/write API ──────────────────────────────

#[inline]
pub unsafe fn bzip2_bz2_bz_read_open(
    bzerror: *mut i32, f: *mut APTR, verbosity: i32, small: i32,
    unused: *mut (), n_unused: i32,
) -> *mut APTR {
    ((*IBZip2).BZ2_bzReadOpen)(IBZip2, bzerror, f, verbosity, small, unused, n_unused)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_read_close(bzerror: *mut i32, b: *mut APTR) {
    ((*IBZip2).BZ2_bzReadClose)(IBZip2, bzerror, b)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_read_get_unused(
    bzerror: *mut i32, b: *mut APTR, unused: *mut *mut (), n_unused: *mut i32,
) {
    ((*IBZip2).BZ2_bzReadGetUnused)(IBZip2, bzerror, b, unused, n_unused)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_read(
    bzerror: *mut i32, b: *mut APTR, buf: *mut (), len: i32,
) -> i32 {
    ((*IBZip2).BZ2_bzRead)(IBZip2, bzerror, b, buf, len)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_write_open(
    bzerror: *mut i32, f: *mut APTR, block_size_100k: i32, verbosity: i32, work_factor: i32,
) -> *mut APTR {
    ((*IBZip2).BZ2_bzWriteOpen)(IBZip2, bzerror, f, block_size_100k, verbosity, work_factor)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_write(bzerror: *mut i32, b: *mut APTR, buf: *mut (), len: i32) {
    ((*IBZip2).BZ2_bzWrite)(IBZip2, bzerror, b, buf, len)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_write_close(
    bzerror: *mut i32, b: *mut APTR, abandon: i32,
    nbytes_in: *mut APTR, nbytes_out: *mut APTR,
) {
    ((*IBZip2).BZ2_bzWriteClose)(IBZip2, bzerror, b, abandon, nbytes_in, nbytes_out)
}

#[inline]
pub unsafe fn bzip2_bz2_bz_write_close64(
    bzerror: *mut i32, b: *mut APTR, abandon: i32,
    nbytes_in_lo: *mut APTR, nbytes_in_hi: *mut APTR,
    nbytes_out_lo: *mut APTR, nbytes_out_hi: *mut APTR,
) {
    ((*IBZip2).BZ2_bzWriteClose64)(
        IBZip2, bzerror, b, abandon, nbytes_in_lo, nbytes_in_hi, nbytes_out_lo, nbytes_out_hi,
    )
}

// ── fopen-style API ──────────────────────────────────────────

#[inline]
pub unsafe fn bzip2_bz2_bzopen(path: *const APTR, mode: *const APTR) -> *mut APTR {
    ((*IBZip2).BZ2_bzopen)(IBZip2, path, mode)
}

#[inline]
pub unsafe fn bzip2_bz2_bzdopen(fd: i32, mode: *const APTR) -> *mut APTR {
    ((*IBZip2).BZ2_bzdopen)(IBZip2, fd, mode)
}

#[inline]
pub unsafe fn bzip2_bz2_bzread(b: *mut APTR, buf: *mut (), len: i32) -> i32 {
    ((*IBZip2).BZ2_bzread)(IBZip2, b, buf, len)
}

#[inline]
pub unsafe fn bzip2_bz2_bzwrite(b: *mut APTR, buf: *mut (), len: i32) -> i32 {
    ((*IBZip2).BZ2_bzwrite)(IBZip2, b, buf, len)
}

#[inline]
pub unsafe fn bzip2_bz2_bzflush(b: *mut APTR) -> i32 {
    ((*IBZip2).BZ2_bzflush)(IBZip2, b)
}

#[inline]
pub unsafe fn bzip2_bz2_bzclose(b: *mut APTR) {
    ((*IBZip2).BZ2_bzclose)(IBZip2, b)
}

#[inline]
pub unsafe fn bzip2_bz2_bzerror(b: *mut APTR, errnum: *mut i32) -> *const APTR {
    ((*IBZip2).BZ2_bzerror)(IBZip2, b, errnum)
}
