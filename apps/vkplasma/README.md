# vkplasma

An animated plasma drawn with Vulkan into an Intuition window: a
swapchain on a VK_AMIGA_surface, through `vulkan.library`,
the `amigaos4` crate (features `vulkan` and `ash`) and ash. A `std`
program on clib4, about 315 KB.

```
vkplasma [FRAMES=n] [MAILBOX] [IMMEDIATE]      (FRAMES/K/N,MAILBOX/S,IMMEDIATE/S)
```

| Key | |
|---|---|
| `p` | next present mode: FIFO, MAILBOX, IMMEDIATE (those the surface offers); the swapchain is made again |
| `q`, Esc, the close gadget, CTRL-C | quit |

- FIFO (the default) shows one frame per vertical blank. MAILBOX and
  IMMEDIATE do not wait for it, so the frame rate is the GPU's;
  IMMEDIATE may tear.
- Resizing the window makes the swapchain again at the new size.
- The frame rate is printed every 2 seconds, shown in the window title,
  and printed for the whole run at the end. `FRAMES=n` stops after n
  frames.

## How it draws

One triangle covers the window; the vertex shader makes its corners
from the vertex index (no vertex buffer). The fragment shader
(`shaders/plasma.frag`) sums four sine waves of the position and time
and colours them through a cosine palette. Time and aspect ratio are
two 32-bit floats of push constants, written in host order (the driver
converts what the GPU reads). A render pass with one colour attachment,
dynamic viewport and scissor, two frames in flight (a fence and an
acquire semaphore each) and a render-done semaphore per swapchain
image.

The shaders are compiled with the Vulkan SDK's glslc and the SPIR-V is
kept beside the GLSL:

```
glslc --target-env=vulkan1.1 -O plasma.vert -o plasma.vert.spv
glslc --target-env=vulkan1.1 -O plasma.frag -o plasma.frag.spv
```

glslc writes SPIR-V little-endian; the program turns the bytes into
host-order words before vkCreateShaderModule.

## Why std (clib4)

RADV opens the card only for a process with a C library context (see
`apps/vkinfo`). It is an ordinary Rust program on clib4 (`fn main`,
linked by cargo; see "Programs" in the repository README) and reads its
command line with ReadArgs (`dos::args::Args`). It needs clib4.library
2.5 or newer.

## Running it

Copy `vkplasma` to the Amiga and run it from a Shell. It needs
`vulkan.library` and `radv_vk.library` set up as described in
`apps/vkinfo`'s README, and an AMD card that RADV supports.

## Tested

On an AmigaOne X5000 with a Radeon RX 590, vulkan.library 1.4.354 and
radv_vk.library (Mesa 26.2.3). A 640x400 window, 3 swapchain images,
B8G8R8A8_UNORM:

| mode | frames | frame rate |
|---|---|---|
| FIFO | 600 | 58.8 fps over the run, 60 fps steady |
| IMMEDIATE | 3000 | 747 fps |
| MAILBOX | 3000 | 1800 fps |

Pressing `p` while it ran went FIFO, MAILBOX (1750-2030 fps), IMMEDIATE
(650-670 fps), FIFO (60 fps), the swapchain made 4 times, and `q`
ended it with return code 0.

In MAILBOX and IMMEDIATE the render loop never waits, so a task of the
same priority barely runs meanwhile (a `Wait 5` in another Shell took
10 s). Resizing the window was not exercised.
