//! Vulkan through `vulkan.library` (feature `vulkan`).
//!
//! `vulkan.library` is the Khronos loader as an AmigaOS library; RADV
//! (`radv_vk.library`) is its driver for AMD cards. Its `main` interface
//! carries the loader's exported entry points, but the API proper is
//! reached the portable way: [`get_instance_proc_addr`] takes
//! `vkGetInstanceProcAddr` from the interface once, and that, like every
//! pointer it hands back, is an ordinary C function (no interface pointer).
//! Any Rust Vulkan binding can be built on it; with the `ash` feature,
//! [`entry`] gives an `ash::Entry`.
//!
//! - [`get_instance_proc_addr`], the loader's `vkGetInstanceProcAddr`
//!   (vulkan.library opened by the crate the first time, kept open).
//! - [`instance_version`], `vkEnumerateInstanceVersion`.
//! - VK_AMIGA_surface (the AmigaOS window system integration, not
//!   registered with Khronos): [`AmigaSurfaceCreateInfo`] and, with
//!   `intuition`, [`create_amiga_surface`] on an open window.
//! - With `ash`, [`entry`] and [`AmigaSurface`] (the extension's loader for an
//!   `ash::Instance`).
//!
//! RADV opens the card only for a process that belongs to a program,
//! which it tells by the process's C library context (pr_CLibData) or its
//! UID: use it from a std program (clib4). A program with no C runtime
//! (the crate's no-CRT examples, a driver) gets an instance and surfaces
//! but no physical device ("the calling task belongs to no program that
//! opened radv_vk.library", on an AmigaOne X5000).
//!
//! The runtime needs `vulkan.library` in LIBS:, and for a card
//! `radv_vk.library` with `SetEnv VK_DRIVER_FILES SYS:Mesa/radv_vk.json`
//! before the loader first loads; without a driver `vkCreateInstance`
//! answers `VK_ERROR_INCOMPATIBLE_DRIVER`.
//! The interface and structure layouts are asserted against the
//! vulkan.library SDK headers in `tests/layout/vulkan.c`.

use core::ffi::{c_char, c_void};

use crate::error::{AmigaError, Result};
use crate::sys;

/// A dispatchable Vulkan handle (VkInstance and the rest): a pointer.
pub type RawHandle = *mut c_void;
/// `PFN_vkVoidFunction`.
pub type VoidFunction = Option<unsafe extern "system" fn()>;
/// `PFN_vkGetInstanceProcAddr`.
pub type GetInstanceProcAddr = unsafe extern "system" fn(instance: RawHandle, name: *const c_char) -> VoidFunction;
type EnumerateInstanceVersion = unsafe extern "system" fn(version: *mut u32) -> i32;

/// `VK_AMIGA_surface`'s name.
pub const AMIGA_SURFACE_EXTENSION_NAME: &core::ffi::CStr = c"VK_AMIGA_surface";
pub const AMIGA_SURFACE_SPEC_VERSION: u32 = 1;
/// `VK_STRUCTURE_TYPE_AMIGA_SURFACE_CREATE_INFO_AMIGA` (not a Khronos
/// number; the older value 1000400000 is refused).
pub const STRUCTURE_TYPE_AMIGA_SURFACE_CREATE_INFO: i32 = 1_099_998_000;
/// `VK_KHR_surface`, which VK_AMIGA_surface needs on the instance.
pub const KHR_SURFACE_EXTENSION_NAME: &core::ffi::CStr = c"VK_KHR_surface";

/// `VkAmigaSurfaceCreateInfoAMIGA` (20 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AmigaSurfaceCreateInfo {
    pub s_type: i32,
    pub p_next: *const c_void,
    pub flags: u32,
    /// The window's screen; NULL: the window's own.
    pub screen: *mut sys::Screen,
    pub window: *mut sys::Window,
}

/// `PFN_vkCreateAmigaSurfaceAMIGA`: (instance, create info, allocator,
/// surface out) -> VkResult.
pub type CreateAmigaSurface = unsafe extern "system" fn(instance: RawHandle, info: *const AmigaSurfaceCreateInfo, allocator: *const c_void, surface: *mut u64) -> i32;

/// The head of `struct VulkanIFace` (`interfaces/vulkan.h`): the
/// four standard slots, then the loader's exports in vulkan-1.def order;
/// only as far as vkGetInstanceProcAddr, the one slot used.
#[repr(C)]
struct VulkanIFace {
    data: sys::InterfaceData,
    obtain: usize,
    release: usize,
    expunge: usize,
    clone: usize,
    create_instance: usize,
    destroy_instance: usize,
    enumerate_physical_devices: usize,
    get_physical_device_features: usize,
    get_physical_device_format_properties: usize,
    get_physical_device_image_format_properties: usize,
    get_physical_device_properties: usize,
    get_physical_device_queue_family_properties: usize,
    get_physical_device_memory_properties: usize,
    get_instance_proc_addr: unsafe extern "C" fn(*mut VulkanIFace, RawHandle, *const c_char) -> VoidFunction,
}

// vkGetInstanceProcAddr is at offset 112 of struct VulkanIFace
// (tests/layout/vulkan.c); the same here on the target.
#[cfg(target_os = "amigaos")]
const _: () = assert!(core::mem::offset_of!(VulkanIFace, get_instance_proc_addr) == 112);

fn iface() -> Result<*mut VulkanIFace> {
    static SLOT: crate::iface::Slot = crate::iface::Slot::new(b"vulkan.library\0", b"main\0", 1, 1);
    // No program global: a Rust program reaches the loader through the
    // crate's own interface.
    SLOT.get(core::ptr::null_mut()).map(|i| i as *mut VulkanIFace).ok_or(AmigaError::NullPointer)
}

/// The loader's `vkGetInstanceProcAddr`, a plain C function: hand it to a
/// Vulkan binding as its entry point. Fails when vulkan.library cannot be
/// opened.
pub fn get_instance_proc_addr() -> Result<GetInstanceProcAddr> {
    let i = iface()?;
    // SAFETY: an open vulkan.library interface; NULL instance and a
    // NUL-terminated name are a global query, which the loader answers
    // with its own trampoline.
    let f = unsafe { ((*i).get_instance_proc_addr)(i, core::ptr::null_mut(), c"vkGetInstanceProcAddr".as_ptr()) };
    match f {
        // SAFETY: the loader returned vkGetInstanceProcAddr for that name.
        Some(f) => Ok(unsafe { core::mem::transmute::<unsafe extern "system" fn(), GetInstanceProcAddr>(f) }),
        None => Err(AmigaError::Unsupported("vulkan.library gave no vkGetInstanceProcAddr")),
    }
}

/// `vkEnumerateInstanceVersion`: the loader's Vulkan version
/// (VK_MAKE_API_VERSION packing; see [`version_parts`]).
pub fn instance_version() -> Result<u32> {
    let gipa = get_instance_proc_addr()?;
    // SAFETY: a global query for a global command.
    let f = unsafe { gipa(core::ptr::null_mut(), c"vkEnumerateInstanceVersion".as_ptr()) }.ok_or(AmigaError::Unsupported("no vkEnumerateInstanceVersion"))?;
    // SAFETY: the loader's vkEnumerateInstanceVersion, its signature.
    let f = unsafe { core::mem::transmute::<unsafe extern "system" fn(), EnumerateInstanceVersion>(f) };
    let mut v = 0u32;
    // SAFETY: a u32 to write.
    let r = unsafe { f(&mut v) };
    if r == 0 { Ok(v) } else { Err(AmigaError::IoError(r)) }
}

/// A packed Vulkan version as (variant, major, minor, patch).
pub const fn version_parts(v: u32) -> (u32, u32, u32, u32) {
    (v >> 29, (v >> 22) & 0x7F, (v >> 12) & 0x3FF, v & 0xFFF)
}

impl AmigaSurfaceCreateInfo {
    /// For a window (its own screen).
    #[cfg(feature = "intuition")]
    pub fn for_window(window: &crate::intuition::Window<'_>) -> Self {
        Self { s_type: STRUCTURE_TYPE_AMIGA_SURFACE_CREATE_INFO, p_next: core::ptr::null(), flags: 0, screen: core::ptr::null_mut(), window: window.as_ptr() }
    }
}

/// `vkCreateAmigaSurfaceAMIGA` on `window` for `instance` (made with
/// VK_KHR_surface and VK_AMIGA_surface enabled): the VkSurfaceKHR, or the
/// VkResult as `IoError`. The caller destroys the surface
/// (vkDestroySurfaceKHR) before the window closes.
///
/// # Safety
/// `instance` must be a live VkInstance created through `gipa` with the
/// two extensions enabled.
#[cfg(feature = "intuition")]
pub unsafe fn create_amiga_surface(gipa: GetInstanceProcAddr, instance: RawHandle, window: &crate::intuition::Window<'_>) -> Result<u64> {
    // SAFETY: the caller's live instance; an instance-level query.
    let f = unsafe { gipa(instance, c"vkCreateAmigaSurfaceAMIGA".as_ptr()) }.ok_or(AmigaError::Unsupported("the instance has no VK_AMIGA_surface"))?;
    // SAFETY: the loader's vkCreateAmigaSurfaceAMIGA, its signature.
    let f = unsafe { core::mem::transmute::<unsafe extern "system" fn(), CreateAmigaSurface>(f) };
    let info = AmigaSurfaceCreateInfo::for_window(window);
    let mut s = 0u64;
    // SAFETY: the caller's instance, a filled create info for an open
    // window (borrowed for the call), no allocator, a u64 to write.
    let r = unsafe { f(instance, &info, core::ptr::null(), &mut s) };
    if r == 0 { Ok(s) } else { Err(AmigaError::IoError(r)) }
}

/// An `ash::Entry` on vulkan.library's loader.
#[cfg(feature = "ash")]
pub fn entry() -> Result<ash::Entry> {
    let gipa = get_instance_proc_addr()?;
    // SAFETY: the two function pointer types have the same signature
    // (ash's vk::Instance is a pointer-sized handle, *const c_char the
    // name, PFN_vkVoidFunction the result), the ABI "system".
    let gipa = unsafe { core::mem::transmute::<GetInstanceProcAddr, ash::vk::PFN_vkGetInstanceProcAddr>(gipa) };
    // SAFETY: a valid vkGetInstanceProcAddr, which ash loads the global
    // commands through.
    Ok(unsafe { ash::Entry::from_static_fn(ash::StaticFn { get_instance_proc_addr: gipa }) })
}

/// VK_AMIGA_surface for an `ash::Instance`.
#[cfg(feature = "ash")]
pub struct AmigaSurface {
    instance: ash::vk::Instance,
    create: CreateAmigaSurface,
}

#[cfg(feature = "ash")]
impl AmigaSurface {
    /// Load the extension's command; fails when the instance was made
    /// without it.
    pub fn new(entry: &ash::Entry, instance: &ash::Instance) -> Result<Self> {
        // SAFETY: a live instance from that entry; an instance-level query.
        let f = unsafe { (entry.static_fn().get_instance_proc_addr)(instance.handle(), c"vkCreateAmigaSurfaceAMIGA".as_ptr()) }.ok_or(AmigaError::Unsupported("the instance has no VK_AMIGA_surface"))?;
        // SAFETY: the loader's vkCreateAmigaSurfaceAMIGA, its signature.
        let create = unsafe { core::mem::transmute::<unsafe extern "system" fn(), CreateAmigaSurface>(f) };
        Ok(Self { instance: instance.handle(), create })
    }

    /// A surface on `window`; destroy it (ash's `khr::surface`) before
    /// the window closes.
    ///
    /// # Safety
    /// The instance this was made for must still be alive.
    #[cfg(feature = "intuition")]
    pub unsafe fn create(&self, window: &crate::intuition::Window<'_>) -> core::result::Result<ash::vk::SurfaceKHR, ash::vk::Result> {
        use ash::vk::Handle;
        let info = AmigaSurfaceCreateInfo::for_window(window);
        let mut s = 0u64;
        // SAFETY: the live instance (caller), a filled create info for an
        // open window, no allocator, a u64 to write.
        let r = unsafe { (self.create)(self.instance.as_raw() as usize as RawHandle, &info, core::ptr::null(), &mut s) };
        if r == 0 { Ok(ash::vk::SurfaceKHR::from_raw(s)) } else { Err(ash::vk::Result::from_raw(r)) }
    }
}
