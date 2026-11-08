/* vulkan.rs against vulkan.library's SDK (check-layout.sh adds its include
 * paths for this file only): struct VulkanIFace's head and
 * VkAmigaSurfaceCreateInfoAMIGA. */
#include <interfaces/vulkan.h>
#include <vulkan/vulkan_amiga.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
A(offsetof(struct VulkanIFace, Obtain) == 60);
A(offsetof(struct VulkanIFace, vkCreateInstance) == 76);
A(offsetof(struct VulkanIFace, vkGetPhysicalDeviceMemoryProperties) == 108);
A(offsetof(struct VulkanIFace, vkGetInstanceProcAddr) == 112);
A(sizeof(VkAmigaSurfaceCreateInfoAMIGA) == 20);
A(offsetof(VkAmigaSurfaceCreateInfoAMIGA, sType) == 0 && sizeof(VkStructureType) == 4);
A(offsetof(VkAmigaSurfaceCreateInfoAMIGA, pNext) == 4 && offsetof(VkAmigaSurfaceCreateInfoAMIGA, flags) == 8);
A(offsetof(VkAmigaSurfaceCreateInfoAMIGA, pScreen) == 12 && offsetof(VkAmigaSurfaceCreateInfoAMIGA, pWindow) == 16);
A(VK_STRUCTURE_TYPE_AMIGA_SURFACE_CREATE_INFO_AMIGA == 1099998000);
A(VK_AMIGA_SURFACE_SPEC_VERSION == 1);
A(sizeof(VkSurfaceKHR) == 8 && sizeof(VkInstance) == 4);
/* "VK_AMIGA_surface" is 16 characters. */
A(sizeof(VK_AMIGA_SURFACE_EXTENSION_NAME) == 17);
