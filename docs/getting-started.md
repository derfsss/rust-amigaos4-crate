# Getting started

This guide takes you from nothing to a Rust program running on AmigaOS 4.1.
It covers installing the compiler, building the program template, and
running the result on an Amiga or in QEMU.

You need:

- a Linux machine, or Windows with **WSL** (Windows Subsystem for Linux);
- `git`, `python3`, a C compiler for the host, and **rustup**;
- **Docker**, for the AmigaOS cross-compiler (gcc and the AmigaOS SDK);
- an AmigaOS 4.1 machine or an emulator (QEMU) with **clib4.library 2.5 or
  newer** installed, if you build programs that use `std`.

All commands below run in a Linux or WSL shell, from the root of the crate's
repository.

## 1. Get the repository

```bash
git clone https://github.com/derfsss/rust-amigaos4-crate
cd rust-amigaos4-crate
```

The templates, examples and build scripts all live in this repository.

## 2. Build the Rust compiler

AmigaOS 4 is not a target in standard Rust. It comes from the AmigaLabs fork
of the compiler (branch `amigaos-port`), which adds the target
`powerpc-unknown-amigaos`. You build that compiler once:

```bash
tools/build-toolchain.sh            # builds into ~/amigaos-rust
```

The script clones the fork, builds `rustc` and the standard library for
both your host and `powerpc-unknown-amigaos`, and links the result as a
rustup toolchain called **`amigaos`**. It takes a while (it is a full
compiler build). When it ends you see:

```
DONE: rustup toolchain 'amigaos' -> /home/you/amigaos-rust/build/...
```

You can pass another folder as the first argument if you want the tree
elsewhere.

The `amigaos` toolchain has no `cargo` of its own. rustup uses the `cargo`
of your default toolchain with the AmigaLabs `rustc`, so keep a normal
stable toolchain installed (`rustup default stable`).

## 3. Get the cross-compiler image

Rust compiles your code, but AmigaOS executables are linked by
`ppc-amigaos-gcc`, which comes in a Docker image together with the AmigaOS
SDK:

```bash
docker pull walkero/amigagccondocker:os4-gcc15-3.6.1
```

The image must be visible to the Docker your Linux shell talks to. Under
WSL with its own Docker engine, images pulled in Docker Desktop are not
visible, so pull it from the WSL shell.

If you have `ppc-amigaos-gcc` installed natively and on your `PATH`, the
build uses that instead of the image for programs.

## 4. Make your first program

Start from the program template. Copy it next to the other packages, so its
relative paths keep working:

```bash
cp -r templates/program apps/hello
```

Then rename the program in `apps/hello/Cargo.toml` (both `name` lines):

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "hello"
path = "src/main.rs"

[profile.release]
opt-level = "s"
lto = true
# Not `true` or "symbols": AmigaOS will not load the file (IoErr 305).
strip = "debuginfo"

[dependencies]
# std (the default) on clib4; add the interfaces the program uses.
amigaos4 = { path = "../..", features = ["intuition"] }
```

`src/main.rs` in the template is already a working program. It reads its
arguments with an AmigaDOS template, prints a greeting with `println!`,
and with the `REQUESTER` switch shows an Intuition requester. You can
replace it with something smaller to begin with:

```rust
// $STACK sets the stack the Shell gives the program.
#[used]
static STACK: [u8; 14] = *b"$STACK:262144\0";

fn main() {
    println!("Hello from Rust on AmigaOS!");
    for (i, a) in std::env::args().enumerate() {
        println!("argument {}: {}", i, a);
    }
}
```

## 5. Build it

```bash
tools/build-package.sh apps/hello
```

The script runs `cargo +amigaos build --release` in the package. For a
program, cargo links the executable itself, through `tools/ppc-amigaos-gcc`
(named in the package's `.cargo/config.toml`). When it works you see:

```
program: target/powerpc-unknown-amigaos/release/hello (… bytes)
```

Extra arguments go to cargo, for example
`tools/build-package.sh apps/hello --features something`.

## 6. Copy it to the Amiga and run it

The executable is `apps/hello/target/powerpc-unknown-amigaos/release/hello`.
Copy it to the Amiga in whatever way you normally move files: a network
share, FTP, a shared folder or a disk image for QEMU, or a USB stick.

Then open a Shell on the Amiga and run it:

```
1.> RAM:hello one two
Hello from Rust on AmigaOS!
argument 0: RAM:hello
argument 1: one
argument 2: two
```

If the file was copied without its protection bits, make it executable
first with `Protect RAM:hello +e`.

### clib4.library

Programs that use `std` run on **clib4**, the C library the AmigaLabs
target is built on. They need **clib4.library 2.5 or newer** in `LIBS:` on
the Amiga. With older versions (2.1, or 2.4, which is the one in the
cross-compiler image's SDK) a program built here crashes during clib4's
start-up or exit code. Install a current clib4 from the AmigaLabs clib4
releases.

Drivers, libraries and devices built without a C runtime (see
[drivers-and-libraries.md](drivers-and-libraries.md)) do not use clib4 at
all.

## Using the crate outside this repository

The templates refer to the crate and the tools by relative paths
(`path = "../.."`, `linker = "../../tools/ppc-amigaos-gcc"`). If you move a
project out of the repository:

- depend on the crate through git instead of a path (the crate is not on
  crates.io):

  ```toml
  [dependencies]
  amigaos4 = { git = "https://github.com/derfsss/rust-amigaos4-crate", features = ["intuition"] }
  ```

- change `linker` in `.cargo/config.toml` to the full path of
  `tools/ppc-amigaos-gcc` in your copy of the repository;
- for no-C-runtime packages, change the `../../glue/...` paths in the
  Makefile. `tools/build-package.sh` also expects the package to be inside
  the repository, because it mounts the repository into the Docker
  container to run `make`.

## Common first errors

| Message | Cause and fix |
|---|---|
| `FAIL: docker cannot see walkero/amigagccondocker:os4-gcc15-3.6.1` | The image is not in the Docker your shell uses. Run the `docker pull` from step 3 in that shell. |
| `ppc-amigaos-gcc: no ppc-amigaos-gcc on PATH and no docker` | The linker wrapper found neither a native gcc nor Docker. Install Docker (or a native cross gcc). |
| `error: toolchain 'amigaos' is not installed` | `tools/build-toolchain.sh` has not finished, or the `rustup toolchain link` step failed. Run it again. |
| cargo itself is not found | rustup has no default toolchain. Run `rustup default stable`. |
| `FAIL: no executable in target/powerpc-unknown-amigaos/release` | The build did not produce a program. Check that `Cargo.toml` has a `[[bin]]` and that the package has no `Makefile` (a Makefile makes the script treat it as a driver). |
| Mismatched array size in `static STACK` or `static VERSION` | The array length must equal the number of bytes in the string, including the final `\0`. |
| AmigaOS says the file is not executable, or IoErr 305 | The release profile has `strip = true` or `strip = "symbols"`. Use `strip = "debuginfo"`. Also check the file's `e` protection bit. |
| The program crashes as it starts or exits | clib4.library on the Amiga is older than 2.5. Update it. |

More problems and their causes are in
[troubleshooting.md](troubleshooting.md).

## Next steps

- [concepts.md](concepts.md): the AmigaOS ideas behind the crate's types.
- [programs.md](programs.md): writing ordinary programs.
- [features.md](features.md): which Cargo features turn on which parts of
  the crate.
- [debugging.md](debugging.md): debug output and what happens on a panic.
