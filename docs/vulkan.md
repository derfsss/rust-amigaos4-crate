# Vulkan: `vulkan`

The `vulkan` module connects Rust to Vulkan on AmigaOS 4.1. Vulkan is the
cross-platform API for GPU graphics and compute. On AmigaOS it comes as
`vulkan.library`, the Khronos loader packaged as an AmigaOS library, and
drivers such as RADV (`radv_vk.library`, Mesa's driver for AMD Radeon
cards) behind it.

The module gives you the loader's entry point, the Vulkan version, and
the AmigaOS window system extension, **VK_AMIGA_surface**, which makes a
Vulkan surface on an Intuition window. With the `ash` feature it hands
you an `ash::Entry`, so everything else is ordinary
[ash](https://crates.io/crates/ash) code.

## Cargo features

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["intuition", "graphics", "vulkan", "ash"] }
ash = { version = "0.38", default-features = false }
```

| Feature | Adds |
|---|---|
| `vulkan` | `get_instance_proc_addr`, `instance_version`, `version_parts`, `AmigaSurfaceCreateInfo`, the extension name constants |
| `vulkan` + `intuition` | `AmigaSurfaceCreateInfo::for_window`, `create_amiga_surface` (surfaces on a `intuition::Window`) |
| `ash` | `entry()` and `AmigaSurface`; pulls in `vulkan` and the `ash` crate (0.38, no default features) |
| `ash` + `intuition` | `AmigaSurface::create` |

Add `ash` to your own `[dependencies]` too, as above, so you can write
`use ash::vk;`. The example programs also turn on `graphics`.

## What is needed at run time

On the Amiga:

1. **vulkan.library** in `LIBS:`. Without it, `get_instance_proc_addr`
   and `entry` fail (the crate opens the library the first time).
2. **A driver**, such as RADV: `radv_vk.library` in `LIBS:` and an AMD
   card it supports.
3. **`VK_DRIVER_FILES`** naming the driver's manifest, set before the
   loader first loads, for example:

   ```
   SetEnv SAVE VK_DRIVER_FILES SYS:Mesa/radv_vk.json
   ```

   The loader reads the variable only when it first loads, so changing it
   needs a reboot. Without a driver, `vkCreateInstance` answers
   `VK_ERROR_INCOMPATIBLE_DRIVER`.
4. **A big stack.** The loader and the driver run on the caller's stack.
   The example programs ask for 1 MB:

   ```rust
   #[used]
   static STACK: [u8; 15] = *b"$STACK:1048576\0";
   ```

## Why a device needs a `std` (clib4) program

RADV opens the card only for a process that belongs to a program, which
it tells by the process's C library context (or its UID). A program
built without a C runtime (a `#![no_std]` program, a driver) gets an
instance and can make surfaces, but sees **no physical devices**; RADV
logs "the calling task belongs to no program that opened
radv_vk.library".

So write Vulkan programs as ordinary `std` programs on clib4: `fn main`,
the crate's default features, linked by cargo. Start from
`templates/program` (see "Programs" in the repository README). They need
clib4.library 2.5 or newer on the Amiga.

## The pieces

The snippets below sit in a function returning
`Result<(), Box<dyn std::error::Error>>`: the crate's `AmigaError` converts
with `?`, and Vulkan results are turned into text with `map_err`.

### Entry point: `get_instance_proc_addr`

```rust
let gipa = amigaos4::vulkan::get_instance_proc_addr()?;
```

This opens vulkan.library (once, kept open) and returns the loader's
`vkGetInstanceProcAddr` as a plain C function pointer
(`GetInstanceProcAddr`). Every function it hands back is an ordinary C
function too, so any Rust Vulkan binding can be built on it. Most
programs use `entry()` instead.

### Version: `instance_version` and `version_parts`

```rust
use amigaos4::vulkan;

let v = vulkan::instance_version()?;          // vkEnumerateInstanceVersion
let (_variant, major, minor, patch) = vulkan::version_parts(v);
println!("Vulkan {}.{}.{}", major, minor, patch);
```

### ash: `entry()`

```rust
let entry = amigaos4::vulkan::entry()?;   // an ash::Entry on vulkan.library
```

From here on it is ash as on any platform: `entry.create_instance`,
`instance.enumerate_physical_devices`, and so on.

### VK_AMIGA_surface

A *surface* is what Vulkan presents images to. On AmigaOS it is made on
an Intuition window with the `VK_AMIGA_surface` extension, which needs
`VK_KHR_surface` too. Enable both on the instance:

```rust
use amigaos4::vulkan::{AMIGA_SURFACE_EXTENSION_NAME, KHR_SURFACE_EXTENSION_NAME};
use ash::vk;

let exts = [KHR_SURFACE_EXTENSION_NAME.as_ptr(), AMIGA_SURFACE_EXTENSION_NAME.as_ptr()];
let app = vk::ApplicationInfo::default().application_name(c"myprog").api_version(vk::API_VERSION_1_1);
let ci = vk::InstanceCreateInfo::default().application_info(&app).enabled_extension_names(&exts);
// SAFETY: a valid create info; the instance is destroyed before the entry goes.
let instance = unsafe { entry.create_instance(&ci, None) }.map_err(|r| format!("vkCreateInstance: {}", r))?;
```

Then make the surface on a window with `AmigaSurface`:

```rust
use amigaos4::intuition::{idcmp, PubScreen, Window, WindowOptions};
use amigaos4::vulkan::AmigaSurface;

let screen = PubScreen::default_screen()?;
let win = Window::open(&screen, &WindowOptions {
    title: "Vulkan",
    inner_width: 640,
    inner_height: 400,
    idcmp: idcmp::CLOSEWINDOW | idcmp::NEWSIZE | idcmp::VANILLAKEY,
    size_gadget: true,
    ..Default::default()
})?;
let amiga = AmigaSurface::new(&entry, &instance)?;         // fails if the extension is not enabled
// SAFETY: the instance is alive until after the surface is destroyed.
let surface: vk::SurfaceKHR = unsafe { amiga.create(&win) }.map_err(|r| format!("{}", r))?;
```

- `AmigaSurface::new(&entry, &instance)` loads
  `vkCreateAmigaSurfaceAMIGA` for that instance.
- `unsafe fn create(&self, &Window) -> Result<vk::SurfaceKHR,
  vk::Result>` makes the surface. It is unsafe because the instance must
  still be alive.
- **Destroy the surface before the window closes**, with ash's
  `khr::surface::Instance::destroy_surface`.

Without ash, the same is available as
`unsafe fn create_amiga_surface(gipa, instance, &window) -> Result<u64>`
(the raw `VkSurfaceKHR`, or the `VkResult` as `AmigaError::IoError`),
and `AmigaSurfaceCreateInfo` is the C structure
(`VkAmigaSurfaceCreateInfoAMIGA`) with `s_type`, `p_next`, `flags`,
`screen` (null: the window's own) and `window`. Its structure type is
`STRUCTURE_TYPE_AMIGA_SURFACE_CREATE_INFO`; `for_window(&win)` fills one
in.

## Walk-through: `apps/vkinfo`

`apps/vkinfo` prints what Vulkan offers on the machine. It is about 200
lines and a good first read.

1. `vulkan::entry()` opens the loader; failure gives return code 20.
2. With ash it prints the loader's version
   (`try_enumerate_instance_version`), the instance extensions and
   layers.
3. It enables `VK_KHR_surface` and `VK_AMIGA_surface` only if the loader
   lists both, and makes an instance. On
   `VK_ERROR_INCOMPATIBLE_DRIVER` it explains that no driver is
   installed (return code 5).
4. For each physical device it prints name, type, versions, vendor and
   device ids, queue families and memory heaps. No devices: return code
   5.
5. With the surface extensions it opens a 320x200 `intuition::Window`,
   makes a surface with `AmigaSurface`, and for each device prints the
   queue families that can present to it, the image count and size
   limits, the formats and the present modes (FIFO, MAILBOX, IMMEDIATE).
6. It destroys the surface, then closes the window, then destroys the
   instance.

Its rule for clean-up order is the one to copy: everything made from the
instance goes before the instance, and the surface goes before its
window.

## Walk-through: `apps/vkplasma`

`apps/vkplasma` draws an animated plasma in a window: a real swapchain
and render loop. Keys: `p` cycles the present mode, `q` or Esc quits.

1. **Arguments** are read with ReadArgs (`dos::args::Args`), template
   `FRAMES/K/N,MAILBOX/S,IMMEDIATE/S`.
2. **Window**: a resizable 640x400 `intuition::Window` with
   `CLOSEWINDOW | NEWSIZE | VANILLAKEY`.
3. **Set-up** (`make_gpu`): an instance with the two surface extensions,
   a surface from `AmigaSurface`, then the first device with a queue
   family that both draws (`GRAPHICS`) and presents to the surface. A
   device with `VK_KHR_swapchain`, a surface format (B8G8R8A8_UNORM if
   offered), the present mode, a render pass, and a pipeline whose
   vertex shader makes one triangle covering the window.
4. **Shaders** are compiled ahead of time with glslc and included with
   `include_bytes!`. glslc writes SPIR-V little-endian, so the program
   turns the bytes into host-order `u32` words before
   `vkCreateShaderModule` (the PowerPC is big-endian).
5. **Render loop** (`run`): first drain the window's events with
   `win.next()`. `NewSize` or a present mode change marks the swapchain
   stale; it is then made again from the surface's current size (or
   `win.inner()` when the surface does not say). A window with no area
   waits with `amigaos4::signal::wait(win.signal_mask())`. CTRL-C is
   checked with `amigaos4::signal::take(SIGBREAKF_CTRL_C)`. Each frame
   waits for its fence, acquires an image, records and submits, and
   presents; two frames are in flight.
6. **Frame rate** is shown in the window title with `win.set_title`.
7. **Clean-up**: wait for the device to go idle, destroy everything in
   reverse order, the surface before the instance, and the window last.

In FIFO mode the program shows one frame per vertical blank. In MAILBOX
and IMMEDIATE the loop never waits, so it uses all the CPU time it gets
and other tasks of the same priority barely run meanwhile.

## Limitations

- A physical device needs a `std` (clib4) program (see above).
- `VK_DRIVER_FILES` is read once, when the loader first loads.
- VK_AMIGA_surface is an AmigaOS extension, not registered with Khronos;
  its structure type number is AmigaOS's own (an older value is refused).
- The module does not wrap the Vulkan API itself; use ash (or another
  binding built on `get_instance_proc_addr`).
