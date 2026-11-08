//! What exec knows about the CPU (GetCPUInfo).

use core::ffi::{c_char, CStr};

use crate::sys::{self, TagItem, TAG_DONE};

/// The CPU, as GetCPUInfo describes it. Strings are exec's own (static).
#[derive(Clone, Copy, Debug)]
pub struct CpuInfo {
    pub count: u32,
    /// CPUFAMILY_* (exec/exectags.h).
    pub family: u32,
    /// CPUTYPE_*.
    pub model: u32,
    pub model_name: &'static str,
    pub version: u32,
    pub version_name: &'static str,
    /// Hz.
    pub cpu_speed: u64,
    /// Hz.
    pub bus_speed: u64,
    /// Hz: the timebase (`arch::timebase()`) ticks this often.
    pub timebase_speed: u64,
    pub l1_cache: u32,
    pub l2_cache: u32,
    pub l3_cache: u32,
    pub cache_line: u32,
    /// VECTORTYPE_* (0: none).
    pub vector_unit: u32,
    pub extensions: &'static str,
    pub cpu_page_size: u32,
    pub exec_page_size: u32,
    /// The core the caller runs on (V54.48).
    pub current_cpu: u32,
}

fn s(p: *const c_char) -> &'static str {
    if p.is_null() {
        return "";
    }
    // SAFETY: GetCPUInfo's strings are exec's static, NUL-terminated text.
    unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("")
}

/// Everything GetCPUInfo reports, in one call.
pub fn cpu() -> CpuInfo {
    let (mut count, mut family, mut model, mut version) = (1u32, 0u32, 0u32, 0u32);
    let (mut l1, mut l2, mut l3, mut line, mut vec, mut cpage, mut epage, mut cur) = (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
    let (mut cpu_speed, mut bus_speed, mut tb) = (0u64, 0u64, 0u64);
    let (mut model_s, mut version_s, mut ext_s): (*const c_char, *const c_char, *const c_char) =
        (core::ptr::null(), core::ptr::null(), core::ptr::null());
    macro_rules! t {
        ($tag:expr, $v:expr) => {
            TagItem { ti_Tag: $tag, ti_Data: &mut $v as *mut _ as u32 }
        };
    }
    let tags = [
        t!(sys::GCIT_NUMBER_OF_CPUS, count),
        t!(sys::GCIT_FAMILY, family),
        t!(sys::GCIT_MODEL, model),
        t!(sys::GCIT_MODELSTRING, model_s),
        t!(sys::GCIT_VERSION, version),
        t!(sys::GCIT_VERSIONSTRING, version_s),
        t!(sys::GCIT_FRONTSIDESPEED, bus_speed),
        t!(sys::GCIT_PROCESSORSPEED, cpu_speed),
        t!(sys::GCIT_L1CACHESIZE, l1),
        t!(sys::GCIT_L2CACHESIZE, l2),
        t!(sys::GCIT_L3CACHESIZE, l3),
        t!(sys::GCIT_CACHE_LINE_SIZE, line),
        t!(sys::GCIT_VECTORUNIT, vec),
        t!(sys::GCIT_EXTENSIONS, ext_s),
        t!(sys::GCIT_CPUPAGESIZE, cpage),
        t!(sys::GCIT_EXECPAGESIZE, epage),
        t!(sys::GCIT_TIMEBASE_SPEED, tb),
        t!(sys::GCIT_CURRENTCPUNUM, cur),
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: every tag points at storage of the type the autodoc gives it
    // (uint32, uint64 or CONST_STRPTR) on this stack.
    unsafe { sys::exec_get_cpuinfo(tags.as_ptr()) };
    CpuInfo {
        count,
        family,
        model,
        model_name: s(model_s),
        version,
        version_name: s(version_s),
        cpu_speed,
        bus_speed,
        timebase_speed: tb,
        l1_cache: l1,
        l2_cache: l2,
        l3_cache: l3,
        cache_line: line,
        vector_unit: vec,
        extensions: s(ext_s),
        cpu_page_size: cpage,
        exec_page_size: epage,
        current_cpu: cur,
    }
}
