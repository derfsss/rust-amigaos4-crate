# amigaos4 crate documentation

Start here. This folder is the user guide for the `amigaos4` Rust crate.

## What the crate is

`amigaos4` lets you write software for **AmigaOS 4.1** (PowerPC) in Rust.
It is built for the `powerpc-unknown-amigaos` target of the AmigaLabs Rust
compiler (a fork of Rust, branch `amigaos-port`). That compiler already
gives programs Rust's `std` on top of **clib4**, an AmigaOS C library. The
crate adds what the compiler does not have:

- **raw bindings** (`sys`) for 130 AmigaOS SDK interfaces: exec, DOS,
  Intuition, graphics and the rest;
- **safe wrappers** that open, use and release OS resources the Rust way:
  files, windows, menus, ReAction GUIs, requesters, tasks, message ports,
  sockets, devices, Vulkan and more;
- **support for code with no C runtime**: an exec-based global allocator,
  a panic handler, debug output and the C glue needed to build device
  drivers, shared libraries and Kickstart modules in Rust.

The crate is at version 0.1. Its interfaces may still change.

## Who it is for

Developers who know some Rust and want to write AmigaOS 4 software. You do
not need to know AmigaOS well: [concepts.md](concepts.md) explains the OS
ideas you meet, and each guide explains AmigaOS terms as they come up.
Experienced Amiga developers will find the raw `sys` layer there when they
need it.

## Two ways to build

| | Programs | Drivers, libraries, devices |
|---|---|---|
| What | Ordinary executables started from the Shell or Workbench | `.library`, `.device`, handlers, resources, Kickstart modules, and Shell programs built the same way |
| Rust | `fn main` with `std` (`println!`, `std::fs`, threads) | `#![no_std]`, `alloc`, a static library |
| C library | clib4 (needs clib4.library 2.5+ on the Amiga) | none |
| Cargo features | defaults (`std`, `dos`, `utility`) plus what you use | `default-features = false` plus what you use |
| Allocator and panic handler | from `std` | you choose: `allocator::SharedExecAllocator` or `ExecAllocator`, `panic::default_panic_handler` |
| Linked by | cargo, through `tools/ppc-amigaos-gcc` | the package's Makefile, with the C glue |
| Template | `templates/program` | `templates/driver`, `templates/library`, `examples/ram-device` |
| Guide | [programs.md](programs.md) | [drivers-and-libraries.md](drivers-and-libraries.md) |

Both are built with `tools/build-package.sh <folder>` on Linux or WSL.

## Reading order

1. [getting-started.md](getting-started.md): install the toolchain, build
   and run a first program.
2. [concepts.md](concepts.md): libraries, tasks, signals, ports, memory
   types, tag lists, and the crate types for each.
3. [programs.md](programs.md) or
   [drivers-and-libraries.md](drivers-and-libraries.md), depending on what
   you are writing.
4. [features.md](features.md): pick the Cargo features you need.
5. The module guides for the areas you use (below).
6. [debugging.md](debugging.md) and [troubleshooting.md](troubleshooting.md)
   when something goes wrong.

## Index

### Overview and how-to guides

| File | About |
|---|---|
| [README.md](README.md) | This page: what the crate is, build modes, reading order, index. |
| [getting-started.md](getting-started.md) | Install the compiler and Docker image, build the program template, run it on an Amiga or QEMU. |
| [concepts.md](concepts.md) | AmigaOS ideas a Rust developer needs, each tied to the crate type that handles it. |
| [programs.md](programs.md) | Writing ordinary `std` programs: Cargo setup, cookies, arguments, a complete window example. |
| [drivers-and-libraries.md](drivers-and-libraries.md) | No-C-runtime packages: allocators, panic handler, C glue, libraries, devices, Expunge, Kickstart modules. |
| [features.md](features.md) | Every Cargo feature: what it enables, what it pulls in, where it can be used. |
| [debugging.md](debugging.md) | Debug output, the panic report and resource release, catch points, the `debug` module. |
| [troubleshooting.md](troubleshooting.md) | Known pitfalls with their causes and fixes. |

### Exec (always built)

| File | About |
|---|---|
| [exec/memory.md](exec/memory.md) | Allocators, `AmigaVec`, pools, named memory, DMA buffers and memory types. |
| [exec/tasks-and-sync.md](exec/tasks-and-sync.md) | Spawning tasks, signals, mutexes and semaphores, interrupt code, separate stacks, CPU information. |
| [exec/messages-and-ports.md](exec/messages-and-ports.md) | Message ports and typed messages, plus hooks, lists, AVL maps, tag lists, OS strings and `AmigaError`. |
| [exec/libraries.md](exec/libraries.md) | Opening libraries and interfaces, the raw `sys` bindings, `expansion` and the PowerPC `arch` helpers. |

### Devices

| File | About |
|---|---|
| [devices/using-devices.md](devices/using-devices.md) | Opening an exec device and doing I/O from a program; timer.device. |
| [devices/writing-devices.md](devices/writing-devices.md) | Writing an exec device: requests, unit tasks, quick I/O, AbortIO, SANA-II, virtio, PCI. |

### DOS and system libraries

| File | About |
|---|---|
| [dos.md](dos.md) | Files, locks, directories, processes, ReadArgs, notification, volumes and assigns. |
| [utility.md](utility.md) | utility.library: dates, tag lists, named objects, text, SHA-1 and random numbers. |
| [locale-prefs-application.md](locale-prefs-application.md) | Locales and catalogs, PrefsObjects, and application.library. |
| [workbench.md](workbench.md) | AppWindows, AppIcons, AppMenuItems, Workbench start-up and tool types. |
| [datatypes.md](datatypes.md) | Identifying files and reading pictures and other data through datatypes. |
| [console.md](console.md) | console.device: key conversion and console windows. |
| [elf.md](elf.md) | Reading ELF files and loading shared objects. |
| [input.md](input.md) | Joysticks, mice and keyboards through AmigaInput. |
| [networking.md](networking.md) | bsdsocket.library: TCP and UDP sockets, waiting on sockets and signals, name lookup. |

### GUI

| File | About |
|---|---|
| [gui/intuition.md](gui/intuition.md) | Screens, windows, events and requesters. |
| [gui/graphics-and-layers.md](gui/graphics-and-layers.md) | Bitmaps, drawing, fonts, regions and layers. |
| [gui/menus.md](gui/menus.md) | GadTools menus, menuclass menus, context menus and pop-up menus. |
| [gui/reaction.md](gui/reaction.md) | ReAction windows built from layouts and gadgets. |
| [gui/asl.md](gui/asl.md) | File, font and screen mode requesters. |

### Graphics hardware

| File | About |
|---|---|
| [vulkan.md](vulkan.md) | Vulkan through vulkan.library, VK_AMIGA_surface, and the `ash` binding. |

## Other places to look

- The repository's top-level `README.md` has a table of every module.
- `cargo doc --open` (on the host, with the features you use) gives the
  API reference generated from the source.
- `apps/`, `examples/` and `testprogs/` hold complete, working packages:
  `apps/imgview` (a picture viewer with a ReAction GUI), `apps/vkinfo` and
  `apps/vkplasma` (Vulkan), `examples/hello-driver`,
  `examples/hello-library`, `examples/ram-device`, and the test programs
  `testprogs/exectest` and `testprogs/libtest`, which exercise most of the
  safe API.
