#!/bin/bash
# Builds one package (an example or a project made from a template) with the
# "amigaos" toolchain.
#
# - A program (no Makefile): an ordinary Rust binary on std and clib4.
#   cargo links it itself; its .cargo/config.toml names tools/ppc-amigaos-gcc
#   as the linker, which runs the cross-compiler image's gcc when none is
#   installed. The executable is target/powerpc-unknown-amigaos/release/<name>.
# - A driver, library, device or other no-CRT package (with a Makefile): a
#   staticlib, then linked by the package's Makefile inside the image.
#
#   tools/build-package.sh apps/vkplasma
#   tools/build-package.sh examples/ram-device
#   MAKE_ARGS="X=1" tools/build-package.sh <package> --features <feature>
#
# Run on Linux (WSL works) after tools/build-toolchain.sh. The package must
# live inside this repository, which is mounted into the container.
set -euo pipefail

PKG="${1:?usage: tools/build-package.sh <package dir> [cargo build arguments]}"
shift
IMAGE="${AMIGA_GCC_IMAGE:-walkero/amigagccondocker:os4-gcc15-3.6.1}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

docker image inspect "$IMAGE" >/dev/null 2>&1 || {
    echo "FAIL: docker cannot see $IMAGE (docker pull $IMAGE)"; exit 1; }

cd "$REPO/$PKG"
# The linked toolchain has no cargo of its own; rustup falls back to the
# default toolchain's cargo, which drives the amigaos rustc.
cargo +amigaos build --release "$@"

if [ ! -f Makefile ]; then
    # A program: cargo has linked it. Name the executable(s) it made.
    n=0
    for f in target/powerpc-unknown-amigaos/release/*; do
        if [ -f "$f" ] && [ -x "$f" ] && [ "${f%.d}" = "$f" ]; then
            echo "program: $f ($(stat -c %s "$f") bytes)"
            n=$((n+1))
        fi
    done
    [ "$n" -gt 0 ] || { echo "FAIL: no executable in target/powerpc-unknown-amigaos/release"; exit 1; }
    exit 0
fi

LIB="$(ls target/powerpc-unknown-amigaos/release/*.a)"
echo "staticlib: $LIB ($(stat -c %s "$LIB") bytes)"

# MAKE_ARGS passes make variables through, e.g. MAKE_ARGS="DISK=1".
docker run --rm -e MAKE_ARGS="${MAKE_ARGS:-}" -v "$REPO:/repo" -w "/repo/$PKG" "$IMAGE"     sh -c 'make clean $MAKE_ARGS >/dev/null && make $MAKE_ARGS'
