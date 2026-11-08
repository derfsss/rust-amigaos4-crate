# ELF: reading program files and loading shared objects

The `elf` module wraps **elf.library**, the AmigaOS library that reads ELF
files. ELF (Executable and Linkable Format) is the file format of AmigaOS 4
programs, libraries and shared objects (`.so` files).

## What it is for

- **Inspecting an ELF file.** `ElfFile` opens a file (or bytes in memory) and
  gives you its header, its sections (named blocks such as `.text` for code
  and `.rodata` for constant data), the bytes of a section, and its symbols
  (named functions and variables).
- **Loading a shared object into your program.** `SharedObject` loads a `.so`
  file and gives you the addresses of its symbols by name.

Use `ElfFile` for tools that look at programs: a symbol lister, a size
report, a checker. Use `SharedObject` only when you must call into a C shared
object, and read the safety section below first.

To load and run another program, use `dos::process::SegList` instead (see
[dos.md](dos.md)).

## Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", features = ["elf"] }
```

| Feature | Gives | Pulls in |
|---|---|---|
| `elf` | `ElfFile`, `ElfHeader`, `SectionInfo`, `Symbol`, `SharedObject` | `dos` |

## Build modes

| Mode | Status |
|---|---|
| no-C-runtime program (`#![no_std]`) | `ElfFile` is tested by the crate's test programs: header, sections, section bytes, symbols by name, by address and all of them, from a file and from memory, and refusal of files that are not ELF. `SharedObject` is tested only for the failure path (a missing object). Loading a real object from such a program is not expected to work: see below. |
| std program (`fn main` on clib4) | Compiles; not exercised by the crate's test programs. |

You need no interface global of your own. If `IElf` is NULL the crate opens
elf.library itself; a std program gets clib4's `IElf`.

**Opening needs a Process.** `ElfFile::from_memory`, `ElfFile::open` and
`SharedObject::open` check that the caller is a DOS Process (a task with the
extra state dos.library needs) and fail with `AmigaError::WrongTask`
otherwise. Programs started from the Shell or Workbench are Processes; a
plain exec task started with the crate's `task` module is not.

## `ElfFile`

```rust
use amigaos4::elf::ElfFile;

let f = ElfFile::open("C:Dir")?;

let h = f.header()?;
// h.class == 1 (32-bit), h.data == 2 (big-endian), h.machine == 20 (PowerPC)

for s in f.sections() {
    // s.index, s.name, s.kind, s.addr, s.offset, s.size, ...
}

if let Some(text) = f.section_by_name(".text") {
    let bytes = f.section_data(text.index); // Option<Vec<u8>>
}

if let Some(sym) = f.find_symbol("_start") {
    // sym.value: the value in the file, as readelf shows it
    // sym.address: where it is in elf.library's loaded copy
    let same = f.symbol_at(sym.address);
}

let all = f.symbols(); // every entry of the symbol table
```

Everything it returns is **copied**, so the results stay valid after the
`ElfFile` is dropped. Dropping it closes the file (CloseElf).

### Opening

| Function | Opens |
|---|---|
| `ElfFile::open(path: &str) -> Result<ElfFile<'static>>` | A file by path. |
| `ElfFile::from_memory(image: &'a [u8]) -> Result<ElfFile<'a>>` | An ELF image in memory. The bytes are borrowed while the handle lives. Fewer than 52 bytes, or no `\x7fELF` at the start, is refused. |

A missing file, a text file, or bytes that are not ELF give an error.

### Queries

| Method | Returns |
|---|---|
| `header(&self) -> Result<ElfHeader>` | Class, byte order, kind (1 relocatable, 2 executable, 3 shared object), machine, entry point, section count, string section. |
| `section_count(&self) -> u32` | The number of sections. |
| `section(&self, index: u32) -> Option<SectionInfo>` | One section header with its name. |
| `sections(&self) -> Vec<SectionInfo>` | Every section. |
| `section_by_name(&self, name: &str) -> Option<SectionInfo>` | The first section with that name. |
| `section_data(&self, index: u32) -> Option<Vec<u8>>` | A section's bytes. Empty for a section with no bytes in the file, such as `.bss`. |
| `find_symbol(&self, name: &str) -> Option<Symbol>` | A symbol by name. |
| `symbol_at(&self, address: u32) -> Option<Symbol>` | The symbol at a loaded address. |
| `symbols(&self) -> Vec<Symbol>` | Every symbol table entry, including the empty entry 0. |

### Value and address

elf.library loads the sections it reads into memory. So a `Symbol` has two
numbers:

- `value` is the symbol's value in the file (what `readelf` shows).
- `address` is where the symbol is in elf.library's loaded copy. It is what
  `symbol_at` takes. It is 0 for a symbol in no loaded section.

Each `ElfFile` handle loads its own copy, so two handles on the same file
give different addresses for the same symbol. `kind` holds the symbol type
(0 none, 1 object, 2 function, 3 section, 4 file) and `bind` its binding
(0 local, 1 global, 2 weak).

Release builds are often stripped, so a program may have only a handful of
symbols.

## `SharedObject`

`SharedObject::open` loads a shared object into your program (DLOpen, with
its symbols global). `symbol` looks up an address by name (DLSym). Dropping
it unloads the object and everything it pulled in (DLClose).

```rust
use amigaos4::elf::SharedObject;

// SAFETY: libfoo.so is built for the C runtime this program uses.
let lib = unsafe { SharedObject::open("libfoo.so") }?;
if let Some(addr) = lib.symbol("foo_version") {
    // SAFETY: foo_version is `int foo_version(void)` in libfoo.so,
    // and `lib` outlives the call.
    let f: extern "C" fn() -> i32 = unsafe { core::mem::transmute(addr) };
    let v = f();
}
```

A relative name is looked for in the current directory, `PROGDIR:`,
`PROGDIR:sobjs/` and `SOBJS:`.

### Why `open` is unsafe

Opening a shared object runs its start-up code, and that code expects the C
runtime it was built for. `SOBJS:` holds objects built for newlib, and
`SOBJS:clib4/` objects built for clib4. A program without a C runtime (the
crate's no-C-runtime programs) most likely cannot load them: an attempt to
load `libz.so` from such a program crashed inside DLOpen. The crate cannot
check this for you, so the caller promises that:

- the object, and every object it needs, is built for the C runtime this
  program has, and
- its start-up code is sound to run in this Process.

Two more rules:

- `symbol` returns an address as a number (`Option<usize>`). Turning it into
  a function or data pointer is your own `unsafe` step. An address of 0 is a
  real answer; `None` means no such symbol.
- The address dangles once the `SharedObject` is dropped.

`SharedObject` is not `Send`: it is part of the calling Process's image.

## Limitations

- Not wrapped: ElfLoadSeg, RelocateSection, data segment copies, and handles
  of other programs.
- Only the failure path of `SharedObject::open` is tested. Loading a real
  object has not been tested.
- `symbol_at` returns names up to 255 bytes long.

## See also

- [dos.md](dos.md): `process::SegList` to load and run a program; the
  Process rule.
