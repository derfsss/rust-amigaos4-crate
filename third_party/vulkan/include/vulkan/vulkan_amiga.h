/* SPDX-License-Identifier: MIT */
/*
 * vulkan_amiga.h -- VK_AMIGA_surface: Vulkan surfaces from AmigaOS 4
 * Intuition windows and screens.
 *
 * Provenance: the extension's shape (name, create-info members, entry
 * point) is VulkanOS4's include/vulkan/vulkan_amiga.h (derfsss/VulkanOS4,
 * MIT), as AmigaOS 4's Mesa port adopted it. Changed there: the
 * extension number and so the sType. VulkanOS4 named extension number 400
 * and placed the sType at 1000400000; number 400 is now reserved in the
 * Khronos registry (VK_JUICE_extension_400) and 1000400000 is the base of
 * extension 401's block (VK_EXT_load_store_op_none). The extension is
 * unregistered, so it takes a number far above the registry's range,
 * where no registered extension will land for a long time:
 *
 *   extension number 99999 -> VkStructureType base
 *   1000000000 + (99999 - 1) * 1000 = 1099998000.
 *
 * A program built against VulkanOS4's header passes 1000400000 and is
 * refused by name, not served under another type.
 *
 * Usage: enable VK_KHR_surface and VK_AMIGA_surface on the instance, take
 * vkCreateAmigaSurfaceAMIGA from vkGetInstanceProcAddr, then create
 * swapchains through VK_KHR_swapchain as on any platform. pWindow names the
 * window presented into (its inner area); pScreen is the window's screen
 * and may be NULL, in which case the window's own WScreen is used. A
 * surface with pWindow NULL (a whole screen) is reserved and refused.
 */
#ifndef VULKAN_AMIGA_H
#define VULKAN_AMIGA_H 1

#include <vulkan/vulkan_core.h>

#ifdef __cplusplus
extern "C" {
#endif

struct Screen;
struct Window;

#define VK_AMIGA_surface 1
#define VK_AMIGA_SURFACE_SPEC_VERSION     1
#define VK_AMIGA_SURFACE_EXTENSION_NAME   "VK_AMIGA_surface"
#define VK_AMIGA_SURFACE_EXTENSION_NUMBER 99999

#define VK_STRUCTURE_TYPE_AMIGA_SURFACE_CREATE_INFO_AMIGA ((VkStructureType)1099998000)

typedef VkFlags VkAmigaSurfaceCreateFlagsAMIGA;

typedef struct VkAmigaSurfaceCreateInfoAMIGA {
    VkStructureType                sType;
    const void                    *pNext;
    VkAmigaSurfaceCreateFlagsAMIGA flags;
    struct Screen                 *pScreen;
    struct Window                 *pWindow;
} VkAmigaSurfaceCreateInfoAMIGA;

typedef VkResult (VKAPI_PTR *PFN_vkCreateAmigaSurfaceAMIGA)(
    VkInstance                           instance,
    const VkAmigaSurfaceCreateInfoAMIGA *pCreateInfo,
    const VkAllocationCallbacks         *pAllocator,
    VkSurfaceKHR                        *pSurface);

#ifndef VK_NO_PROTOTYPES
VKAPI_ATTR VkResult VKAPI_CALL vkCreateAmigaSurfaceAMIGA(
    VkInstance                           instance,
    const VkAmigaSurfaceCreateInfoAMIGA *pCreateInfo,
    const VkAllocationCallbacks         *pAllocator,
    VkSurfaceKHR                        *pSurface);
#endif

#ifdef __cplusplus
}
#endif

#endif /* VULKAN_AMIGA_H */
