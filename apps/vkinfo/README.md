# vkinfo

What Vulkan offers on this machine, through `vulkan.library`,
written in Rust with the `amigaos4` crate (features `vulkan` and `ash`)
and [ash](https://crates.io/crates/ash). An ordinary `std` program on
clib4, about 195 KB.

```
vkinfo
```

It prints:

- the loader's Vulkan version, instance extensions and layers;
- whether an instance can be made (with VK_KHR_surface and
  VK_AMIGA_surface when they are offered);
- each physical device: name, type, Vulkan and driver versions, vendor
  and device ids, queue families, memory heaps;
- with the surface extensions: a VK_AMIGA_surface made on a small
  Intuition window, and per device the queue families that present to
  it, its image count and size limits, formats and present modes.

Return code 0 with a device, 5 without one (no driver, or no card), 10
when the surface could not be made, 20 when vulkan.library does not open.

## Why std (clib4)

RADV opens the card only for a process that belongs to a program, which
it tells by the process's C library context or its UID. Built without a
C runtime (as the crate's other examples are), vkinfo got an instance
and a surface but no device, and RADV logged "the calling task belongs
to no program that opened radv_vk.library". So it is an ordinary Rust
program on clib4: `fn main`, linked by cargo through tools/ppc-amigaos-gcc
(see "Programs" in the repository README). It needs clib4.library 2.5 or
newer.

## Running it

Copy `vkinfo` to the Amiga and run it from a Shell. It needs a Vulkan
loader, `vulkan.library`, in LIBS:. To see a card it also needs the
RADV driver, `radv_vk.library`, in LIBS:, and its manifest named before
the loader first loads, for example:

```
SetEnv SAVE VK_DRIVER_FILES SYS:Mesa/radv_vk.json
```

The loader reads VK_DRIVER_FILES only when it first loads, so a change
needs a reboot. The binary asks for a 1 MB stack (`$STACK`).

## How it reaches Vulkan

`amigaos4::vulkan::entry()` opens vulkan.library, takes the loader's
`vkGetInstanceProcAddr` from its interface (a plain C function) and
builds an `ash::Entry` on it; everything after that is ordinary ash.
`amigaos4::vulkan::AmigaSurface` loads `vkCreateAmigaSurfaceAMIGA` for
the instance and makes the surface from a crate `Window`.

## Tested

- AmigaOne X5000 with a Radeon RX 590, vulkan.library 1.4.354 and
  radv_vk.library (Mesa 26.2.3): Radeon RX 590 (RADV POLARIS10),
  discrete GPU, Vulkan 1.4.354, vendor 0x1002 device 0x67df; queue
  families graphics+compute+transfer+sparse x1, compute x8, video decode
  x1, sparse x1; heaps 7936 MB and 256 MB device-local, 1022 MB host; a
  VK_AMIGA_surface on a 320x200 window: family 0 presents, extent
  320x200, format 44 (B8G8R8A8_UNORM), FIFO, MAILBOX and IMMEDIATE;
  rc 0.
- QEMU AmigaOne (no AMD card): with no driver, 4 instance extensions and
  `vkCreateInstance` -9; with radv_vk.library, the instance and the
  surface, 0 devices, rc 5.

Not done: a swapchain and rendering (see `apps/vkplasma`).
