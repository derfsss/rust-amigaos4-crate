# amigaos4

A Rust crate for AmigaOS 4.1: raw SDK bindings, PowerPC helpers, an Exec
allocator and safe wrappers for device drivers, shared libraries and programs.

It is built for the `powerpc-unknown-amigaos` target of the AmigaLabs Rust
fork ([AmigaLabs/rust, branch `amigaos-port`](https://github.com/AmigaLabs/rust/tree/amigaos-port)).
That fork gives programs a full `std` on top of clib4. This crate adds what
the fork does not have: the AmigaOS interfaces, and a way to build code that
runs without a C runtime.

**Status: early (0.1).** The driver core is complete and in use (network,
block and entropy drivers and a TCP/IP stack are built on it); the
application layers (Intuition, ReAction, menus, ASL, datatypes, DOS, ...)
cover the common calls and grow as programs need them. APIs may still
change. It supersedes the earlier
[rust-for-amigaos4](https://github.com/derfsss/rust-for-amigaos4).

## What is in it

| Module | Contents |
|---|---|
| `sys` | Raw bindings for 130 SDK interfaces (bsdsocket's from the SDK's netinclude): vtables, types, constants, wrapper functions. Exec is always built; every other interface is a Cargo feature. |
| `arch` | PowerPC instructions: `dcbst`/`dcbf` cache maintenance, byte-reversed MMIO reads and writes, `sync`/`lwsync`/`eieio`/`mbar`. |
| `allocator` | `ExecAllocator`, a `GlobalAlloc` over `AllocVecTagList`/`FreeVec`. |
| `mem` | `AmigaVec`, `AmigaPool`, and `DmaBuffer`/`DmaMapping` for device DMA; `avail`, `type_of`, `copy` (CopyMem), `copy_quick` (CopyMemQuick on `[u32]`), `MemLock` (LockMem while it lives). |
| `pool` | `MemPool` (ASOT_MEMPOOL, protected; zeroed `PoolBlock`s freed on drop), `ItemPool<T>` (ASOT_ITEMPOOL, protected; `PoolItem` owns a `T`; item limit), `ResourceMap` (ASOT_RMAP; `ResourceRange` returned on drop, aligned ranges). |
| `named` | Named memory: `alloc`/`alloc_with` (checksum), `exists`, `lock` (a `NamedLock` derefs to the block, `free`, `update_checksum`). Blocks carry an 8-byte size header; no `try_lock` (exec's AttemptNamedMemory spins under contention). |
| `hook` | `Hook<F>`: a utility Hook (ASOT_HOOK) calling a `Send + Sync` closure with the object and message pointers. |
| `list` | `NameList`: an exec list owning named nodes (ASOT_LIST/ASOT_NODE): push at either end, `enqueue` by priority, `find`/`find_ignore_case`, `remove`, pops, `append` (MoveList), iteration; `as_ptr` for OS calls. |
| `avl` | `AvlMap<V>`: an ordered `u32`-keyed map on exec's AVL trees: insert (replaces), get, get_mut, remove, iteration in order, `range_from`. |
| `library` | `Library` (OpenLibrary; version, revision, id string), `Interface<I>` (GetInterface, dropped before the library closes), `open_resource`. |
| `interrupt` (more) | `SoftInt`: a software interrupt run by `cause` (before Cause returns, from a task). |
| `sysinfo` | `cpu()`: everything GetCPUInfo reports (name, speeds, timebase, caches, line size, pages, vector unit, current core). |
| `stack` | `run_with_stack(size, f)`: a closure on a stack of its own (NewStackRun). |
| `debug` | Feature `debug`: `Debug::open` (exec's debug interface, no IDebug global), `symbol(addr)` (module, segment, offset, source), `backtrace(max)`, `watch_output(&signal)` (StartDebugOutputNotify: the signal is set whenever the debug output is written; ended on drop). |
| `port`, `tag`, `io`, `cstr`, `error` | Message ports (`AmigaMsgPort`: AllocSysObject, private or public by name), tag lists, `Read`/`Write`, OS strings (`amstr!`), errors. |
| `msg` | Typed messages between tasks: `Port<T>` (the owner's), `Sender<T>` (any task; a send to a dropped port comes back), `Message<T>` (MEMF_SHARED, optional reply port), `Received<T>` (reply, or replied on drop; a dropped port replies to what it still holds). |
| `devio` | Using a device (feature `device`): `Device::open`/close, `base()`, the device's request and duplicates (`new_request`, or `new_request_on` a port of its own), `do_io`, `do_io_data`, `send`/`send_data` returning an `InFlight` that borrows the request and buffer (wait, abort; aborted and waited for on drop), and for requests without a buffer `start`/`check`/`finish`/`cancel_pending` (anything touching a started request cancels it first). |
| `timer` | timer.device (feature `timer`, on `devio`): `Waiter` (wait for signals with a time limit, ended by Ctrl-C with `Err(Cancelled)`: `Waited::Signals` or `TimedOut`; `msg::Port::recv_timeout` uses it); `AmigaTimer` (delay on a request and reply port of its own, so a started request is left running; start/is_done/finish/abort; `delay_ticks` for the EClock unit, a `TimerVal` converted there; `up_time`, `eclock`, `get_sys_time` through the unit's ITimer); `TimerVal` normalised, ordered, checked add/sub; free `get_up_time`, `sys_time`, `eclock`, `micro_delay` through the program's ITimer or one the crate opens. |
| `track` | What a panic leaves behind. Files, locks, signal bits, message ports, public screen locks, windows, ReAction windows, AppWindows, AppIcons, Tools menu items, libraries, interfaces, devices, interrupt servers, reset callbacks, low-memory handlers, DOS and variable notifications, seglists, shared objects, bitmaps, fonts, datatypes objects, locales, catalogs and bsdsocket bases register with the task that made them (and the caller's source line); a panic (through `panic::default_panic_handler`) prints the message and a backtrace from the panic site to the serial output and `T:<command>.crash`, releases what the task still holds (newest first, raw release calls, no `Drop`), and returns to the task's catch point: `track::main(f)` for a program (RETURN_FAIL to the Shell), the crate's spawned tasks and processes (their join/wait give an error). `catch(f)` makes a catch point (PowerPC register save in `global_asm!`). At the end of `track::main`, what is still registered is reported as leaked and released. A task with no catch point, or a program whose children still run, sleeps (Wait(0)) instead of spinning (with children running, nothing is released). A child's leftovers pass to its parent when it ends (its own signal bits and ports are only forgotten). std (clib4) programs call `track::install_std_hook()`: the same report, and with `panic = "abort"` the release, before std's abort. |
| `task` | Exec tasks: `spawn`/`Builder` with a start-up handshake (the child waits for CTRL-F until CreateTask has returned), `JoinHandle` (join, or join on drop; the control block is MEMF_SHARED; the child's last act is a tail call into exec's Signal, so the program may end right after a join; a child that panics gives `join` an error), the calling task's priority (`priority`, `set_priority`) and CPU affinity, `can_block` (GetSystemInfo), `cpu_count`. No DOS in a child: exec tasks may not call it. |
| `signal` | `Signal` (a signal bit of the calling task, freed on drop, not `Send`), `SignalTarget` (signal another task; `unsafe`), `wait`, `take`, `peek`, `current_task`; `Cancel` (Ctrl-C as a stop request: `check()?` gives `Err(Cancelled)`, `MASK` for waits). |
| `sync` | `Mutex<T>` (exec mutex, created not recursive, its owner tracked: a second lock by the holder is `WouldDeadlock`, since exec's MutexAttemptWithSignal grants it again; release only by the guard), `Semaphore<T>` (SignalSemaphore as a reader-writer lock; the exclusive owner's second lock is refused with `WouldDeadlock`), each with try and lock-or-signal forms; a `Semaphore` can also be bid for without blocking (`bid_write`/`bid_read`: Procure, the bid's port signal joins any Wait, `take`/`wait` give the usual guard, dropping an untaken bid Vacates it). No Forbid. |
| `interrupt` | `IntServer` (AddIntServer), `ResetCallback` (AddResetCallback) and `MemHandler` (AddMemHandler: called when an allocation fails, under Forbid, with the request's size and flags) nodes. |
| `device` | Exec device driver building blocks (feature `device`): `IoReq` (an `IOStdReq`'s fields by name, `finish` honouring `IOF_QUICK`, `reply`, `mark_pending`), `RequestQueue` (a unit's queue and current request under an exec mutex, with AbortIO's cases and CMD_FLUSH), `Worker` (the unit task's work signal, published when ready, and a stop request), `answer_device_query`, and the `cmd`/`ioerr` constants. |
| `virtio` | virtio over PCI (feature `virtio`): split rings, `Virtqueue`, the legacy and modern interfaces, `PciTransport` that picks one, `Notifier` (a queue's notify register for another task), and `virtio::net` (virtio-net feature bits, config offsets, header and control-queue helpers). |
| `sana2` | SANA-II Revision 7 (feature `sana2`): `IOSana2Req` and the query, statistics and hook structures with their sizes asserted, the constants (112 checked against the SDK by `tests/sana2_consts.rs`), `S2Req` accessors, and `BufferManagement` (the stack's copy hook or its OpenDevice copy functions, 68k ones through Emulate). |
| `intuition` | Windows (feature `intuition`): `PubScreen` (LockPubScreen, size, title, beep; unlocked on drop), `Window` on it (OpenWindowTagList with owned title strings; bounds, borders, inner box, set_title, activate, to_front, change_box, set_idcmp, set_busy (busy pointer), set_menu_checked; `next`/`wait` copy each IntuiMessage into an `Event` and reply it; `rastport()` with feature `graphics`: the window's RastPort borrowed, its font put back afterwards; CloseWindow on drop), `easy_request` (blocking; texts shown literally, the button's index from the left) and `Requester` (non-blocking, polled, freed on drop). With feature `gadtools`: `MenuStrip` built from `MenuEntry`s (titles, items with Amiga-key shortcuts, toggles, sub-items, bars), attached with `Window::set_menu`, choices arriving as `Event::Menu` with the entry's id. `Window::refresh()` is a BeginRefresh/EndRefresh guard for simple-refresh windows. Gadgets, own screens and Workbench start-up are not wrapped yet. |
| `layers` | Layers on a bitmap of the program's own (feature `layers`, pulls in `graphics`): `LayerSet` over a borrowed `BitMap`, `Layer`s created in front or behind (smart refresh), moved, sized and restacked inside the bitmap, `is_top_at` (WhichLayer), drawn into through their own RastPort; deleted on drop. With `intuition`: `Window::clip(&Region)` installs a clip region until the guard drops. |
| `workbench` | Workbench (feature `workbench`, pulls in `dos`): `AppPort` copies each AppMessage into an `AppEvent` (kind, id, class, drop point, files as paths) and replies it; `Window::set_app_window` (with `intuition`; the window keeps its AppWindow and removes it before it closes), `AppIcon` (with `icon`, the default tool icon) and `AppMenuItem` borrow the port and are removed on drop. `WbStartup` takes the start-up message of a program started from Workbench (the tool and project icons as paths, the tool types with `icon`) and replies it on drop, without Forbid (DOS frees the code after the process ends); `tool_types` / `set_tool_types` read and write an icon's tool types. |
| `vulkan` | Vulkan through an AmigaOS 4 `vulkan.library` (the Khronos loader as a library, with a driver such as Mesa's RADV): `get_instance_proc_addr` takes the loader's plain-C `vkGetInstanceProcAddr` from the library's interface (the crate opens it), the entry point for any Rust Vulkan binding; `instance_version`; VK_AMIGA_surface (`AmigaSurfaceCreateInfo`, with `intuition` `create_amiga_surface` on a `Window`). Feature `ash` (the crate's one optional dependency, ash 0.38 with no_std) adds `entry()` giving an `ash::Entry` and `AmigaSurface` for an `ash::Instance`. Needs `vulkan.library` at run time, and for a card its driver library and `VK_DRIVER_FILES`. A std (clib4) program is needed for a device: RADV refuses a process with no C library context. `apps/vkinfo` (std) lists the devices (a Radeon RX 590 on an AmigaOne X5000) and makes a surface; `apps/vkplasma` renders and presents through a swapchain (FIFO 60 fps, MAILBOX 1800 fps on the RX 590, `p` switches modes). |
| `menu` | BOOPSI menus (menuclass, Intuition V54; feature `intuition`): a `Strip` of `Menu`s of `Item`s (actions with keys, toggles, radio groups, separators, submenus at any depth, disabled items) built into a `MenuTree` that keeps its texts; state by id (`checked`, `set_checked`, `disabled`, `set_disabled`; MM_GETSTATE/MM_SETSTATE) and the picked ids (MM_NEXTSELECT). `intuition::Window::set_menu_strip` and `set_context_menu` (the right button over the window's inside, through WA_ContextMenuHook) own their trees and report picks as `Event::Menu`; ReAction windows take `WindowSpec::menu` and `Gadget::context_menu`. |
| `popup` | Pop-up menus (popupmenu.class; feature `popupmenu`): a `PopupMenu` from the same `Item`s, `open(window)` returns the picked id (PM_OPEN). |
| `reaction` | ReAction windows (feature `reaction`, pulls in `intuition` and `listbrowser`): a window described as a tree of `Layout`s (rows, columns, framed groups) and `Gadget`s (button, push button, string, check box, integer, slider, chooser, list, gauge; a label, weight and minimum size each), opened with `Window::open`, which makes the BOOPSI objects (window.class, layout.gadget, the gadget classes, label images) and keeps every text, label array and list node list they point at. `next` returns `Event`s (a gadget released with its id and code, close, keys, resize); values are read and set by gadget id (`text`, `checked`, `number`, `level`, `active`, `selected`, `set_items`, `set_disabled`). Dropping the window disposes the objects, then frees the lists. |
| `asl` | asl.library's requesters (feature `asl`, pulls in `dos` and `intuition`): `FileRequester` (open, save, several files, drawers only; a pattern; the drawer, file and joined paths copied out), `FontRequester` (name, size, style) and `ScreenModeRequester` (mode id, size, depth), each kept between requests so it remembers where the user was; a parent window is put to sleep while one is shown; Cancel is `Ok(None)`. |
| `datatypes` | What a file is and its contents, without a window (feature `datatypes`, pulls in `dos`): `examine` (a path or bytes), `list_all`, `dt_string`; `DtObject` from a file or borrowed bytes (pictures kept in their own depth, not remapped), copied attributes (`get_u32`, `data_type`, `bitmap_header`, `methods`, `text`), and with `intuition` `read_pixels` (RGB/RGBA/ARGB through PDTM_READPIXELARRAY). `save_iff` (DTM_WRITE as IFF; tested for pictures, which come out as ILBM); with `graphics`, `new_picture` makes a 24-bit picture from RGB/RGBA/ARGB pixels (handed to picture.datatype as a bitmap) to save. Showing objects in windows, printing, a datatype's own format and tools are not wrapped. |
| `console` | console.device (feature `console`, pulls in `device` and `timer`): `raw_key_convert` (RawKeyConvert through the default keymap, the device opened as a library once); `Console` in a window console.device creates and closes itself: `write` (text and CSI sequences), `read` with a time limit (an unanswered read is aborted), `clear_input`, `size` and `cursor` (CD_GETATTRS). `WindowConsole` (with `intuition`): a console on a window of the program's own, borrowed while it lives. Keymaps, palettes and snips are not wrapped. |
| `elf` | Reading ELF files (feature `elf`, pulls in `dos`): `ElfFile` from a path or borrowed bytes, closed on drop; the header, sections with names and their bytes, symbols by name, by loaded address, or all of them, each with its value in the file and its address in elf.library's loaded copy. `SharedObject` (DLOpen, symbol addresses by DLSym, DLClose on drop; `open` is unsafe: the object must match the program's C runtime). ElfLoadSeg is not wrapped (`dos::process::SegList` loads and runs programs). |
| `amigainput` | AmigaInput, polled (module `input`): `Context` (API 1, no port or window; deleted on drop), `devices()` copied, typed `query`, `obtain` a `Device` (offsets queried once; released on drop) and `read` its axes, buttons and hats into a `State`; errors carry AIN_GetError's code. Event mode and the device requester are not wrapped. Obtaining and reading are untested: the QEMU image has no AmigaInput drivers. |
| `graphics` | Off-screen graphics (feature `graphics`, graphics.library 54): `BitMap` (AllocBitMapTagList with a pixel format, cleared, user-private; size, format, RTG; `lock` = LockBitMap memory; `blit_from`/`blit_within` = BltBitMap; FreeBitMap on drop), `RastPort` of the crate's own on a borrowed bitmap (pens, draw mode, SetRast/SetRastColor, RectFill/RectFillColor, lines, ellipses, pixels and ARGB pixels, Read/WritePixelArray in five formats, text with a borrowed `Font`), `Font` (OpenFont, metrics), `Region` (rectangle and region operations, bounds, rectangles, area). A RastPort without layers does not clip, so every call checks its coordinates and buffer sizes first. Windows are in `intuition`, layers in `layers`. |
| `application` | application.library's "application" interface v2 (feature `application`): `Application::register` (URL identifier, unique, no icon, hidden, description; unregistered on drop after replying what is queued), `id`, `identifier`, `signal_mask`, `next`/`wait` (each message copied into an `Event` and replied), `send` a `Request` (Quit, Hide, OpenDoc, Custom...) to one application or all; `find`, `find_identifier`, `list`. |
| `prefs` | PrefsObjects v2 (feature `prefs_objects`): `PrefsDict` and `PrefsArray` (released on drop; values moved in as `Value`: string, int, bool, array, dictionary), typed reads, an `ArrayRef` borrowed from its dictionary, `remove`, `len`, `write`/`read` as XML to a path (ReadPrefs clears the dictionary first). |
| `locale` | locale.library (feature `locale`; the crate opens it when `ILocale` is NULL): `Locale` (current or from a prefs file; language, code set, country, GMT offset; GetLocaleStr texts; Is* tests; ConvToUpper/Lower; StrnCmp and StrConvert sort keys with ASCII/Collate1/Collate2; FormatDate and ParseDate, the template's codes checked), `Catalog` (OpenCatalogA; a missing catalog answers with the built-in strings). Texts are bytes in the locale's code set, with `_lossy` helpers. `format` (FormatString32: positions, widths, `%D`/`%U` grouped by the locale; every command checked against the `FmtArg`s first). |
| `net` | Owned sockets over `SocketLib` (feature `bsdsocket`): `Socket` (CloseSocket on drop; peer/local address, non-blocking, bytes available, typed options: reuse, keepalive, broadcast, nodelay, read/write timeouts as `Duration`, linger, take_error, shutdown), `TcpStream`/`TcpListener`/`UdpSocket` (`tcp_connect`, `tcp_listen`, `udp_bind`, send_to/recv_from), `SocketLib::wait` (WaitSelect: sockets and exec signals together, a timeout), `resolve` (gethostbyname, copied), `hostname`, `set_break_mask`; `would_block`, `interrupted`. Not `Send`: a socket belongs to the task that opened the library. |
| `bsdsocket` | bsdsocket.library, Roadshow (feature `bsdsocket`): `SocketLib` (open the library; IPv4 sockets; `QueryInterfaceTagList`, `ObtainInterfaceList`, `GetNetworkStatistics`, `SocketBaseTagList`), `SockAddrIn`, `SANA2CopyStats`, `ProtocolConnectionData`, the `tcpstat`/`ipstat`/`udpstat`/`mbstat` field tables, and the constants (89 checked against the SDK by `tests/bsdsocket_consts.rs`, with the field tables). |
| `utility` | utility.library, safe (feature `utility`; the crate opens the interface itself when `IUtility` is NULL): `Utility` (dates: `amiga_to_date`, `date_to_amiga` that tells the epoch from an illegal date; `unique_id`; the library version), `Sha1`, `Random`; `text` (Stricmp, Strnicmp, ToLower/ToUpper with locale rules; UTF-8 compare, UCS-4 case mapping, `ucs4_valid`, `utf8_decode_lossy`); `tags` (`TagList`/`TagListMut` checked like NextTagItem walks them, TAG_MORE refused: find, get, iterate, filter, map, apply, filter_changes, PackBoolTags, TagInArray; `TagClone` with refresh); `names` (`Named`, `Published` in the root or a nested namespace, `Found` holding a use, `find`, `find_all`). `NameList::sort_by` is SortList. Skip lists, splay trees, string copies and printf-likes are left to Rust. |
| `dos` | dos.library, safe (feature `dos`; calls that need a DOS Process return `WrongTask` in an exec task): `fs::File` (Open, Read, Write, seek, size, truncate, Close on drop; `close` returns Close's result), `fs::Lock` (UnLock on drop; DupLock, ParentDir, NameFromLock, SameLock, OpenFromLock), `fs::Metadata` (ExamineObject, copied out), `fs::ReadDir` (ObtainDirContext/ExamineDir with every field, optionally a pattern), create_dir, delete, rename, protection, comment, date, exists; `fault`, `DateStamp` (now, seconds, ordered), local and global variables, `Pattern`, AddPart/FilePart/PathPart, the Process's streams (`Stream`: `core::fmt::Write` and `io::Read`/`Write`); `process` (a closure in a child Process: `spawn`/`ProcessBuilder` with priority, stack, input, output, current directory; CTRL-F start-up handshake, NP_Child; `Child::wait` = WaitForChildExit then the result; `ProcessBuilder::spawn_detached` for a server process that outlives its starter (a library's or device's: wait until FindTask(name) is NULL before unloading); `pid`, `parent_pid`); `args` (ReadArgs by template from a string or the program's command line, read back by name at each option's type; FreeArgs on drop); `notify::Watcher` (StartNotify, signal method; EndNotify on drop), `notify::VarWatcher` (NotifyVar: a signal when a global variable is created, changed or deleted); `volume` (names of volumes, devices, assigns from a DOS_VOLUMELIST snapshot; `disk_info` = GetDiskInfo; assigns made and removed: `assign_lock`, `assign_path`, `assign_late`, `assign_add` (multi-assign, at a position), `remove_assign_entry`, `remove_assign`; a lock DOS keeps is handed back on failure); `process::processes()` (ProcessScan: pid, parent, name, priority, copied out); `process::SegList` (LoadSeg, `run` = RunCommand in the calling Process, UnLoadSeg on drop); `misc` (`system` = synchronous System, `CurrentDir` and `NoRequesters` guards that restore on drop, `DateStamp::to_text`/`from_text` = DateToStr/StrToDate). Left out on purpose: buffered FOpen I/O (use `File` with Rust buffering), MatchFirst/MatchNext (`read_dir_matching` and `Pattern` cover it), asynchronous System (it closes the streams it is given), CLI internals, packets. |
| `pci` | PCI devices, safe (feature `pci`; the crate opens expansion.library's `pci` interface when `IPCI` is NULL): `Pci::find`/`devices` with a `Query` (vendor, device, class under a mask, or a bus:dev.fn address); `PciDevice` (FreeDevice on drop): IDs, class, status, checked configuration reads, `bars()` copied out, the capability list, `interrupt()` (MapInterrupt); `lock` (never waits: `InUse`) gives a `PciLock` (Unlock on drop) with I/O and memory decoding, a bounds-checked `IoRegion` over an I/O BAR (FIFO block forms too) and, `unsafe`, configuration writes, bus mastering and an `MmioRegion` (a memory BAR the CPU may not reach behind every bridge). |
| `expansion` | `machine_info()` (GetMachineInfo, feature `expansion`): machine type and name, memory, total physical memory (64-bit), extension words. |
| `fmt`, `panic` | `serial_println!` to the debug output, and a panic handler that uses it. |

The C glue in `glue/` is compiled into each driver: `amiga_glue.c` for
varargs calls, and `freestanding_mem.c` for `memcpy`, `memmove`, `memset`,
`memcmp`, `bcmp` and `strlen` (the copies and fills move aligned words;
`tests/host/memtest.c` checks them on the host).

## Device DMA

`DmaBuffer` follows the exec autodocs: `StartDMA`, then `GetDMAList` for the
physical blocks, then `EndDMA`. The device is given physical addresses, never
the CPU's virtual address, and the cache work is done by exec. This is
correct on cache non-coherent machines such as the Sam460.

```rust
let mut buf = DmaBuffer::alloc(4096)?;
buf.as_mut_slice()[..len].copy_from_slice(data);
{
    let map = buf.map(DmaDirection::ToDevice)?;   // StartDMA + GetDMAList
    for e in map.entries() {
        // one descriptor per block: e.PhysicalAddress, e.BlockLength
    }
    // ... run the transfer and wait for it ...
}                                                  // EndDMA
```

The mapping borrows the buffer, so the CPU cannot touch it while the device
owns it. `DmaMapping::new` maps memory you did not allocate, such as a
caller's `io_Data`.

## Repository layout

| Folder | Holds |
|---|---|
| `src/` | the crate (`src/sys/` the generated bindings) |
| `glue/` | C linked into every package without a C runtime |
| `apps/` | programs that do something: `imgview` (a picture viewer), `vkinfo` (Vulkan devices and surfaces), `vkplasma` (a Vulkan swapchain demo) |
| `testprogs/` | test programs and probes run on the Amiga: `test-driver`, `exectest`, `libtest`, `kick-test`, `expunge`, `panictest`, `panicstd` |
| `examples/` | minimal samples to copy: `hello-driver`, `hello-library`, `ram-device` |
| `templates/` | starting points for a program, a driver and a library |
| `tests/` | host tests: constants, the SDK audit, C layout checks (`tests/layout`) |
| `third_party/vulkan/` | the Vulkan headers the layout checks need (Khronos, Apache-2.0; the AmigaOS ones MIT) |
| `tools/` | toolchain and package builds, the layout gate, the cargo linker wrapper |

Each package under `apps/`, `testprogs/` and `examples/` is its own cargo
project with the crate as a path dependency (`../..`); build one with
`tools/build-package.sh <folder>`.

## Using the crate

### Drivers, libraries and resources

These have no C runtime. Turn default features off and pick the interfaces
you need:

```toml
[lib]
crate-type = ["staticlib"]

[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", default-features = false, features = ["dos", "expansion"] }
```

```rust
#![no_std]
#![no_main]

#[global_allocator]
static ALLOCATOR: amigaos4::allocator::ExecAllocator = amigaos4::allocator::ExecAllocator;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}
```

rustc does not link a static library. The package's Makefile links it with
`ppc-amigaos-gcc -nostartfiles -nodefaultlibs`, the C glue and `-lgcc`, so
clib4 never enters the binary. Start from a template:

| Template | Makes | Reference example |
|---|---|---|
| `templates/program` | a program: `fn main`, std on clib4, linked by cargo (see Programs) | `apps/vkplasma` |
| `templates/driver` | a handler or driver with its own `_start` | `examples/hello-driver` |
| `templates/library` | a `.library` with a Resident and interface vectors | `examples/hello-library` |
| (copy the example) | an exec `.device` with BeginIO/AbortIO and quick I/O | `examples/ram-device` |

A library or device loaded from disk must unload cleanly. Close, when it
is the last one and an Expunge was deferred (`LIBF_DELEXP`), calls the
Expunge itself, which removes the node from exec's list before handing
back the segment list; returning the segment list from Close alone leaves
exec's list pointing into freed code. `testprogs/expunge` tests it on the
machine: `expunge DEVICE <name> [unit]` or `expunge LIBRARY <name>` opens
it, calls RemDevice or RemLibrary (which defers, as it is open) and closes
it, after which it must be gone from exec's list and load afresh when
opened again. On QEMU's AmigaOne, ram.device and hello.library pass twice
each.

### Programs

A program is an ordinary Rust binary: `fn main`, the default `std`
feature, the target's `std` on clib4 (`println!`, `std::env::args`,
`std::fs`, threads). Start from `templates/program`:

```
tools/build-package.sh templates/program     # under WSL
# -> templates/program/target/powerpc-unknown-amigaos/release/my-program
```

- **Linking.** rustc links it with the target's linker, `ppc-amigaos-gcc
  -mcrt=clib4`. The program's `.cargo/config.toml` names
  `tools/ppc-amigaos-gcc`, which runs the installed gcc, or else the one in
  the cross-compiler image (the paths rustc passes are mounted at the same
  places). No Makefile and no C glue.
- **Interface globals.** The crate gives every interface global it reads
  (`IIntuition`, `IGraphics`, ... 127 of them) a weak NULL definition
  (`src/sys/weak_globals.rs`, from `tools/gen-weak-globals.py`), so a
  program declares none: clib4's own (`IExec`, `IDOS`, `IUtility`, `IElf`)
  replace theirs, and a NULL one makes the safe module open the library
  itself.
- **Stripping.** `strip = "debuginfo"` in the release profile. `true` (or
  `"symbols"`) passes `--strip-all` to the linker, and AmigaOS refuses
  the file (IoErr 305).
- **Arguments.** `std::env::args()` works; `dos::args::Args` reads the
  command line with ReadArgs and an AmigaDOS template.
- **clib4.library 2.5** (23.10.2026) or newer on the Amiga. With 2.1 and
  2.4 (the one in the cross-compiler image's SDK) a program built here
  crashes in clib4's start-up or exit code (`clib4_start` ->
  `CloseLibraryInterface` -> `__pthread_exit`, a DSI); with 2.5 it runs.
- **Cookies.** `#[used] static STACK: [u8; N] = *b"$STACK:262144\0";` sets
  the stack the Shell gives it, and a `$VER:` string the same way.

`apps/vkinfo` and `apps/vkplasma` are programs built this way.

### Features

Features only add code. A feature turned on anywhere in a dependency tree
never breaks a driver.

- `std` (default): `std` trait impls such as `std::error::Error`.
- one feature per interface: `dos`, `utility` (both default), `timer`,
  `expansion`, `pci`, `mmu`, `openfirmware`, `fsldma`, `i2c`, `usbsys`, ...
- `full-sdk`: all 129 optional interfaces.
- `device`, `virtio`: the device-driver building blocks and virtio support.
- `sana2`: SANA-II network driver support (needs `device` and `utility`).
- `bsdsocket`: bsdsocket.library for programs (needs `utility`).

## Building

Target builds need Linux. WSL works.

1. **Build the AmigaLabs compiler**:

   ```bash
   tools/build-toolchain.sh          # tree in ~/amigaos-rust
   ```

   This builds stage1 `rustc` and the `powerpc-unknown-amigaos` sysroot, and
   links them as the rustup toolchain `amigaos`.

2. **Get the cross-compiler image** into the Docker that Linux sees. Under
   WSL with its own Docker engine, Docker Desktop's images are not visible:

   ```bash
   docker pull walkero/amigagccondocker:os4-gcc15-3.6.1   # gcc 15.3.0, SDK 54.25
   ```

3. **Build a package**:

   ```bash
   tools/build-package.sh examples/ram-device
   ```

The packages set `target = "powerpc-unknown-amigaos"` in `.cargo/config.toml`.
The `amigaos` toolchain has no cargo of its own, so rustup uses the default
toolchain's cargo with the AmigaLabs `rustc`.

### Target tests

`testprogs/test-driver` is a no-C-runtime program, built like a driver and
started from the Shell. It tests the allocator, memory types, the memory
functions, tag lists, message ports, DMA mapping, timer.device and the cache
instructions. It then opens the example `ram.device` and checks a write and
read round trip. Results go to the debug output (serial):

```bash
tools/build-package.sh examples/ram-device
tools/build-package.sh testprogs/test-driver
```

Copy `test-driver` and `ram.device` to `RAM:rusttest/` on the Amiga, run
`RAM:rusttest/test-driver`, and look for `test-driver: 14/14 tests passed` on
serial. The return code is 0 when every test passes, 20 otherwise.

`testprogs/exectest` tests `task`, `signal` and `sync` from the Shell
(`exectest [rounds]`, output to the Shell, return code 0 when all pass):
spawn and join at three priorities relative to the caller, 8 children
alive at once, join on drop; a mutex under 8 children with the hand-offs
counted, refused re-entry, lock-or-signal against another holder; a
semaphore whose readers are ticked half-way through a write and must be
refused first, then see the data whole; signal bits and free memory
unchanged at the end; typed messages and request/reply between tasks;
device I/O on timer.device and the example ram.device; memory pools, item
pools shared by children, resource maps, copies, named memory, hooks;
lists, AVL maps, libraries, soft interrupts, CPU info, a separate stack,
debug symbols and a backtrace. Copy `exectest`
and `examples/ram-device/ram.device` to `RAM:` on the Amiga (exectest opens
`RAM:ram.device`) and run `RAM:exectest` from a Shell.

`testprogs/libtest` tests the safe layers over other libraries
(`libtest utility`, `libtest pci`, `libtest timer`, `libtest net`, `libtest locale`, `libtest app`, `libtest gfx`, `libtest win`, `libtest layers`, `libtest wb`, `libtest dt`, `libtest con`, `libtest elf`, `libtest ain` (and `libtest gui`, `libtest wbgui`, `libtest congui` and `libtest aingui`, which
wait for input injected from the host), `libtest dos` (with
`process` and `args`), `libtest all`; `libtest probe` repeats the raw named-object probes
behind `utility::names`, leaking its objects on purpose). Its glue leaves
`IUtility` NULL, so the crate's own interface opening is what runs. The
utility checks: dates against a Rust date computation, SHA-1 test
vectors, Random, unique IDs from 4 tasks, string compares, UTF-8 and
UCS-4, tag lists, named objects (a drop that waits for another task's
holder), SortList. The pci checks: GetMachineInfo; every device listed
with its BARs and capabilities, found again by IDs and by address; class
queries; the library's capability list against a walk from 0x34; BAR
kinds against the BAR registers; devices drivers hold refused by Lock,
another task refused while this one holds a lock, shared locks; an I/O
window's bounds and a virtio device's features through it. The dos
checks (in `RAM:libtest.d`, removed afterwards): WrongTask from an exec
task; files written, read, sought, truncated, appended, examined; locks
compared, duplicated, walked to the root, refused while another holds
them; directories listed (with a pattern, through a lock); protection,
comment, date, rename, a non-empty directory refused; memory around 100
file cycles; local and global variables (3000 bytes with newlines);
patterns; AddPart, FilePart, PathPart; dates. Processes: a child's
result, PID and parent; 200 children 5 above the parent that end at once
(the CreateNewProc race); 8 at once writing files through DOS; a given
output file and current directory; a drop that waits. ReadArgs: every
modifier, absent options, the four DOS errors, the program's own
command line. Timer: ITimer opened by the crate, delays measured with
the E-clock, a delay while a started request is pending (its signal
still arrives), abort, EClock and WaitUntil units, a drop with a request
pending, memory around 200 cycles. Net: resolve, set_opt refusing the
timeval options, WaitSelect timing out, a UDP loopback datagram,
EWOULDBLOCK, SO_RCVTIMEO, WaitSelect returning for a signal, 20000
bytes echoed over TCP to a child process, sockets closed on drop.
Locale: the current locale, its texts, Is* and case, StrnCmp and sort
keys, FormatDate and ParseDate (30 February refused), catalogs that are
missing or not needed. Application: register, find, list, messages to
ourselves (the library copies them), a unique application's second
instance, unregistering on drop, new IDs; PrefsObjects: a dictionary of
every kind written as XML and read back. Graphics: ARGB and palette
bitmaps, colour and pen fills read back to their edges, every
out-of-bounds call refused, locked memory, pixel arrays and conversion,
blits, topaz text inside its box, region algebra. Windows: the default public screen, a window's geometry,
deferred activation and resizing seen as events, drawing into it, a
second window taking the activation, a non-blocking requester, memory
around 100 windows; `gui` checks keys, a click and the close gadget
typed or clicked by the person running it, Amiga-key menu shortcuts (a toggle checked then unchecked), and an EasyRequest button. More DOS: volume, device and assign names, GetDiskInfo of SYS:,
directory and file notification, System into a file and its return
codes, CurrentDir, NoRequesters, DateToStr/StrToDate round trips. Copy
`libtest` to the Amiga and run `libtest all` from a Shell; the interactive
modes (`gui`, `wbgui`, `congui`, `aingui`, ...) wait for the keys and clicks
their functions in `testprogs/libtest/src/main.rs` describe.
Lines are also sent to the serial debug output. `libtest notifyprobe`
measures StartNotify/EndNotify memory. exectest, libtest and test-driver
all pass on QEMU's AmigaOne.

`tools/check-layout.sh` compiles every `tests/layout/*.c` (static
asserts on the structure layouts the safe modules use) against the
image's SDK, and `tests/layout/vulkan.c` against the Vulkan headers in
`third_party/vulkan/include` (the AmigaOS SDK has none; `VULKAN_SDK_INCLUDE`
points it at another copy).

### Kickstart module test

`testprogs/kick-test` builds `rusttest.device`, a kickstart module with two
Residents in one file:

- a COLDSTART auto-init device (`RTF_NATIVE|RTF_COLDSTART|RTF_AUTOINIT`,
  pri 0). Exec creates it during the cold start; its init prints the
  Resident, device base, task, exec version, free memory, CPU and whether
  dos.library's Resident is present, and checks the Rust heap;
- an AFTERDOS tag (`RTF_NATIVE|RTF_AFTERDOS`, pri -100). It runs on the
  Initial CLI once DOS is up, reports again, opens `rusttest.device` by name
  and reads unit 0 with DoIO.

To add it, put the file in the zip as `Kickstart/rusttest.device` and add a
line to `Kickstart/Kicklayout` inside the configuration (before the blank
line that ends it):

```
MODULE Kickstart/rusttest.device
```

The serial output ends with `rusttest.device: kickstart load test PASSED`
(tested on QEMU's AmigaOne).

### Projects built on the crate

- [VirtioRust](https://github.com/derfsss/VirtioRust): drivers for QEMU's
  virtio devices (virtionet.device, virtiorng.device, virtioscsi.device),
  with their test programs and benches. The crate keeps the generic
  `virtio` layer they use.
- [RustSocket](https://github.com/derfsss/RustSocket): rustsocket.library,
  a Roadshow-compatible TCP/IP stack (bsdsocket.library) on smoltcp.

Both use this crate as a git submodule.

### Host tests

The crate builds on any host for testing:

```bash
cargo test                          # default features
cargo test --no-default-features    # the driver configuration
cargo test --features full-sdk      # every interface
cargo test --features device        # the device module (and its SDK check)
```

`tests/sdk_audit.rs` checks the bindings against the SDK headers: every SDK
method must exist in Rust, and every Rust vtable must have the SDK's slots in
the SDK's order. It reads the headers from `sdk/include/include_h` in the
repository (or from `$AMIGA_SDK/include/include_h`), and passes with a notice
when they are absent. `sdk/` is git-ignored: the SDK headers must not be
committed. To get SDK 54.25 out of the image:

```bash
mkdir -p sdk/include
id=$(docker create walkero/amigagccondocker:os4-gcc15-3.6.1)
docker cp $id:/opt/ppc-amigaos/ppc-amigaos/SDK/include/include_h sdk/include/
docker cp $id:/opt/ppc-amigaos/ppc-amigaos/SDK/include/netinclude sdk/include/
docker rm $id
cargo test --test sdk_audit
cargo test --features device --test device_consts
cargo test --features sana2 --test sana2_consts
cargo test --features bsdsocket --test bsdsocket_consts
```

`tests/device_consts.rs` checks `amigaos4::device`'s command, error and
flag values against exec/io.h, exec/errors.h and devices/newstyle.h, and
fails by name on any it cannot find.
`tests/sana2_consts.rs` does the same for `amigaos4::sana2` against
devices/sana2.h, evaluating the header's expressions (shifts, sums and
named constants). `tests/bsdsocket_consts.rs` checks `amigaos4::bsdsocket`
against netinclude (sys/socket.h, netinet/, libraries/bsdsocket.h), and its
statistics field tables against the structs. The audit finds bsdsocket's
interface in `netinclude/interfaces`.

## Notes

- The crate needs the AmigaLabs compiler because `powerpc-unknown-amigaos`
  exists only in that fork (a tier 3 target, not in upstream Rust). The
  PowerPC `asm!` in `arch` needs no feature gate on it.
- The byte-reversed MMIO helpers (`lwbrx`/`stwbrx`) work where PCI memory is
  mapped directly, as on the Pegasos II, but not on the AmigaOne (Articia S
  bridge).
- The sysroot's `compiler_builtins` exports no memory functions, which is why
  `glue/freestanding_mem.c` exists. Its functions are weak, so a definition
  elsewhere in the link wins.

## Credits

- [AmigaLabs/rust](https://github.com/AmigaLabs/rust) (branch `amigaos-port`):
  the Rust fork with the `powerpc-unknown-amigaos` target this crate is
  built for.
- [AmigaLabs/clib4](https://github.com/AmigaLabs/clib4): the C library
  that programs' `std` runs on.
- [walkero-gr/amigagccondocker](https://github.com/walkero-gr/amigagccondocker):
  the cross-compiler Docker image (gcc and the AmigaOS SDK) the packages
  link in.
- [ash-rs/ash](https://github.com/ash-rs/ash): the Vulkan binding behind
  the optional `ash` feature.
- [KhronosGroup/Vulkan-Headers](https://github.com/KhronosGroup/Vulkan-Headers):
  the Vulkan headers copied into `third_party/vulkan` (Apache-2.0).
- [derfsss/VulkanOS4](https://github.com/derfsss/VulkanOS4): where the
  VK_AMIGA_surface extension's shape (`vulkan_amiga.h`) comes from.
- [derfsss/rust-for-amigaos4](https://github.com/derfsss/rust-for-amigaos4):
  the earlier crate this one supersedes.
- [derfsss/VirtualSCSIDevice](https://github.com/derfsss/VirtualSCSIDevice)
  and [derfsss/Virtio9PFS-handler](https://github.com/derfsss/Virtio9PFS-handler):
  the C drivers whose MMIO practice the `arch` helpers follow.
- The AISS toolbar images in `apps/imgview` are Martin Merz's
  ([masonicons.de](http://www.masonicons.de)); see that app's README.
- The AmigaOS SDK (Hyperion Entertainment) is what the bindings are
  checked against; its headers are not part of this repository.

## Licence

MIT. See [LICENSE](LICENSE).

---

Richard Gibbs — rich_gibbs@hotmail.com
