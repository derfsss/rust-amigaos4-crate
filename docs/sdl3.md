# SDL3 in Rust programs

[SDL3](https://www.libsdl.org/) (windows, rendering, input, gamepads,
audio, timers) works in Rust programs for AmigaOS 4 through the ordinary
crates.io bindings: [`sdl3-sys`](https://crates.io/crates/sdl3-sys) (raw)
and [`sdl3`](https://crates.io/crates/sdl3) (safe). Nothing in this crate
is needed for it: a program links the AmigaOS 4 build of SDL3 that is
already in the cross-compiler image.

Examples: `testprogs/sdl3-hello` (raw bindings) and `testprogs/sdl3-demo`
(the safe crate: a window, a texture, keys, mouse, a tone).

## What the image has

`walkero/amigagccondocker:os4-gcc15-3.6.1` carries SDL3 built for clib4
(and newlib) by the [AmigaPorts](https://github.com/AmigaPorts/SDL)
port:

| Library | Version | File (under `/opt/ppc-amigaos/ppc-amigaos/SDK/local/clib4/`) |
|---|---|---|
| SDL3 | 3.4.18 | `lib/libSDL3.a`, headers `include/SDL3` |
| SDL3_image | 3.4.4 | `lib/libSDL3_image.a` |
| SDL3_ttf | 3.2.2 | `lib/libSDL3_ttf.a` |
| SDL3_gfx | | `lib/libSDL3_gfx.a` |

`libSDL3.a` needs no other library. Its renderers: compositing
(graphics.library, the default on real hardware), OpenGL (Mesa), OpenGL ES
2 (ogles2.library) and software. There is no Vulkan or SDL GPU support on
AmigaOS 4.

## Setting up a program

Start from `templates/program` (a `std` program on clib4) and add:

```toml
[dependencies]
# The safe crate (no default features: nothing to build from source).
sdl3 = { version = "0.22.1", default-features = false }
# Or only the raw bindings:
# sdl3-sys = { version = "0.7.3", default-features = false }

# crates.io's libc does not know AmigaOS (clib4). The toolchain's std uses
# AmigaLabs' fork; every crate in the program that uses libc (sdl3 does)
# must use it too. Same commit as the toolchain.
[patch.crates-io]
libc = { git = "https://github.com/AmigaLabs/libc", rev = "4982c2db17c19cb599ec29c268651bbdec324114" }
```

- `default-features = false` turns off pkg-config and vcpkg. The bindings
  then ask the linker for `-lSDL3`, and the image's `ppc-amigaos-gcc
  -mcrt=clib4 -static` finds `libSDL3.a` on its own search path: no
  `build.rs`, no link arguments.
- `sdl3-sys` 0.7.3 is generated from SDL 3.4.20; the image has 3.4.18 (the
  same 3.4 series). A function added in a later 3.4 release would fail to
  link.
- If a lock file already has crates.io's `libc`, delete `Cargo.lock` after
  adding the patch (cargo reports the patch as unused otherwise).

Build as any program: `cargo +amigaos build --release`.

## Running

- The programs need **clib4.library 2.5** (they print "This program
  requires clib4.library version 2.5" on older ones).
- On QEMU there is no hardware compositing: SDL uses its software
  renderer (about 37 fps for a 640x400 window with one sprite on the QEMU
  AmigaOne). SDL's AmigaOS notes suggest patchcompositetags (OS4Depot) on
  emulators.
- Audio uses SDL's `amigaos4` driver (AHI).

Checked on the QEMU AmigaOne with clib4 2.5: video driver `amigaos4`,
alpha blending and colour modulation of textures on the software
renderer, keyboard, mouse buttons with window coordinates, audio opened,
free memory the same after the program as before.

## Known quirks

- A mouse button pressed before the pointer has moved over the window
  reports position 0,0; after a movement the coordinates are right.
- On QEMU, the first events after a window opens can include Alt and
  Control key presses that nobody made.
