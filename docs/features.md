# Cargo features

The crate is split into Cargo **features** so a package only compiles what
it uses. This matters most for drivers and Kickstart modules, which should
stay small.

## The rules

- **Features only add code.** A feature never switches the crate into a
  different mode, so turning a feature on anywhere in a dependency tree
  never breaks a driver.
- **exec is always built.** The exec bindings and the modules built on
  them (`allocator`, `mem`, `pool`, `named`, `task`, `signal`, `sync`,
  `msg`, `port`, `library`, `list`, `avl`, `hook`, `interrupt`, `stack`,
  `sysinfo`, `tag`, `cstr`, `io`, `error`, `fmt`, `panic`, `track`) need
  no feature.
- **One feature per SDK interface.** `dos`, `intuition`, `graphics` and so
  on each add the raw bindings for that library in `sys`, and the safe
  module where there is one.
- **The default set is `std`, `dos` and `utility`.** Programs keep the
  defaults and add to them. Code without a C runtime uses
  `default-features = false` and lists what it needs.
- The `arch` module (PowerPC cache, barrier and MMIO instructions) is not a
  feature: it is built whenever the target is `powerpc-unknown-amigaos`.

```toml
# A program: defaults (std, dos, utility) plus windows and drawing.
amigaos4 = { path = "../..", features = ["intuition", "graphics"] }

# A driver: no std, only what it uses.
amigaos4 = { path = "../..", default-features = false, features = ["device", "timer", "pci"] }
```

## Build modes

Every feature compiles in both build modes: `std` programs on clib4, and
`no_std` packages without a C runtime. The exceptions are about run time,
not compiling:

- `std` itself is for programs only (it needs clib4).
- `vulkan` / `ash`: the code builds anywhere, but the RADV driver only gives
  a GPU device to a `std` program (see [vulkan.md](vulkan.md)).
- Features whose modules call dos.library (`dos`, `asl`, `datatypes`,
  `elf`, `workbench`, ...) need the calling task to be a DOS process.
  Calls from a plain exec task return `AmigaError::WrongTask`.

## Mode and driver features

| Feature | Enables | Pulls in | Modes | Guide |
|---|---|---|---|---|
| `std` (default) | `std` trait impls (`std::error::Error` for `AmigaError`); `track::install_std_hook` | – | programs | [programs.md](programs.md) |
| `device` | `device` (driver building blocks: `IoReq`, `RequestQueue`, `Worker`, `answer_device_query`, command and error constants) and `devio` (using a device: `Device`, requests, `InFlight`) | – | both | [devices/writing-devices.md](devices/writing-devices.md), [devices/using-devices.md](devices/using-devices.md) |
| `sana2` | `sana2`: SANA-II Revision 7 structures, constants, `S2Req`, `BufferManagement`; `sana2ext`: the unofficial extensions ([devices/sana2ext.md](devices/sana2ext.md)) | `device`, `utility` | both | [devices/writing-devices.md](devices/writing-devices.md) |
| `virtio` | `virtio`: split rings, `Virtqueue`, legacy and modern PCI interfaces, `PciTransport`, `Notifier`, `virtio::net` | – | both (drivers) | [devices/writing-devices.md](devices/writing-devices.md) |
| `vulkan` | `vulkan`: `get_instance_proc_addr`, `instance_version`, VK_AMIGA_surface types; with `intuition`, `create_amiga_surface` on a window | – | both; a GPU needs `std` | [vulkan.md](vulkan.md) |
| `ash` | `vulkan::entry()` giving an `ash::Entry`, and `AmigaSurface` | `vulkan`, the `ash` crate 0.38 (the crate's only optional dependency) | both; a GPU needs `std` | [vulkan.md](vulkan.md) |
| `full-sdk` | every one of the 129 interface features below | all of them, and `device` through `console` and `timer`; not `std`, `sana2`, `virtio`, `vulkan` or `ash` | both | – |

## Interface features with safe modules

These add a safe Rust module on top of the raw bindings.

| Feature | Safe module and main types | Pulls in | Extra with other features | Guide |
|---|---|---|---|---|
| `dos` (default) | `dos`: `fs::File`, `fs::Lock`, `fs::ReadDir`, `process::spawn`, `args::Args`, `notify::Watcher`, `volume`, `misc`, variables, patterns | – | the panic report's `T:<name>.crash` file | [dos.md](dos.md) |
| `utility` (default) | `utility`: dates, `Sha1`, `Random`, `text`, `tags`, `names` | – | `NameList::sort_by` | [utility.md](utility.md) |
| `timer` | `timer`: `Waiter`, `AmigaTimer`, `TimerVal`, `micro_delay`, `get_up_time` | `device` | `msg::Port::recv_timeout` | [devices/using-devices.md](devices/using-devices.md) |
| `expansion` | `expansion::machine_info` | – | – | [devices/using-devices.md](devices/using-devices.md) |
| `pci` | `pci`: `Pci`, `Query`, `PciDevice`, `PciLock`, `IoRegion`, `MmioRegion` | – | – | [devices/writing-devices.md](devices/writing-devices.md) |
| `debug` | `debug`: `Debug::open`, `symbol`, `backtrace`, `watch_output` | – | symbol names in panic backtraces | [debugging.md](debugging.md) |
| `intuition` | `intuition`: `PubScreen`, `Window`, `Event`, `easy_request`, `Requester`; `menu` (menuclass menus) | – | `graphics`: `Window::rastport`; `gadtools`: `MenuStrip`; `layers`: `Window::clip`; `workbench`: `Window::set_app_window` | [gui/intuition.md](gui/intuition.md), [gui/menus.md](gui/menus.md) |
| `gadtools` | GadTools menus: `intuition::MenuStrip`, `MenuEntry`, `Window::set_menu` | – | needs `intuition` to be useful | [gui/menus.md](gui/menus.md) |
| `graphics` | `graphics`: `BitMap`, `RastPort`, `Font`, `Region`, `Rect` | – | `intuition`: window drawing | [gui/graphics-and-layers.md](gui/graphics-and-layers.md) |
| `layers` | `layers`: `LayerSet`, `Layer` | `graphics` | `intuition`: `Window::clip` | [gui/graphics-and-layers.md](gui/graphics-and-layers.md) |
| `reaction` | `reaction`: `Layout`, `Gadget`, `Window`, `WindowSpec`, events and values by gadget id | `intuition`, `listbrowser`, `workbench`, `speedbar` | `graphics` | [gui/reaction.md](gui/reaction.md) |
| `popupmenu` | `popup::PopupMenu` | `intuition` | – | [gui/menus.md](gui/menus.md) |
| `asl` | `asl`: `FileRequester`, `FontRequester`, `ScreenModeRequester` | `dos`, `intuition` | – | [gui/asl.md](gui/asl.md) |
| `workbench` | `workbench`: `AppPort`, `AppEvent`, `AppMenuItem`, `WbStartup`, `tool_types` | `dos` | `icon`: `AppIcon`, tool types; `intuition`: AppWindows | [workbench.md](workbench.md) |
| `icon` | raw icon.library bindings | – | with `workbench`: `AppIcon`, `WbStartup` tool types, `tool_types`/`set_tool_types` | [workbench.md](workbench.md) |
| `datatypes` | `datatypes`: `examine`, `list_all`, `DtObject`, `save_iff` | `dos` | `intuition`: `read_pixels`; `graphics`: `new_picture` | [datatypes.md](datatypes.md) |
| `console` | `console`: `raw_key_convert`, `Console` | `device`, `timer` | `intuition`: `WindowConsole` | [console.md](console.md) |
| `elf` | `elf`: `ElfFile`, `SharedObject` | `dos` | – | [elf.md](elf.md) |
| `amigainput` | the module named `input`: `Context`, `Device`, `State` (AmigaInput, polled) | – | – | [input.md](input.md) |
| `locale` | `locale`: `Locale` (texts, dates, `format`), `Catalog` | – | – | [locale-prefs-application.md](locale-prefs-application.md) |
| `application` | `application`: `Application`, `Event`, `Request` | – | – | [locale-prefs-application.md](locale-prefs-application.md) |
| `prefs_objects` | `prefs`: `PrefsDict`, `PrefsArray`, `Value` | – | – | [locale-prefs-application.md](locale-prefs-application.md) |
| `bsdsocket` | `bsdsocket` (`SocketLib`, Roadshow statistics and constants) and `net` (`Socket`, `TcpStream`, `TcpListener`, `UdpSocket`, `resolve`) | `utility` | – | [networking.md](networking.md) |

Note the two input features: **`amigainput`** gives the safe module
`input` (game controllers through AmigaInput). **`input`** is only the raw
bindings for input.device.

## Raw-only interface features

These add the raw bindings in `sys` for one SDK interface (vtable, types,
constants and `unsafe` wrapper functions), with no safe module yet. Each
pulls in nothing else. You call them through `sys` with the SDK autodocs
as your guide; [exec/libraries.md](exec/libraries.md) explains how.

| Area | Features |
|---|---|
| Audio and MIDI | `ahi`, `ahisub`, `camd`, `camd_driver` |
| BOOPSI gadgets and images (ReAction classes) | `bevel`, `bitmap_gc`, `button`, `checkbox`, `chooser`, `clicktab`, `colorwheel`, `datebrowser`, `diffview`, `drawlist`, `filler`, `fuelgauge`, `getcolor`, `getfile`, `getfont`, `getscreenmode`, `glyph`, `integer`, `label`, `layout`, `listbrowser`, `palette`, `penmap`, `radiobutton`, `requester`, `scroller`, `sketchboard`, `slider`, `space`, `speedbar`, `string_gc`, `texteditor`, `virtual_gc`, `window` |
| Graphics and fonts | `bullet`, `cybergfx`, `diskfont`, `p96`, `picture`, `warp3d` |
| Datatypes and documents | `amigaguide`, `dtclass`, `iffparse`, `textclip` |
| System and Workbench | `blankermodule`, `commodities`, `docky`, `iconmodule`, `keymap`, `language`, `screenblanker`, `version`, `misc`, `resource` |
| Scripting | `arexx`, `rexxsys` |
| Storage and file systems | `bootsd`, `diskio`, `filesysbox`, `hunk`, `mounter`, `mounter_private`, `partition`, `ramdrive` |
| Compression and data | `btree`, `bzip2`, `charset`, `expat`, `lzma`, `xadmaster`, `z` |
| Networking and security | `amissl`, `amisslmaster` |
| Time | `battclock`, `battmem`, `realtime`, `timesync`, `timezone` |
| Hardware and low level | `acpi`, `card`, `cia`, `cyrus`, `fsldma`, `i2c`, `i2c_resource`, `input`, `lowlevel`, `mmu`, `nv`, `openfirmware`, `pasdma`, `perfmon`, `potgo` |
| USB | `usbfd`, `usbhcd`, `usbresource`, `usbsys` |
| Catweasel | `catweasel`, `cw3_floppy`, `cw3_gameport`, `cw3_keyboard`, `cw3_sid` |
| Other | `xena` |

`listbrowser` and `speedbar` are turned on by `reaction`, which uses them
internally.

## Choosing features: examples

| You are writing | Features |
|---|---|
| A Shell tool that reads files | defaults only |
| A program with a window and drawing | `intuition`, `graphics` |
| A ReAction GUI with a file requester | `reaction`, `asl` |
| A picture viewer | `reaction`, `datatypes`, `graphics`, `asl`, `workbench`, `icon` |
| A TCP client | `bsdsocket` |
| A Vulkan program | `vulkan`, `ash`, `intuition` (a `std` program) |
| An exec device | `default-features = false`, `device` (plus `timer`, `pci` as needed) |
| A SANA-II network driver | `default-features = false`, `sana2`, `virtio` or `pci` |
| A shared library | `default-features = false`, plus only what its functions call |
| Panic backtraces with names | add `debug` |
