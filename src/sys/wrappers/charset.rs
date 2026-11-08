//! ICharset global(s) and convenience wrappers.
//!
//! Hand-written binding for locale.library's charset sub-interface —
//! per-character-set classification (isAlpha/isDigit/etc.) and case
//! conversion. All methods are `CS_*`-prefixed per the SDK.

use crate::sys::types::*;
use crate::sys::interfaces::charset::*;

// ---- ICharset (CharsetIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ICharset: *mut CharsetIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ICharset: *mut CharsetIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn charset_cs_get_driver_info() -> u32 {
    ((*ICharset).CS_GetDriverInfo)(ICharset)
}

#[inline]
pub unsafe fn charset_cs_conv_to_lower(c: u32) -> u32 {
    ((*ICharset).CS_ConvToLower)(ICharset, c)
}

#[inline]
pub unsafe fn charset_cs_conv_to_upper(c: u32) -> u32 {
    ((*ICharset).CS_ConvToUpper)(ICharset, c)
}

#[inline]
pub unsafe fn charset_cs_never_called_get_code_set() -> u32 {
    ((*ICharset).CS_NeverCalledGetCodeSet)(ICharset)
}

#[inline]
pub unsafe fn charset_cs_never_called_get_locale_str(id: u32) -> CONST_STRPTR {
    ((*ICharset).CS_NeverCalledGetLocaleStr)(ICharset, id)
}

// ── isXxx predicates ─────────────────────────────────────────

#[inline]
pub unsafe fn charset_cs_is_al_num(c: u32) -> i32 { ((*ICharset).CS_IsAlNum)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_alpha(c: u32) -> i32 { ((*ICharset).CS_IsAlpha)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_cntrl(c: u32) -> i32 { ((*ICharset).CS_IsCntrl)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_digit(c: u32) -> i32 { ((*ICharset).CS_IsDigit)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_graph(c: u32) -> i32 { ((*ICharset).CS_IsGraph)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_lower(c: u32) -> i32 { ((*ICharset).CS_IsLower)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_print(c: u32) -> i32 { ((*ICharset).CS_IsPrint)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_punct(c: u32) -> i32 { ((*ICharset).CS_IsPunct)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_space(c: u32) -> i32 { ((*ICharset).CS_IsSpace)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_upper(c: u32) -> i32 { ((*ICharset).CS_IsUpper)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_xdigit(c: u32) -> i32 { ((*ICharset).CS_IsXDigit)(ICharset, c) }
#[inline]
pub unsafe fn charset_cs_is_blank(c: u32) -> i32 { ((*ICharset).CS_IsBlank)(ICharset, c) }

// ── String operations ───────────────────────────────────────

#[inline]
pub unsafe fn charset_cs_str_convert(
    src: CONST_STRPTR, dst: APTR, dst_len: u32, flags: u32,
) -> u32 {
    ((*ICharset).CS_StrConvert)(ICharset, src, dst, dst_len, flags)
}

#[inline]
pub unsafe fn charset_cs_strn_cmp(
    s1: CONST_STRPTR, s2: CONST_STRPTR, n: i32, flags: u32,
) -> i32 {
    ((*ICharset).CS_StrnCmp)(ICharset, s1, s2, n, flags)
}
