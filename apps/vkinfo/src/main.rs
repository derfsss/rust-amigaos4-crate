//! vkinfo: what Vulkan offers on this machine, through
//! vulkan.library, written with the amigaos4 crate and ash.
//!
//!   vkinfo
//!
//! Prints the loader's Vulkan version, its instance extensions and layers,
//! then makes an instance (with VK_KHR_surface and VK_AMIGA_surface when
//! the loader offers them) and lists each physical device: name, type,
//! versions, queue families and memory heaps. With the surface extension
//! it opens a small window, makes a VK_AMIGA_surface on it and lists, per
//! device, the queue families that can present to it and the surface's
//! formats, present modes and size limits.
//!
//! Without a driver (no radv_vk.library, or VK_DRIVER_FILES unset) the
//! instance answers VK_ERROR_INCOMPATIBLE_DRIVER; with RADV and no AMD
//! card it has no physical devices. Both are reported, return code 5.
//!
//! An ordinary std program on clib4: RADV opens the card only for a
//! process that belongs to a program, which it tells by the process's C
//! library context (or its UID); a process with no C runtime is refused
//! ("the calling task belongs to no program that opened radv_vk.library").

use std::ffi::CStr;

use amigaos4::intuition::{idcmp, PubScreen, Window, WindowOptions};
use amigaos4::vulkan::{self, AmigaSurface, AMIGA_SURFACE_EXTENSION_NAME, KHR_SURFACE_EXTENSION_NAME};
use ash::{khr, vk};

/// The Vulkan loader and driver run on the caller's stack.
#[used]
static STACK: [u8; 15] = *b"$STACK:1048576\0";

macro_rules! say {
    ($($t:tt)*) => { println!($($t)*) };
}

fn version(v: u32) -> String {
    let (_, a, b, c) = vulkan::version_parts(v);
    format!("{}.{}.{}", a, b, c)
}

fn text(c: Result<&CStr, impl core::fmt::Debug>) -> String {
    c.map(|s| String::from(s.to_str().unwrap_or("?"))).unwrap_or_else(|_| String::from("?"))
}

fn device_type(t: vk::PhysicalDeviceType) -> &'static str {
    match t {
        vk::PhysicalDeviceType::INTEGRATED_GPU => "integrated GPU",
        vk::PhysicalDeviceType::DISCRETE_GPU => "discrete GPU",
        vk::PhysicalDeviceType::VIRTUAL_GPU => "virtual GPU",
        vk::PhysicalDeviceType::CPU => "CPU",
        _ => "other",
    }
}

fn queue_flags(f: vk::QueueFlags) -> String {
    let mut s = String::new();
    for (bit, name) in [(vk::QueueFlags::GRAPHICS, "graphics"), (vk::QueueFlags::COMPUTE, "compute"), (vk::QueueFlags::TRANSFER, "transfer"), (vk::QueueFlags::SPARSE_BINDING, "sparse"), (vk::QueueFlags::VIDEO_DECODE_KHR, "video-decode")] {
        if f.contains(bit) {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(name);
        }
    }
    s
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let entry = match vulkan::entry() {
        Ok(e) => e,
        Err(e) => {
            say!("vkinfo: cannot open vulkan.library: {}", e);
            return 20;
        }
    };
    // SAFETY (each ash call below): the entry's own commands, called with
    // the arguments the Vulkan specification gives them; every object made
    // is destroyed before the entry goes.
    let api = unsafe { entry.try_enumerate_instance_version() }.ok().flatten().unwrap_or(vk::API_VERSION_1_0);
    say!("loader: Vulkan {}", version(api));

    let exts = unsafe { entry.enumerate_instance_extension_properties(None) }.unwrap_or_default();
    say!("instance extensions: {}", exts.len());
    for e in &exts {
        say!("  {} (version {})", text(e.extension_name_as_c_str()), e.spec_version);
    }
    let layers = unsafe { entry.enumerate_instance_layer_properties() }.unwrap_or_default();
    say!("instance layers: {}", layers.len());
    for l in &layers {
        say!("  {} (Vulkan {})", text(l.layer_name_as_c_str()), version(l.spec_version));
    }

    let has = |n: &CStr| exts.iter().any(|e| e.extension_name_as_c_str() == Ok(n));
    let surface_ext = has(KHR_SURFACE_EXTENSION_NAME) && has(AMIGA_SURFACE_EXTENSION_NAME);
    let mut enable: Vec<*const core::ffi::c_char> = Vec::new();
    if surface_ext {
        enable.push(KHR_SURFACE_EXTENSION_NAME.as_ptr());
        enable.push(AMIGA_SURFACE_EXTENSION_NAME.as_ptr());
    }
    let app = vk::ApplicationInfo::default().application_name(c"vkinfo").application_version(1).engine_name(c"amigaos4 crate").api_version(vk::API_VERSION_1_1);
    let ci = vk::InstanceCreateInfo::default().application_info(&app).enabled_extension_names(&enable);
    let instance = match unsafe { entry.create_instance(&ci, None) } {
        Ok(i) => i,
        Err(r) => {
            say!("vkCreateInstance: {} ({})", r, r.as_raw());
            if r == vk::Result::ERROR_INCOMPATIBLE_DRIVER {
                say!("no Vulkan driver: install radv_vk.library and SetEnv VK_DRIVER_FILES to its .json file");
            }
            return 5;
        }
    };
    say!("instance made{}", if surface_ext { " with VK_KHR_surface and VK_AMIGA_surface" } else { "" });

    let devices = unsafe { instance.enumerate_physical_devices() }.unwrap_or_default();
    say!("physical devices: {}", devices.len());
    for (k, &pd) in devices.iter().enumerate() {
        let p = unsafe { instance.get_physical_device_properties(pd) };
        say!("device {}: {} ({}), Vulkan {}, driver {:#x}, vendor {:#06x} device {:#06x}", k, text(p.device_name_as_c_str()), device_type(p.device_type), version(p.api_version), p.driver_version, p.vendor_id, p.device_id);
        let qf = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        for (i, q) in qf.iter().enumerate() {
            say!("  queue family {}: {} x{}", i, queue_flags(q.queue_flags), q.queue_count);
        }
        let mem = unsafe { instance.get_physical_device_memory_properties(pd) };
        for h in 0..mem.memory_heap_count as usize {
            let heap = mem.memory_heaps[h];
            say!("  heap {}: {} MB{}", h, heap.size >> 20, if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) { " device-local" } else { "" });
        }
    }

    let mut rc = if devices.is_empty() { 5 } else { 0 };
    if surface_ext {
        if !surface_report(&entry, &instance, &devices) {
            rc = 10;
        }
    }
    unsafe { instance.destroy_instance(None) };
    say!("instance destroyed");
    rc
}

/// A window, a VK_AMIGA_surface on it, what each device says about it;
/// false when the surface could not be made.
fn surface_report(entry: &ash::Entry, instance: &ash::Instance, devices: &[vk::PhysicalDevice]) -> bool {
    let Ok(scr) = PubScreen::default_screen() else {
        say!("surface: no public screen");
        return false;
    };
    let Ok(win) = Window::open(&scr, &WindowOptions { title: "vkinfo surface", left: 60, top: 60, inner_width: 320, inner_height: 200, idcmp: idcmp::CLOSEWINDOW, activate: false, ..Default::default() }) else {
        say!("surface: no window");
        return false;
    };
    let amiga = match AmigaSurface::new(entry, instance) {
        Ok(a) => a,
        Err(e) => {
            say!("surface: {}", e);
            return false;
        }
    };
    // SAFETY: the instance is alive until the caller destroys it, after
    // this surface.
    let surface = match unsafe { amiga.create(&win) } {
        Ok(s) => s,
        Err(r) => {
            say!("vkCreateAmigaSurfaceAMIGA: {} ({})", r, r.as_raw());
            return false;
        }
    };
    say!("surface made on the window");
    let si = khr::surface::Instance::new(entry, instance);
    for (k, &pd) in devices.iter().enumerate() {
        let n = unsafe { instance.get_physical_device_queue_family_properties(pd) }.len() as u32;
        let present: Vec<u32> = (0..n).filter(|&i| unsafe { si.get_physical_device_surface_support(pd, i, surface) }.unwrap_or(false)).collect();
        say!("device {}: queue families that present: {:?}", k, present);
        if let Ok(c) = unsafe { si.get_physical_device_surface_capabilities(pd, surface) } {
            say!("  images {}..{}, extent {}x{} (min {}x{}, max {}x{})", c.min_image_count, c.max_image_count, c.current_extent.width, c.current_extent.height, c.min_image_extent.width, c.min_image_extent.height, c.max_image_extent.width, c.max_image_extent.height);
        }
        let formats = unsafe { si.get_physical_device_surface_formats(pd, surface) }.unwrap_or_default();
        for f in &formats {
            say!("  format {} colour space {}", f.format.as_raw(), f.color_space.as_raw());
        }
        let modes = unsafe { si.get_physical_device_surface_present_modes(pd, surface) }.unwrap_or_default();
        let names: Vec<&str> = modes
            .iter()
            .map(|m| match *m {
                vk::PresentModeKHR::FIFO => "FIFO",
                vk::PresentModeKHR::MAILBOX => "MAILBOX",
                vk::PresentModeKHR::IMMEDIATE => "IMMEDIATE",
                vk::PresentModeKHR::FIFO_RELAXED => "FIFO_RELAXED",
                _ => "other",
            })
            .collect();
        say!("  present modes: {:?}", names);
    }
    // The surface goes before the window.
    unsafe { si.destroy_surface(surface, None) };
    say!("surface destroyed");
    drop(win);
    true
}
