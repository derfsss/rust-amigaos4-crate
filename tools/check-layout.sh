#!/bin/bash
# Compile every tests/layout/*.c against the image's SDK: each is a list of
# _Static_assert()s on the structure layouts the crate relies on, so a
# successful compile is the check. Run on Linux (WSL works) with Docker:
#   bash tools/check-layout.sh
#
# tests/layout/sana2ext.c checks the crate's own include/devices/sana2ext.h.
# tests/layout/vulkan.c also needs the Vulkan headers, which are not in the
# AmigaOS SDK: by default the copy in third_party/vulkan/include; set
# VULKAN_SDK_INCLUDE to another include directory to check against that.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
image=walkero/amigagccondocker:os4-gcc15-3.6.1
vk="${VULKAN_SDK_INCLUDE:-$here/third_party/vulkan/include}"
files=$(cd "$here" && ls tests/layout/*.c)
[ -n "$files" ] || { echo "no layout files" >&2; exit 1; }
[ -f "$vk/interfaces/vulkan.h" ] && [ -f "$vk/vulkan/vulkan_amiga.h" ] || {
    echo "FAIL: no Vulkan headers (interfaces/vulkan.h, vulkan/vulkan_amiga.h) in $vk for tests/layout/vulkan.c" >&2; exit 1; }
vk=$(cd "$vk" && pwd)
docker run --rm -v "$here:/src" -v "$vk:/vk:ro" -w /src "$image" bash -c '
set -e
n=0
for f in tests/layout/*.c; do
    extra=""
    if [ "$f" = tests/layout/vulkan.c ]; then
        extra="-I/vk"
    elif [ "$f" = tests/layout/sana2ext.c ]; then
        extra="-I/src/include"
    fi
    ppc-amigaos-gcc -fsyntax-only -D__USE_INLINE__ $extra "$f"
    echo "ok $f"
    n=$((n+1))
done
[ "$n" -gt 0 ] || { echo "nothing compiled" >&2; exit 1; }
'
