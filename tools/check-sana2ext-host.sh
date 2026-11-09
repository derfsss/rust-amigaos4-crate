#!/bin/bash
# Compile tests/layout/sana2ext.c with the host's compilers, against the
# stand-in exec headers in tests/host/sana2ext (natural alignment, no
# pack(2)): the extension layout must be the same on other CPUs and with
# 64-bit pointers (docs/devices/sana2ext.md, Portability). Run on Linux
# (WSL works): bash tools/check-sana2ext-host.sh
# Compilers that are not installed are skipped, saying so; at least one
# must run.
set -u
here=$(cd "$(dirname "$0")/.." && pwd)
src="$here/tests/layout/sana2ext.c"
inc="-I$here/tests/host/sana2ext -I$here/include"
n=0
fail=0
for cc in gcc "gcc -m32" clang aarch64-linux-gnu-gcc arm-linux-gnueabihf-gcc powerpc-linux-gnu-gcc powerpc64-linux-gnu-gcc; do
    bin=${cc%% *}
    if ! command -v "$bin" >/dev/null 2>&1; then
        echo "--   $cc (not installed)"
        continue
    fi
    if out=$($cc -std=c11 -fsyntax-only $inc "$src" 2>&1); then
        echo "ok   $cc"
        n=$((n+1))
    elif echo "$out" | grep -q "No such file or directory"; then
        echo "--   $cc (its C headers are missing)"
    else
        echo "FAIL $cc"
        echo "$out" | head -5
        fail=1
    fi
done
[ "$n" -gt 0 ] || { echo "no compiler ran" >&2; exit 1; }
exit $fail
