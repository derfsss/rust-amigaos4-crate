# Memory

This page covers how a Rust program gets memory from AmigaOS through the
`amigaos4` crate:

| Module | What it gives you | Cargo feature |
|---|---|---|
| `allocator` | A global allocator for code without a C runtime | none |
| `mem` | Owned OS memory blocks, exec pools, memory queries, DMA buffers | none |
| `pool` | Protected memory pools, typed item pools, number-range maps | none |
| `named` | Memory blocks found by name, shared between programs | none |

All four are always built. They work in both build modes:

- a **std program** (the default `std` feature, running on clib4), and
- a **no-C-runtime build** (`#![no_std]`, default features off: device
  drivers, libraries, handlers and small tools).

## Some AmigaOS terms first

- **exec** is the AmigaOS kernel library. Every memory call here ends up in
  exec.
- **Memory type flags** (`MEMF_*`, in `amigaos4::sys`) say what kind of
  memory you want. The two that matter most:
  - `MEMF_PRIVATE`: "only visible to the allocator task". Another task
    must not touch it.
  - `MEMF_SHARED`: visible to all tasks. Use it for anything another task,
    a device or an interrupt will read or write.
- **Task**: an exec thread of execution. A **process** is a task that can
  also call dos.library.
- **AllocSysObject**: exec's general "make me an object of this kind"
  call. Pools, item pools and resource maps are made with it.

---

## `allocator`: a global allocator without a C runtime

A std program does not need this module. clib4 provides the heap, and
`Vec`, `Box` and `String` just work.

Code built without a C runtime has no heap until you name one. The crate
offers two `GlobalAlloc` types, both backed by exec's
`AllocVecTagList`/`FreeVec`. They differ only in the memory type:

| Type | Memory | Use it for |
|---|---|---|
| `ExecAllocator` | `MEMF_PRIVATE` | A `#![no_std]` program whose heap only one task uses. |
| `SharedExecAllocator` | `MEMF_SHARED` | A device, library or handler. Also a `#![no_std]` program whose spawned tasks share heap data (for example an `Arc` created by one task and dropped by another). |

Why a device or library must use `SharedExecAllocator`: its `Open`,
`Close` and `BeginIO` entry points run in the **caller's** task, and may
allocate there. Its heap is used by many tasks, so it cannot be one task's
private memory. Using `MEMF_PRIVATE` here has crashed real systems.

```toml
[dependencies]
amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", default-features = false, features = ["dos"] }
```

```rust
#![no_std]
#![no_main]

extern crate alloc;

use amigaos4::allocator::SharedExecAllocator;

#[global_allocator]
// MEMF_SHARED: BeginIO, Open and Close run in the callers' tasks.
static ALLOCATOR: SharedExecAllocator = SharedExecAllocator;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    amigaos4::panic::default_panic_handler(info)
}
```

Limitations from the source:

- The `IExec` interface global must be set before the first allocation.
  The C start-up glue in the templates sets it before calling into Rust.
  If `IExec` is NULL, allocation returns null (an allocation failure).
- Exec returns blocks aligned to at least 16 bytes. For a type that needs
  more, the allocator over-allocates and stores the original pointer just
  before the aligned block. That is automatic, but costs a little memory.
- For DMA buffers use `mem::DmaBuffer` (below), not the global heap.

---

## `mem`: OS memory blocks, pools and queries

### `AmigaVec`: a block with chosen memory flags

`AmigaVec` is a block from `AllocVecTagList`, freed when it is dropped.
Use it when you need memory of a particular type and do not want it to go
through the global allocator.

| Method | What it does |
|---|---|
| `AmigaVec::alloc(size, mem_type)` | Allocate `size` bytes of `MEMF_*` type `mem_type`. |
| `AmigaVec::alloc_cleared(size, mem_type, fill)` | The same, every byte set to `fill`. |
| `as_slice()`, `as_mut_slice()` | The block as a byte slice. |
| `as_ptr()`, `as_mut_ptr()`, `len()`, `is_empty()` | Raw access and size. |

```rust
use amigaos4::mem::AmigaVec;
use amigaos4::sys::MEMF_SHARED;

fn shared_buffer() -> amigaos4::Result<()> {
    let mut buf = AmigaVec::alloc_cleared(4096, MEMF_SHARED, 0)?;
    buf.as_mut_slice()[0] = 0x42;
    assert_eq!(buf.len(), 4096);
    Ok(()) // freed here
}
```

Both constructors return `Err(AmigaError::AllocationFailed)` when exec has
no memory, or when `size` does not fit in 32 bits.

### `AmigaPool`: an exec memory pool

An exec pool (`CreatePool`/`AllocPooled`) carves many small allocations
out of larger chunks called **puddles**. Dropping the pool frees
everything in it at once, even allocations you never freed one by one.
That suits a piece of work whose memory should all go away together.

```rust
use amigaos4::mem::AmigaPool;
use amigaos4::sys::MEMF_PRIVATE;

fn pool_demo() -> amigaos4::Result<()> {
    // 8 KiB puddles; requests up to 2 KiB come from puddles.
    let pool = AmigaPool::new(MEMF_PRIVATE, 8192, 2048)?;
    let p = pool.alloc(100); // a raw pointer; null on failure
    if !p.is_null() {
        // SAFETY: p came from this pool's alloc with size 100.
        unsafe { pool.free(p, 100) }; // optional
    }
    Ok(()) // DeletePool frees whatever is left
}
```

- `new(mem_type, puddle, thresh)`: `thresh` is the largest allocation
  served from puddles and must be no bigger than `puddle`. Larger ones get
  a block of their own.
- `alloc(size)` returns a raw `*mut u8` (null on failure). The pointer is
  not tied to the pool's lifetime by the compiler: do not use it after the
  pool is dropped.
- `free(ptr, size)` is `unsafe`: `ptr` must come from this pool with the
  same `size`.

For a pool that several tasks share safely, and for blocks that free
themselves, use `pool::MemPool` instead.

### Memory queries and copies

| Function | Exec call | What it does |
|---|---|---|
| `avail(flags)` | AvailMem | Free memory of a kind: `MEMF_ANY`, `MEMF_SHARED`, `MEMF_LARGEST` (largest free block), `MEMF_TOTAL`... |
| `type_of(&value)` | TypeOfMem | The `MEMF_*` attributes of the memory `value` lives in. 0 for memory exec does not manage (a stack may be such memory). |
| `copy(src, dst)` | CopyMem | Copy bytes. Panics if the lengths differ, as `copy_from_slice` does. |
| `copy_quick(src, dst)` | CopyMemQuick | Copy `[u32]` slices (longword aligned, whole longwords). Panics if the lengths differ. |

```rust
use amigaos4::mem;
use amigaos4::sys::{MEMF_ANY, MEMF_LARGEST};

fn report() {
    let free = mem::avail(MEMF_ANY);
    let largest = mem::avail(MEMF_LARGEST);
    let src = [1u8, 2, 3, 4];
    let mut dst = [0u8; 4];
    mem::copy(&src, &mut dst);
    let _ = (free, largest);
}
```

### `MemLock`: keep memory in physical RAM

`MemLock::new(&slice)` calls `LockMem`, so the memory cannot be paged out
while the lock lives. It returns `None` if exec refuses. Locks nest; each
is undone on drop (`UnlockMem`).

### `DmaBuffer` and `DmaMapping`: memory a device reads or writes

**DMA** (direct memory access) means a hardware device reads or writes RAM
itself, without the CPU. On AmigaOS this must go through exec's
`StartDMA`/`GetDMAList`/`EndDMA` calls. They flush or invalidate the CPU
caches correctly (vital on machines whose caches are not kept coherent
with DMA, such as the Sam460), and they give the **physical** addresses
the device needs, which can differ from the CPU's addresses and can be
split into several blocks.

- `DmaBuffer::alloc(size)` gives zeroed `MEMF_SHARED` memory.
- `DmaBuffer::alloc_contiguous(size, align)` gives one physically
  contiguous block, aligned to `align` (a power of two), for structures a
  device addresses by one base address.
- `buf.map(DmaDirection::ToDevice)` (or `FromDevice`) returns a
  `DmaMapping`. Its `entries()` are the physical blocks to give the device.
  The mapping borrows the buffer mutably, so the CPU cannot touch the
  buffer while the device owns it. Dropping the mapping runs `EndDMA`.

```rust
use amigaos4::mem::{DmaBuffer, DmaDirection};

fn send_to_device(data: &[u8]) -> amigaos4::Result<()> {
    let mut buf = DmaBuffer::alloc(4096)?;
    buf.as_mut_slice()[..data.len()].copy_from_slice(data);
    {
        let map = buf.map(DmaDirection::ToDevice)?;
        for e in map.entries() {
            // Program one descriptor per block:
            // e.PhysicalAddress, e.BlockLength
            let _ = (e.PhysicalAddress, e.BlockLength);
        }
        // ... start the transfer and wait for it to finish ...
    } // EndDMA here
    Ok(())
}
```

Never give a device `buf.as_ptr()`: that is the CPU's virtual address.
For the full story (mapping a caller's buffer, aborting a transfer,
long-lived mappings, virtio queues) see
[Writing device drivers](../devices/writing-devices.md#dma-the-right-way).

---

## `pool`: protected pools, item pools and resource maps

These are exec's allocators beyond `AllocVec`, all made with
`AllocSysObject`. Blocks and items **borrow** their pool, so the compiler
makes sure a pool outlives everything taken from it.

### `MemPool`: a pool tasks can share

`MemPool` is a pool of variable-size blocks (`AllocVecPooled`), created
**protected**: exec locks it on every allocation and free. It is `Send`
and `Sync`, so you can share it between tasks (for example in an `Arc`).

```rust
use amigaos4::pool::MemPool;
use amigaos4::sys::MEMF_SHARED;

fn mempool_demo() -> amigaos4::Result<()> {
    // puddle 8 KiB, threshold 2 KiB (0 and 0 pick exec's defaults)
    let pool = MemPool::new(MEMF_SHARED, 8192, 2048)?;
    if let Some(mut block) = pool.alloc(256) {
        block[0] = 1; // a PoolBlock derefs to [u8], zeroed
    } // the block goes back to the pool here
    Ok(())
}
```

- `alloc(size)` returns `Option<PoolBlock>`: `None` when the pool cannot
  grow (or `size` is 0).
- Use `MEMF_SHARED` for memory other tasks touch.

### `ItemPool<T>`: fixed-size items of one type

An item pool hands out equal-sized items quickly. `ItemPool<T>` stores one
`T` per item, in `MEMF_SHARED` memory, protected. A good fit for a
driver's per-request state.

```rust
use amigaos4::pool::ItemPool;

struct Job { id: u32, len: u32 }

fn itempool_demo() -> amigaos4::Result<()> {
    let pool: ItemPool<Job> = ItemPool::new()?;   // no limit
    match pool.alloc(Job { id: 1, len: 512 }) {
        Ok(item) => assert_eq!(item.id, 1),       // PoolItem derefs to Job
        Err(_job) => { /* pool full or out of memory: value handed back */ }
    }
    let small: ItemPool<u64> = ItemPool::with_limit(64, 16)?; // at most 64, 16 at a time
    small.gc(); // give unused memory back (ItemPoolGC)
    Ok(())
}
```

- `with_limit(max_items, batch)`: `max_items` 0 means no limit; `batch` 0
  lets exec choose; `max_items` must be a multiple of `batch`.
- `alloc(value)` returns `Err(value)` when the pool is full, so nothing is
  lost.
- Dropping a `PoolItem` drops the `T` and returns the item.
- Limitation: `T` may be aligned to at most 8 bytes (the constructor
  panics otherwise).

### `ResourceMap`: ranges of a number space

A resource map hands out ranges of an abstract number space, such as I/O
addresses or IDs. It is not memory.

```rust
use amigaos4::pool::ResourceMap;

fn rmap_demo() -> amigaos4::Result<()> {
    // numbers 0x10000..0x20000, handed out in multiples of 16
    let map = ResourceMap::new(0x10000, 0x10000, 16)?;
    if let Some(r) = map.alloc(4096) {
        let _first = r.start();
    } // range returned here
    let _aligned = map.alloc_aligned(4096, 4096);
    Ok(())
}
```

Limitations from the source:

- `base` must not be 0 (exec reports a failed allocation as 0), and
  `base + size` must fit in 32 bits.
- `quantum` 0 means exec's default.
- `ResourceMap` is `Send` but **not** `Sync`: exec does not lock a map, so
  do not share one between tasks without your own lock (for example
  `sync::Mutex`).

---

## `named`: memory found by name

**Named memory** (exec V51 and later) is a block found by a namespace and
a name instead of a pointer. Different programs can share it. The
`"resident"` namespace survives a reboot.

| Function | What it does |
|---|---|
| `named::alloc(space, name, size)` | Make a zeroed block. `space` is `None` for the global namespace. `Err(AmigaError::Exists)` if the name is taken. |
| `named::alloc_with(space, name, size, checksum)` | The same, choosing whether exec keeps a checksum. A resident block whose checksum fails after a reboot is discarded. |
| `named::exists(space, name)` | Is there such a block? |
| `named::lock(space, name)` | Exclusive access, waiting for another holder. `None` if there is no such block or it was not made by this module. |

A `NamedLock` derefs to the block's bytes. It is released on drop. Its
methods:

- `update_checksum()`: recompute the checksum after a change.
- `free()`: unlock and free the block. Returns `false` if exec refused.

```rust
use amigaos4::named;

fn named_demo() -> amigaos4::Result<()> {
    let space = Some("myprog");
    if !named::exists(space, "settings") {
        named::alloc(space, "settings", 256)?;
    }
    if let Some(mut block) = named::lock(space, "settings") {
        block[0] = 1;       // exclusive access while `block` lives
    }
    // Later, to remove it:
    if let Some(block) = named::lock(space, "settings") {
        block.free();
    }
    Ok(())
}
```

Limitations from the source:

- `named::alloc` makes a block **without** a checksum. Use `alloc_with`
  with `checksum = true` if you want one.
- Each block made by this module starts with an 8-byte header holding a
  magic word and the size, because exec has no cheap way to ask a block's
  size. `lock` refuses blocks without that header (blocks made by other
  software). The header is trusted: another program that overwrites it
  can make this one read past the block.
- There is no `try_lock`. exec's `AttemptNamedMemory`, documented as
  never blocking, can spin in the kernel and stall the machine while
  another task holds the block. `lock` waits properly instead.
- `NamedLock` is not `Send`: unlock it on the task that locked it.

---

## Which one do I want?

| Need | Use |
|---|---|
| Ordinary heap in a std program | `Vec`, `Box` (clib4's heap) |
| Heap in a no_std driver, library or handler | `SharedExecAllocator` |
| Heap in a single-task no_std program | `ExecAllocator` |
| A block of a particular memory type | `mem::AmigaVec` |
| Many small blocks freed together | `mem::AmigaPool` or `pool::MemPool` |
| A pool shared between tasks | `pool::MemPool` |
| Many objects of one type | `pool::ItemPool<T>` |
| IDs or address ranges | `pool::ResourceMap` |
| Memory a device reads or writes | `mem::DmaBuffer` + `DmaMapping` |
| A block other programs find by name | `named` |
