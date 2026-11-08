# Vulkan headers for AmigaOS 4

The headers `tests/layout/vulkan.c` (run by `tools/check-layout.sh`)
checks the `vulkan` module's structure layouts against. The AmigaOS SDK
has no Vulkan headers, so a copy is kept here.

| Path | What | Licence |
|---|---|---|
| `include/vulkan/vulkan_core.h`, `vk_platform.h`, `include/vk_video/*` | the Khronos Vulkan headers (the version AmigaOS 4's Vulkan loader is built with) | Apache-2.0 (The Khronos Group Inc.; notices in each file) |
| `include/vulkan/vulkan_amiga.h` | VK_AMIGA_surface: Vulkan surfaces on Intuition windows | MIT |
| `include/interfaces/vulkan.h` | `struct VulkanIFace`, the "main" interface `vulkan.library` publishes | MIT |

To check against a different SDK, set `VULKAN_SDK_INCLUDE` to an include
directory holding the same files.
