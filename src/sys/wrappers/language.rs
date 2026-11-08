//! ILanguage global(s) and convenience wrappers.
//!
//! Hand-written binding for locale.library's language sub-interface —
//! similar shape to charset but with `LG_*` prefix; per-language
//! classifiers and case conversion.

use crate::sys::types::*;
use crate::sys::interfaces::language::*;

// ---- ILanguage (LanguageIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static ILanguage: *mut LanguageIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut ILanguage: *mut LanguageIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn language_lg_get_driver_info() -> u32 {
    ((*ILanguage).LG_GetDriverInfo)(ILanguage)
}

#[inline]
pub unsafe fn language_lg_conv_to_lower(c: u32) -> u32 {
    ((*ILanguage).LG_ConvToLower)(ILanguage, c)
}

#[inline]
pub unsafe fn language_lg_conv_to_upper(c: u32) -> u32 {
    ((*ILanguage).LG_ConvToUpper)(ILanguage, c)
}

#[inline]
pub unsafe fn language_lg_get_code_set() -> u32 {
    ((*ILanguage).LG_GetCodeSet)(ILanguage)
}

#[inline]
pub unsafe fn language_lg_get_locale_str(id: u32) -> CONST_STRPTR {
    ((*ILanguage).LG_GetLocaleStr)(ILanguage, id)
}

#[inline]
pub unsafe fn language_lg_is_al_num(c: u32) -> i32 { ((*ILanguage).LG_IsAlNum)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_alpha(c: u32) -> i32 { ((*ILanguage).LG_IsAlpha)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_cntrl(c: u32) -> i32 { ((*ILanguage).LG_IsCntrl)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_digit(c: u32) -> i32 { ((*ILanguage).LG_IsDigit)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_graph(c: u32) -> i32 { ((*ILanguage).LG_IsGraph)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_lower(c: u32) -> i32 { ((*ILanguage).LG_IsLower)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_print(c: u32) -> i32 { ((*ILanguage).LG_IsPrint)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_punct(c: u32) -> i32 { ((*ILanguage).LG_IsPunct)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_space(c: u32) -> i32 { ((*ILanguage).LG_IsSpace)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_upper(c: u32) -> i32 { ((*ILanguage).LG_IsUpper)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_xdigit(c: u32) -> i32 { ((*ILanguage).LG_IsXDigit)(ILanguage, c) }
#[inline]
pub unsafe fn language_lg_is_blank(c: u32) -> i32 { ((*ILanguage).LG_IsBlank)(ILanguage, c) }

#[inline]
pub unsafe fn language_lg_str_convert(
    src: CONST_STRPTR, dst: APTR, dst_len: u32, flags: u32,
) -> u32 {
    ((*ILanguage).LG_StrConvert)(ILanguage, src, dst, dst_len, flags)
}

#[inline]
pub unsafe fn language_lg_strn_cmp(
    s1: CONST_STRPTR, s2: CONST_STRPTR, n: i32, flags: u32,
) -> i32 {
    ((*ILanguage).LG_StrnCmp)(ILanguage, s1, s2, n, flags)
}
