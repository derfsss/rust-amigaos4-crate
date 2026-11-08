#!/bin/bash
# Builds the AmigaLabs Rust compiler (branch amigaos-port) and its
# powerpc-unknown-amigaos sysroot, then links it as the rustup toolchain
# "amigaos". Run on Linux (WSL works). Follows the fork's README_AMIGA.md.
#
#   tools/build-toolchain.sh [tree]      default tree: ~/amigaos-rust
#
# Needs: git, python3, a C toolchain, rustup (~/.cargo/bin). Docker is only
# needed later, for linking AmigaOS programs; the build here never calls gcc.
set -euo pipefail

TREE="${1:-$HOME/amigaos-rust}"
export AMIGA_GCC_IMAGE="${AMIGA_GCC_IMAGE:-walkero/amigagccondocker:os4-gcc15-3.6.1}"

if [ ! -d "$TREE/.git" ]; then
    # The depth must reach the upstream bors merge so download-ci-llvm finds it.
    git clone --depth 80 --branch amigaos-port https://github.com/AmigaLabs/rust.git "$TREE"
fi
cd "$TREE"
git log -1 --format='HEAD %h %ad %s' --date=short

[ -f bootstrap.toml ] || cp amiga/bootstrap.toml.example bootstrap.toml
export PATH="$TREE/amiga/bin:$PATH"

BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build compiler/rustc
# One invocation for both targets: each ./x build rebuilds the stage1 sysroot
# only for the targets it names.
BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build library/std library/proc_macro library/test \
    --target x86_64-unknown-linux-gnu,powerpc-unknown-amigaos

SYSROOT="$TREE/build/x86_64-unknown-linux-gnu/stage1"
LIB="$SYSROOT/lib/rustlib/powerpc-unknown-amigaos/lib"
for c in core alloc compiler_builtins std; do
    ls "$LIB"/lib$c-*.rlib >/dev/null 2>&1 || { echo "FAIL: lib$c missing in $LIB"; exit 1; }
done

"$HOME/.cargo/bin/rustup" toolchain link amigaos "$SYSROOT"
"$SYSROOT/bin/rustc" --version
echo "DONE: rustup toolchain 'amigaos' -> $SYSROOT"
