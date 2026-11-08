/* SPDX-License-Identifier: MIT */
/*
 * Generated from the loader's export list -- do not edit by hand.
 *
 * struct VulkanIFace, the interface vulkan.library publishes as
 * "main": the four slots an AmigaOS interface always begins with,
 * then 263 Vulkan entry points in the order
 * vulkan-loader/loader/vulkan-1.def gives them.
 *
 * THIS IS NOT HOW AN APPLICATION USUALLY REACHES VULKAN. The core
 * of the API is reached through the function pointers
 * vkGetInstanceProcAddr and vkGetDeviceProcAddr hand back, which
 * are ordinary C pointers into the loader's trampolines and take
 * no interface pointer. This interface carries the entry points a
 * program links against BY NAME; the SDK's stub archive forwards
 * to it, and both paths reach the same loader.
 *
 * APICALL is __attribute__((libcall)) on this toolchain, so the
 * interface pointer travels implicitly and a call site does not
 * write Self. The declaration here does.
 */
#ifndef VULKAN_INTERFACE_DEF_H
#define VULKAN_INTERFACE_DEF_H

#include <exec/types.h>
#include <exec/interfaces.h>

/*
 * No VK_USE_PLATFORM_* is defined: this target has no X11, Wayland
 * or Win32 surface (its own surfaces are VK_AMIGA_surface, in
 * vulkan/vulkan_amiga.h). The loader is built the same way, so the two
 * cannot disagree about a structure.
 */
#include <vulkan/vulkan_core.h>

#ifdef __cplusplus
#ifdef __USE_AMIGAOS_NAMESPACE__
namespace AmigaOS {
#endif
extern "C" {
#endif

struct VulkanIFace
{
   struct InterfaceData Data;

   ULONG APICALL (*Obtain)(struct VulkanIFace *Self);
   ULONG APICALL (*Release)(struct VulkanIFace *Self);
   void APICALL (*Expunge)(struct VulkanIFace *Self);
   struct Interface * APICALL (*Clone)(struct VulkanIFace *Self);
   VkResult APICALL (*vkCreateInstance)(struct VulkanIFace *Self, const VkInstanceCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkInstance* pInstance);
   void APICALL (*vkDestroyInstance)(struct VulkanIFace *Self, VkInstance instance, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkEnumeratePhysicalDevices)(struct VulkanIFace *Self, VkInstance instance, uint32_t* pPhysicalDeviceCount, VkPhysicalDevice* pPhysicalDevices);
   void APICALL (*vkGetPhysicalDeviceFeatures)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkPhysicalDeviceFeatures* pFeatures);
   void APICALL (*vkGetPhysicalDeviceFormatProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkFormat format, VkFormatProperties* pFormatProperties);
   VkResult APICALL (*vkGetPhysicalDeviceImageFormatProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkFormat format, VkImageType type, VkImageTiling tiling, VkImageUsageFlags usage, VkImageCreateFlags flags, VkImageFormatProperties* pImageFormatProperties);
   void APICALL (*vkGetPhysicalDeviceProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkPhysicalDeviceProperties* pProperties);
   void APICALL (*vkGetPhysicalDeviceQueueFamilyProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pQueueFamilyPropertyCount, VkQueueFamilyProperties* pQueueFamilyProperties);
   void APICALL (*vkGetPhysicalDeviceMemoryProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkPhysicalDeviceMemoryProperties* pMemoryProperties);
   PFN_vkVoidFunction APICALL (*vkGetInstanceProcAddr)(struct VulkanIFace *Self, VkInstance instance, const char* pName);
   PFN_vkVoidFunction APICALL (*vkGetDeviceProcAddr)(struct VulkanIFace *Self, VkDevice device, const char* pName);
   VkResult APICALL (*vkCreateDevice)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkDeviceCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkDevice* pDevice);
   void APICALL (*vkDestroyDevice)(struct VulkanIFace *Self, VkDevice device, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkEnumerateInstanceExtensionProperties)(struct VulkanIFace *Self, const char* pLayerName, uint32_t* pPropertyCount, VkExtensionProperties* pProperties);
   VkResult APICALL (*vkEnumerateDeviceExtensionProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const char* pLayerName, uint32_t* pPropertyCount, VkExtensionProperties* pProperties);
   VkResult APICALL (*vkEnumerateInstanceLayerProperties)(struct VulkanIFace *Self, uint32_t* pPropertyCount, VkLayerProperties* pProperties);
   VkResult APICALL (*vkEnumerateDeviceLayerProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pPropertyCount, VkLayerProperties* pProperties);
   void APICALL (*vkGetDeviceQueue)(struct VulkanIFace *Self, VkDevice device, uint32_t queueFamilyIndex, uint32_t queueIndex, VkQueue* pQueue);
   VkResult APICALL (*vkQueueSubmit)(struct VulkanIFace *Self, VkQueue queue, uint32_t submitCount, const VkSubmitInfo* pSubmits, VkFence fence);
   VkResult APICALL (*vkQueueWaitIdle)(struct VulkanIFace *Self, VkQueue queue);
   VkResult APICALL (*vkDeviceWaitIdle)(struct VulkanIFace *Self, VkDevice device);
   VkResult APICALL (*vkAllocateMemory)(struct VulkanIFace *Self, VkDevice device, const VkMemoryAllocateInfo* pAllocateInfo, const VkAllocationCallbacks* pAllocator, VkDeviceMemory* pMemory);
   void APICALL (*vkFreeMemory)(struct VulkanIFace *Self, VkDevice device, VkDeviceMemory memory, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkMapMemory)(struct VulkanIFace *Self, VkDevice device, VkDeviceMemory memory, VkDeviceSize offset, VkDeviceSize size, VkMemoryMapFlags flags, void** ppData);
   void APICALL (*vkUnmapMemory)(struct VulkanIFace *Self, VkDevice device, VkDeviceMemory memory);
   VkResult APICALL (*vkFlushMappedMemoryRanges)(struct VulkanIFace *Self, VkDevice device, uint32_t memoryRangeCount, const VkMappedMemoryRange* pMemoryRanges);
   VkResult APICALL (*vkInvalidateMappedMemoryRanges)(struct VulkanIFace *Self, VkDevice device, uint32_t memoryRangeCount, const VkMappedMemoryRange* pMemoryRanges);
   void APICALL (*vkGetDeviceMemoryCommitment)(struct VulkanIFace *Self, VkDevice device, VkDeviceMemory memory, VkDeviceSize* pCommittedMemoryInBytes);
   VkResult APICALL (*vkBindBufferMemory)(struct VulkanIFace *Self, VkDevice device, VkBuffer buffer, VkDeviceMemory memory, VkDeviceSize memoryOffset);
   VkResult APICALL (*vkBindImageMemory)(struct VulkanIFace *Self, VkDevice device, VkImage image, VkDeviceMemory memory, VkDeviceSize memoryOffset);
   void APICALL (*vkGetBufferMemoryRequirements)(struct VulkanIFace *Self, VkDevice device, VkBuffer buffer, VkMemoryRequirements* pMemoryRequirements);
   void APICALL (*vkGetImageMemoryRequirements)(struct VulkanIFace *Self, VkDevice device, VkImage image, VkMemoryRequirements* pMemoryRequirements);
   void APICALL (*vkGetImageSparseMemoryRequirements)(struct VulkanIFace *Self, VkDevice device, VkImage image, uint32_t* pSparseMemoryRequirementCount, VkSparseImageMemoryRequirements* pSparseMemoryRequirements);
   void APICALL (*vkGetPhysicalDeviceSparseImageFormatProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkFormat format, VkImageType type, VkSampleCountFlagBits samples, VkImageUsageFlags usage, VkImageTiling tiling, uint32_t* pPropertyCount, VkSparseImageFormatProperties* pProperties);
   VkResult APICALL (*vkQueueBindSparse)(struct VulkanIFace *Self, VkQueue queue, uint32_t bindInfoCount, const VkBindSparseInfo* pBindInfo, VkFence fence);
   VkResult APICALL (*vkCreateFence)(struct VulkanIFace *Self, VkDevice device, const VkFenceCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkFence* pFence);
   void APICALL (*vkDestroyFence)(struct VulkanIFace *Self, VkDevice device, VkFence fence, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkResetFences)(struct VulkanIFace *Self, VkDevice device, uint32_t fenceCount, const VkFence* pFences);
   VkResult APICALL (*vkGetFenceStatus)(struct VulkanIFace *Self, VkDevice device, VkFence fence);
   VkResult APICALL (*vkWaitForFences)(struct VulkanIFace *Self, VkDevice device, uint32_t fenceCount, const VkFence* pFences, VkBool32 waitAll, uint64_t timeout);
   VkResult APICALL (*vkCreateSemaphore)(struct VulkanIFace *Self, VkDevice device, const VkSemaphoreCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkSemaphore* pSemaphore);
   void APICALL (*vkDestroySemaphore)(struct VulkanIFace *Self, VkDevice device, VkSemaphore semaphore, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateEvent)(struct VulkanIFace *Self, VkDevice device, const VkEventCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkEvent* pEvent);
   void APICALL (*vkDestroyEvent)(struct VulkanIFace *Self, VkDevice device, VkEvent event, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkGetEventStatus)(struct VulkanIFace *Self, VkDevice device, VkEvent event);
   VkResult APICALL (*vkSetEvent)(struct VulkanIFace *Self, VkDevice device, VkEvent event);
   VkResult APICALL (*vkResetEvent)(struct VulkanIFace *Self, VkDevice device, VkEvent event);
   VkResult APICALL (*vkCreateQueryPool)(struct VulkanIFace *Self, VkDevice device, const VkQueryPoolCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkQueryPool* pQueryPool);
   void APICALL (*vkDestroyQueryPool)(struct VulkanIFace *Self, VkDevice device, VkQueryPool queryPool, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkGetQueryPoolResults)(struct VulkanIFace *Self, VkDevice device, VkQueryPool queryPool, uint32_t firstQuery, uint32_t queryCount, size_t dataSize, void* pData, VkDeviceSize stride, VkQueryResultFlags flags);
   VkResult APICALL (*vkCreateBuffer)(struct VulkanIFace *Self, VkDevice device, const VkBufferCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkBuffer* pBuffer);
   void APICALL (*vkDestroyBuffer)(struct VulkanIFace *Self, VkDevice device, VkBuffer buffer, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateBufferView)(struct VulkanIFace *Self, VkDevice device, const VkBufferViewCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkBufferView* pView);
   void APICALL (*vkDestroyBufferView)(struct VulkanIFace *Self, VkDevice device, VkBufferView bufferView, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateImage)(struct VulkanIFace *Self, VkDevice device, const VkImageCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkImage* pImage);
   void APICALL (*vkDestroyImage)(struct VulkanIFace *Self, VkDevice device, VkImage image, const VkAllocationCallbacks* pAllocator);
   void APICALL (*vkGetImageSubresourceLayout)(struct VulkanIFace *Self, VkDevice device, VkImage image, const VkImageSubresource* pSubresource, VkSubresourceLayout* pLayout);
   VkResult APICALL (*vkCreateImageView)(struct VulkanIFace *Self, VkDevice device, const VkImageViewCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkImageView* pView);
   void APICALL (*vkDestroyImageView)(struct VulkanIFace *Self, VkDevice device, VkImageView imageView, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateShaderModule)(struct VulkanIFace *Self, VkDevice device, const VkShaderModuleCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkShaderModule* pShaderModule);
   void APICALL (*vkDestroyShaderModule)(struct VulkanIFace *Self, VkDevice device, VkShaderModule shaderModule, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreatePipelineCache)(struct VulkanIFace *Self, VkDevice device, const VkPipelineCacheCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkPipelineCache* pPipelineCache);
   void APICALL (*vkDestroyPipelineCache)(struct VulkanIFace *Self, VkDevice device, VkPipelineCache pipelineCache, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkGetPipelineCacheData)(struct VulkanIFace *Self, VkDevice device, VkPipelineCache pipelineCache, size_t* pDataSize, void* pData);
   VkResult APICALL (*vkMergePipelineCaches)(struct VulkanIFace *Self, VkDevice device, VkPipelineCache dstCache, uint32_t srcCacheCount, const VkPipelineCache* pSrcCaches);
   VkResult APICALL (*vkCreateGraphicsPipelines)(struct VulkanIFace *Self, VkDevice device, VkPipelineCache pipelineCache, uint32_t createInfoCount, const VkGraphicsPipelineCreateInfo* pCreateInfos, const VkAllocationCallbacks* pAllocator, VkPipeline* pPipelines);
   VkResult APICALL (*vkCreateComputePipelines)(struct VulkanIFace *Self, VkDevice device, VkPipelineCache pipelineCache, uint32_t createInfoCount, const VkComputePipelineCreateInfo* pCreateInfos, const VkAllocationCallbacks* pAllocator, VkPipeline* pPipelines);
   void APICALL (*vkDestroyPipeline)(struct VulkanIFace *Self, VkDevice device, VkPipeline pipeline, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreatePipelineLayout)(struct VulkanIFace *Self, VkDevice device, const VkPipelineLayoutCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkPipelineLayout* pPipelineLayout);
   void APICALL (*vkDestroyPipelineLayout)(struct VulkanIFace *Self, VkDevice device, VkPipelineLayout pipelineLayout, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateSampler)(struct VulkanIFace *Self, VkDevice device, const VkSamplerCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkSampler* pSampler);
   void APICALL (*vkDestroySampler)(struct VulkanIFace *Self, VkDevice device, VkSampler sampler, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateDescriptorSetLayout)(struct VulkanIFace *Self, VkDevice device, const VkDescriptorSetLayoutCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkDescriptorSetLayout* pSetLayout);
   void APICALL (*vkDestroyDescriptorSetLayout)(struct VulkanIFace *Self, VkDevice device, VkDescriptorSetLayout descriptorSetLayout, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateDescriptorPool)(struct VulkanIFace *Self, VkDevice device, const VkDescriptorPoolCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkDescriptorPool* pDescriptorPool);
   void APICALL (*vkDestroyDescriptorPool)(struct VulkanIFace *Self, VkDevice device, VkDescriptorPool descriptorPool, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkResetDescriptorPool)(struct VulkanIFace *Self, VkDevice device, VkDescriptorPool descriptorPool, VkDescriptorPoolResetFlags flags);
   VkResult APICALL (*vkAllocateDescriptorSets)(struct VulkanIFace *Self, VkDevice device, const VkDescriptorSetAllocateInfo* pAllocateInfo, VkDescriptorSet* pDescriptorSets);
   VkResult APICALL (*vkFreeDescriptorSets)(struct VulkanIFace *Self, VkDevice device, VkDescriptorPool descriptorPool, uint32_t descriptorSetCount, const VkDescriptorSet* pDescriptorSets);
   void APICALL (*vkUpdateDescriptorSets)(struct VulkanIFace *Self, VkDevice device, uint32_t descriptorWriteCount, const VkWriteDescriptorSet* pDescriptorWrites, uint32_t descriptorCopyCount, const VkCopyDescriptorSet* pDescriptorCopies);
   VkResult APICALL (*vkCreateFramebuffer)(struct VulkanIFace *Self, VkDevice device, const VkFramebufferCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkFramebuffer* pFramebuffer);
   void APICALL (*vkDestroyFramebuffer)(struct VulkanIFace *Self, VkDevice device, VkFramebuffer framebuffer, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkCreateRenderPass)(struct VulkanIFace *Self, VkDevice device, const VkRenderPassCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkRenderPass* pRenderPass);
   void APICALL (*vkDestroyRenderPass)(struct VulkanIFace *Self, VkDevice device, VkRenderPass renderPass, const VkAllocationCallbacks* pAllocator);
   void APICALL (*vkGetRenderAreaGranularity)(struct VulkanIFace *Self, VkDevice device, VkRenderPass renderPass, VkExtent2D* pGranularity);
   VkResult APICALL (*vkCreateCommandPool)(struct VulkanIFace *Self, VkDevice device, const VkCommandPoolCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkCommandPool* pCommandPool);
   void APICALL (*vkDestroyCommandPool)(struct VulkanIFace *Self, VkDevice device, VkCommandPool commandPool, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkResetCommandPool)(struct VulkanIFace *Self, VkDevice device, VkCommandPool commandPool, VkCommandPoolResetFlags flags);
   VkResult APICALL (*vkAllocateCommandBuffers)(struct VulkanIFace *Self, VkDevice device, const VkCommandBufferAllocateInfo* pAllocateInfo, VkCommandBuffer* pCommandBuffers);
   void APICALL (*vkFreeCommandBuffers)(struct VulkanIFace *Self, VkDevice device, VkCommandPool commandPool, uint32_t commandBufferCount, const VkCommandBuffer* pCommandBuffers);
   VkResult APICALL (*vkBeginCommandBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkCommandBufferBeginInfo* pBeginInfo);
   VkResult APICALL (*vkEndCommandBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer);
   VkResult APICALL (*vkResetCommandBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkCommandBufferResetFlags flags);
   void APICALL (*vkCmdBindPipeline)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPipelineBindPoint pipelineBindPoint, VkPipeline pipeline);
   void APICALL (*vkCmdSetViewport)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t firstViewport, uint32_t viewportCount, const VkViewport* pViewports);
   void APICALL (*vkCmdSetScissor)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t firstScissor, uint32_t scissorCount, const VkRect2D* pScissors);
   void APICALL (*vkCmdSetLineWidth)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, float lineWidth);
   void APICALL (*vkCmdSetDepthBias)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, float depthBiasConstantFactor, float depthBiasClamp, float depthBiasSlopeFactor);
   void APICALL (*vkCmdSetBlendConstants)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const float blendConstants[4]);
   void APICALL (*vkCmdSetDepthBounds)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, float minDepthBounds, float maxDepthBounds);
   void APICALL (*vkCmdSetStencilCompareMask)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkStencilFaceFlags faceMask, uint32_t compareMask);
   void APICALL (*vkCmdSetStencilWriteMask)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkStencilFaceFlags faceMask, uint32_t writeMask);
   void APICALL (*vkCmdSetStencilReference)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkStencilFaceFlags faceMask, uint32_t reference);
   void APICALL (*vkCmdBindDescriptorSets)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPipelineBindPoint pipelineBindPoint, VkPipelineLayout layout, uint32_t firstSet, uint32_t descriptorSetCount, const VkDescriptorSet* pDescriptorSets, uint32_t dynamicOffsetCount, const uint32_t* pDynamicOffsets);
   void APICALL (*vkCmdBindIndexBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer buffer, VkDeviceSize offset, VkIndexType indexType);
   void APICALL (*vkCmdBindVertexBuffers)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t firstBinding, uint32_t bindingCount, const VkBuffer* pBuffers, const VkDeviceSize* pOffsets);
   void APICALL (*vkCmdDraw)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t vertexCount, uint32_t instanceCount, uint32_t firstVertex, uint32_t firstInstance);
   void APICALL (*vkCmdDrawIndexed)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t indexCount, uint32_t instanceCount, uint32_t firstIndex, int32_t vertexOffset, uint32_t firstInstance);
   void APICALL (*vkCmdDrawIndirect)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer buffer, VkDeviceSize offset, uint32_t drawCount, uint32_t stride);
   void APICALL (*vkCmdDrawIndexedIndirect)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer buffer, VkDeviceSize offset, uint32_t drawCount, uint32_t stride);
   void APICALL (*vkCmdDispatch)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t groupCountX, uint32_t groupCountY, uint32_t groupCountZ);
   void APICALL (*vkCmdDispatchIndirect)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer buffer, VkDeviceSize offset);
   void APICALL (*vkCmdCopyBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer srcBuffer, VkBuffer dstBuffer, uint32_t regionCount, const VkBufferCopy* pRegions);
   void APICALL (*vkCmdCopyImage)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkImage srcImage, VkImageLayout srcImageLayout, VkImage dstImage, VkImageLayout dstImageLayout, uint32_t regionCount, const VkImageCopy* pRegions);
   void APICALL (*vkCmdBlitImage)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkImage srcImage, VkImageLayout srcImageLayout, VkImage dstImage, VkImageLayout dstImageLayout, uint32_t regionCount, const VkImageBlit* pRegions, VkFilter filter);
   void APICALL (*vkCmdCopyBufferToImage)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer srcBuffer, VkImage dstImage, VkImageLayout dstImageLayout, uint32_t regionCount, const VkBufferImageCopy* pRegions);
   void APICALL (*vkCmdCopyImageToBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkImage srcImage, VkImageLayout srcImageLayout, VkBuffer dstBuffer, uint32_t regionCount, const VkBufferImageCopy* pRegions);
   void APICALL (*vkCmdUpdateBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer dstBuffer, VkDeviceSize dstOffset, VkDeviceSize dataSize, const void* pData);
   void APICALL (*vkCmdFillBuffer)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer dstBuffer, VkDeviceSize dstOffset, VkDeviceSize size, uint32_t data);
   void APICALL (*vkCmdClearColorImage)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkImage image, VkImageLayout imageLayout, const VkClearColorValue* pColor, uint32_t rangeCount, const VkImageSubresourceRange* pRanges);
   void APICALL (*vkCmdClearDepthStencilImage)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkImage image, VkImageLayout imageLayout, const VkClearDepthStencilValue* pDepthStencil, uint32_t rangeCount, const VkImageSubresourceRange* pRanges);
   void APICALL (*vkCmdClearAttachments)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t attachmentCount, const VkClearAttachment* pAttachments, uint32_t rectCount, const VkClearRect* pRects);
   void APICALL (*vkCmdResolveImage)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkImage srcImage, VkImageLayout srcImageLayout, VkImage dstImage, VkImageLayout dstImageLayout, uint32_t regionCount, const VkImageResolve* pRegions);
   void APICALL (*vkCmdSetEvent)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkEvent event, VkPipelineStageFlags stageMask);
   void APICALL (*vkCmdResetEvent)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkEvent event, VkPipelineStageFlags stageMask);
   void APICALL (*vkCmdWaitEvents)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t eventCount, const VkEvent* pEvents, VkPipelineStageFlags srcStageMask, VkPipelineStageFlags dstStageMask, uint32_t memoryBarrierCount, const VkMemoryBarrier* pMemoryBarriers, uint32_t bufferMemoryBarrierCount, const VkBufferMemoryBarrier* pBufferMemoryBarriers, uint32_t imageMemoryBarrierCount, const VkImageMemoryBarrier* pImageMemoryBarriers);
   void APICALL (*vkCmdPipelineBarrier)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPipelineStageFlags srcStageMask, VkPipelineStageFlags dstStageMask, VkDependencyFlags dependencyFlags, uint32_t memoryBarrierCount, const VkMemoryBarrier* pMemoryBarriers, uint32_t bufferMemoryBarrierCount, const VkBufferMemoryBarrier* pBufferMemoryBarriers, uint32_t imageMemoryBarrierCount, const VkImageMemoryBarrier* pImageMemoryBarriers);
   void APICALL (*vkCmdBeginQuery)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkQueryPool queryPool, uint32_t query, VkQueryControlFlags flags);
   void APICALL (*vkCmdEndQuery)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkQueryPool queryPool, uint32_t query);
   void APICALL (*vkCmdResetQueryPool)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkQueryPool queryPool, uint32_t firstQuery, uint32_t queryCount);
   void APICALL (*vkCmdWriteTimestamp)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPipelineStageFlagBits pipelineStage, VkQueryPool queryPool, uint32_t query);
   void APICALL (*vkCmdCopyQueryPoolResults)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkQueryPool queryPool, uint32_t firstQuery, uint32_t queryCount, VkBuffer dstBuffer, VkDeviceSize dstOffset, VkDeviceSize stride, VkQueryResultFlags flags);
   void APICALL (*vkCmdPushConstants)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPipelineLayout layout, VkShaderStageFlags stageFlags, uint32_t offset, uint32_t size, const void* pValues);
   void APICALL (*vkCmdBeginRenderPass)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkRenderPassBeginInfo* pRenderPassBegin, VkSubpassContents contents);
   void APICALL (*vkCmdNextSubpass)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkSubpassContents contents);
   void APICALL (*vkCmdEndRenderPass)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer);
   void APICALL (*vkCmdExecuteCommands)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t commandBufferCount, const VkCommandBuffer* pCommandBuffers);
   void APICALL (*vkDestroySurfaceKHR)(struct VulkanIFace *Self, VkInstance instance, VkSurfaceKHR surface, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkGetPhysicalDeviceSurfaceSupportKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t queueFamilyIndex, VkSurfaceKHR surface, VkBool32* pSupported);
   VkResult APICALL (*vkGetPhysicalDeviceSurfaceCapabilitiesKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkSurfaceKHR surface, VkSurfaceCapabilitiesKHR* pSurfaceCapabilities);
   VkResult APICALL (*vkGetPhysicalDeviceSurfaceFormatsKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkSurfaceKHR surface, uint32_t* pSurfaceFormatCount, VkSurfaceFormatKHR* pSurfaceFormats);
   VkResult APICALL (*vkGetPhysicalDeviceSurfaceCapabilities2KHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkPhysicalDeviceSurfaceInfo2KHR* pSurfaceInfo, VkSurfaceCapabilities2KHR* pSurfaceCapabilities);
   VkResult APICALL (*vkGetPhysicalDeviceSurfaceFormats2KHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkPhysicalDeviceSurfaceInfo2KHR* pSurfaceInfo, uint32_t* pSurfaceFormatCount, VkSurfaceFormat2KHR* pSurfaceFormats);
   VkResult APICALL (*vkGetPhysicalDeviceSurfacePresentModesKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkSurfaceKHR surface, uint32_t* pPresentModeCount, VkPresentModeKHR* pPresentModes);
   VkResult APICALL (*vkCreateSwapchainKHR)(struct VulkanIFace *Self, VkDevice device, const VkSwapchainCreateInfoKHR* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkSwapchainKHR* pSwapchain);
   void APICALL (*vkDestroySwapchainKHR)(struct VulkanIFace *Self, VkDevice device, VkSwapchainKHR swapchain, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkGetSwapchainImagesKHR)(struct VulkanIFace *Self, VkDevice device, VkSwapchainKHR swapchain, uint32_t* pSwapchainImageCount, VkImage* pSwapchainImages);
   VkResult APICALL (*vkAcquireNextImageKHR)(struct VulkanIFace *Self, VkDevice device, VkSwapchainKHR swapchain, uint64_t timeout, VkSemaphore semaphore, VkFence fence, uint32_t* pImageIndex);
   VkResult APICALL (*vkQueuePresentKHR)(struct VulkanIFace *Self, VkQueue queue, const VkPresentInfoKHR* pPresentInfo);
   VkResult APICALL (*vkGetPhysicalDeviceDisplayPropertiesKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pPropertyCount, VkDisplayPropertiesKHR* pProperties);
   VkResult APICALL (*vkGetPhysicalDeviceDisplayPlanePropertiesKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pPropertyCount, VkDisplayPlanePropertiesKHR* pProperties);
   VkResult APICALL (*vkGetDisplayPlaneSupportedDisplaysKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t planeIndex, uint32_t* pDisplayCount, VkDisplayKHR* pDisplays);
   VkResult APICALL (*vkGetDisplayModePropertiesKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkDisplayKHR display, uint32_t* pPropertyCount, VkDisplayModePropertiesKHR* pProperties);
   VkResult APICALL (*vkCreateDisplayModeKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkDisplayKHR display, const VkDisplayModeCreateInfoKHR* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkDisplayModeKHR* pMode);
   VkResult APICALL (*vkGetDisplayPlaneCapabilitiesKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkDisplayModeKHR mode, uint32_t planeIndex, VkDisplayPlaneCapabilitiesKHR* pCapabilities);
   VkResult APICALL (*vkCreateDisplayPlaneSurfaceKHR)(struct VulkanIFace *Self, VkInstance instance, const VkDisplaySurfaceCreateInfoKHR* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkSurfaceKHR* pSurface);
   VkResult APICALL (*vkCreateSharedSwapchainsKHR)(struct VulkanIFace *Self, VkDevice device, uint32_t swapchainCount, const VkSwapchainCreateInfoKHR* pCreateInfos, const VkAllocationCallbacks* pAllocator, VkSwapchainKHR* pSwapchains);
   VkResult APICALL (*vkCreateHeadlessSurfaceEXT)(struct VulkanIFace *Self, VkInstance instance, const VkHeadlessSurfaceCreateInfoEXT* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkSurfaceKHR* pSurface);
   VkResult APICALL (*vkEnumerateInstanceVersion)(struct VulkanIFace *Self, uint32_t* pApiVersion);
   VkResult APICALL (*vkEnumeratePhysicalDeviceGroups)(struct VulkanIFace *Self, VkInstance instance, uint32_t* pPhysicalDeviceGroupCount, VkPhysicalDeviceGroupProperties* pPhysicalDeviceGroupProperties);
   void APICALL (*vkGetPhysicalDeviceFeatures2)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkPhysicalDeviceFeatures2* pFeatures);
   void APICALL (*vkGetPhysicalDeviceProperties2)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkPhysicalDeviceProperties2* pProperties);
   void APICALL (*vkGetPhysicalDeviceFormatProperties2)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkFormat format, VkFormatProperties2* pFormatProperties);
   void APICALL (*vkGetPhysicalDeviceQueueFamilyProperties2)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pQueueFamilyPropertyCount, VkQueueFamilyProperties2* pQueueFamilyProperties);
   void APICALL (*vkGetPhysicalDeviceMemoryProperties2)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkPhysicalDeviceMemoryProperties2* pMemoryProperties);
   void APICALL (*vkGetPhysicalDeviceSparseImageFormatProperties2)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkPhysicalDeviceSparseImageFormatInfo2* pFormatInfo, uint32_t* pPropertyCount, VkSparseImageFormatProperties2* pProperties);
   void APICALL (*vkGetPhysicalDeviceExternalBufferProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkPhysicalDeviceExternalBufferInfo* pExternalBufferInfo, VkExternalBufferProperties* pExternalBufferProperties);
   void APICALL (*vkGetPhysicalDeviceExternalSemaphoreProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkPhysicalDeviceExternalSemaphoreInfo* pExternalSemaphoreInfo, VkExternalSemaphoreProperties* pExternalSemaphoreProperties);
   void APICALL (*vkGetPhysicalDeviceExternalFenceProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkPhysicalDeviceExternalFenceInfo* pExternalFenceInfo, VkExternalFenceProperties* pExternalFenceProperties);
   VkResult APICALL (*vkBindBufferMemory2)(struct VulkanIFace *Self, VkDevice device, uint32_t bindInfoCount, const VkBindBufferMemoryInfo* pBindInfos);
   VkResult APICALL (*vkBindImageMemory2)(struct VulkanIFace *Self, VkDevice device, uint32_t bindInfoCount, const VkBindImageMemoryInfo* pBindInfos);
   void APICALL (*vkGetDeviceGroupPeerMemoryFeatures)(struct VulkanIFace *Self, VkDevice device, uint32_t heapIndex, uint32_t localDeviceIndex, uint32_t remoteDeviceIndex, VkPeerMemoryFeatureFlags* pPeerMemoryFeatures);
   void APICALL (*vkCmdSetDeviceMask)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t deviceMask);
   void APICALL (*vkCmdDispatchBase)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t baseGroupX, uint32_t baseGroupY, uint32_t baseGroupZ, uint32_t groupCountX, uint32_t groupCountY, uint32_t groupCountZ);
   void APICALL (*vkGetImageMemoryRequirements2)(struct VulkanIFace *Self, VkDevice device, const VkImageMemoryRequirementsInfo2* pInfo, VkMemoryRequirements2* pMemoryRequirements);
   void APICALL (*vkGetBufferMemoryRequirements2)(struct VulkanIFace *Self, VkDevice device, const VkBufferMemoryRequirementsInfo2* pInfo, VkMemoryRequirements2* pMemoryRequirements);
   void APICALL (*vkTrimCommandPool)(struct VulkanIFace *Self, VkDevice device, VkCommandPool commandPool, VkCommandPoolTrimFlags flags);
   void APICALL (*vkGetDeviceQueue2)(struct VulkanIFace *Self, VkDevice device, const VkDeviceQueueInfo2* pQueueInfo, VkQueue* pQueue);
   VkResult APICALL (*vkCreateSamplerYcbcrConversion)(struct VulkanIFace *Self, VkDevice device, const VkSamplerYcbcrConversionCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkSamplerYcbcrConversion* pYcbcrConversion);
   void APICALL (*vkDestroySamplerYcbcrConversion)(struct VulkanIFace *Self, VkDevice device, VkSamplerYcbcrConversion ycbcrConversion, const VkAllocationCallbacks* pAllocator);
   void APICALL (*vkGetDescriptorSetLayoutSupport)(struct VulkanIFace *Self, VkDevice device, const VkDescriptorSetLayoutCreateInfo* pCreateInfo, VkDescriptorSetLayoutSupport* pSupport);
   VkResult APICALL (*vkGetDeviceGroupPresentCapabilitiesKHR)(struct VulkanIFace *Self, VkDevice device, VkDeviceGroupPresentCapabilitiesKHR* pDeviceGroupPresentCapabilities);
   VkResult APICALL (*vkGetDeviceGroupSurfacePresentModesKHR)(struct VulkanIFace *Self, VkDevice device, VkSurfaceKHR surface, VkDeviceGroupPresentModeFlagsKHR* pModes);
   VkResult APICALL (*vkGetPhysicalDevicePresentRectanglesKHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkSurfaceKHR surface, uint32_t* pRectCount, VkRect2D* pRects);
   VkResult APICALL (*vkAcquireNextImage2KHR)(struct VulkanIFace *Self, VkDevice device, const VkAcquireNextImageInfoKHR* pAcquireInfo, uint32_t* pImageIndex);
   VkResult APICALL (*vkCreateDescriptorUpdateTemplate)(struct VulkanIFace *Self, VkDevice device, const VkDescriptorUpdateTemplateCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkDescriptorUpdateTemplate* pDescriptorUpdateTemplate);
   void APICALL (*vkDestroyDescriptorUpdateTemplate)(struct VulkanIFace *Self, VkDevice device, VkDescriptorUpdateTemplate descriptorUpdateTemplate, const VkAllocationCallbacks* pAllocator);
   void APICALL (*vkUpdateDescriptorSetWithTemplate)(struct VulkanIFace *Self, VkDevice device, VkDescriptorSet descriptorSet, VkDescriptorUpdateTemplate descriptorUpdateTemplate, const void* pData);
   VkResult APICALL (*vkGetPhysicalDeviceDisplayProperties2KHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pPropertyCount, VkDisplayProperties2KHR* pProperties);
   VkResult APICALL (*vkGetPhysicalDeviceDisplayPlaneProperties2KHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pPropertyCount, VkDisplayPlaneProperties2KHR* pProperties);
   VkResult APICALL (*vkGetDisplayModeProperties2KHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, VkDisplayKHR display, uint32_t* pPropertyCount, VkDisplayModeProperties2KHR* pProperties);
   VkResult APICALL (*vkGetDisplayPlaneCapabilities2KHR)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkDisplayPlaneInfo2KHR* pDisplayPlaneInfo, VkDisplayPlaneCapabilities2KHR* pCapabilities);
   void APICALL (*vkGetImageSparseMemoryRequirements2)(struct VulkanIFace *Self, VkDevice device, const VkImageSparseMemoryRequirementsInfo2* pInfo, uint32_t* pSparseMemoryRequirementCount, VkSparseImageMemoryRequirements2* pSparseMemoryRequirements);
   VkResult APICALL (*vkGetPhysicalDeviceImageFormatProperties2)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, const VkPhysicalDeviceImageFormatInfo2* pImageFormatInfo, VkImageFormatProperties2* pImageFormatProperties);
   VkResult APICALL (*vkCreateRenderPass2)(struct VulkanIFace *Self, VkDevice device, const VkRenderPassCreateInfo2* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkRenderPass* pRenderPass);
   void APICALL (*vkCmdBeginRenderPass2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkRenderPassBeginInfo* pRenderPassBegin, const VkSubpassBeginInfo* pSubpassBeginInfo);
   void APICALL (*vkCmdNextSubpass2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkSubpassBeginInfo* pSubpassBeginInfo, const VkSubpassEndInfo* pSubpassEndInfo);
   void APICALL (*vkCmdEndRenderPass2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkSubpassEndInfo* pSubpassEndInfo);
   void APICALL (*vkCmdDrawIndirectCount)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer buffer, VkDeviceSize offset, VkBuffer countBuffer, VkDeviceSize countBufferOffset, uint32_t maxDrawCount, uint32_t stride);
   void APICALL (*vkCmdDrawIndexedIndirectCount)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer buffer, VkDeviceSize offset, VkBuffer countBuffer, VkDeviceSize countBufferOffset, uint32_t maxDrawCount, uint32_t stride);
   VkResult APICALL (*vkGetSemaphoreCounterValue)(struct VulkanIFace *Self, VkDevice device, VkSemaphore semaphore, uint64_t* pValue);
   VkResult APICALL (*vkWaitSemaphores)(struct VulkanIFace *Self, VkDevice device, const VkSemaphoreWaitInfo* pWaitInfo, uint64_t timeout);
   VkResult APICALL (*vkSignalSemaphore)(struct VulkanIFace *Self, VkDevice device, const VkSemaphoreSignalInfo* pSignalInfo);
   VkDeviceAddress APICALL (*vkGetBufferDeviceAddress)(struct VulkanIFace *Self, VkDevice device, const VkBufferDeviceAddressInfo* pInfo);
   uint64_t APICALL (*vkGetBufferOpaqueCaptureAddress)(struct VulkanIFace *Self, VkDevice device, const VkBufferDeviceAddressInfo* pInfo);
   uint64_t APICALL (*vkGetDeviceMemoryOpaqueCaptureAddress)(struct VulkanIFace *Self, VkDevice device, const VkDeviceMemoryOpaqueCaptureAddressInfo* pInfo);
   void APICALL (*vkResetQueryPool)(struct VulkanIFace *Self, VkDevice device, VkQueryPool queryPool, uint32_t firstQuery, uint32_t queryCount);
   VkResult APICALL (*vkGetPhysicalDeviceToolProperties)(struct VulkanIFace *Self, VkPhysicalDevice physicalDevice, uint32_t* pToolCount, VkPhysicalDeviceToolProperties* pToolProperties);
   VkResult APICALL (*vkCreatePrivateDataSlot)(struct VulkanIFace *Self, VkDevice device, const VkPrivateDataSlotCreateInfo* pCreateInfo, const VkAllocationCallbacks* pAllocator, VkPrivateDataSlot* pPrivateDataSlot);
   void APICALL (*vkDestroyPrivateDataSlot)(struct VulkanIFace *Self, VkDevice device, VkPrivateDataSlot privateDataSlot, const VkAllocationCallbacks* pAllocator);
   VkResult APICALL (*vkSetPrivateData)(struct VulkanIFace *Self, VkDevice device, VkObjectType objectType, uint64_t objectHandle, VkPrivateDataSlot privateDataSlot, uint64_t data);
   void APICALL (*vkGetPrivateData)(struct VulkanIFace *Self, VkDevice device, VkObjectType objectType, uint64_t objectHandle, VkPrivateDataSlot privateDataSlot, uint64_t* pData);
   void APICALL (*vkCmdSetEvent2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkEvent event, const VkDependencyInfo* pDependencyInfo);
   void APICALL (*vkCmdResetEvent2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkEvent event, VkPipelineStageFlags2 stageMask);
   void APICALL (*vkCmdWaitEvents2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t eventCount, const VkEvent* pEvents, const VkDependencyInfo* pDependencyInfos);
   void APICALL (*vkCmdPipelineBarrier2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkDependencyInfo* pDependencyInfo);
   void APICALL (*vkCmdWriteTimestamp2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPipelineStageFlags2 stage, VkQueryPool queryPool, uint32_t query);
   VkResult APICALL (*vkQueueSubmit2)(struct VulkanIFace *Self, VkQueue queue, uint32_t submitCount, const VkSubmitInfo2* pSubmits, VkFence fence);
   void APICALL (*vkCmdCopyBuffer2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkCopyBufferInfo2* pCopyBufferInfo);
   void APICALL (*vkCmdCopyImage2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkCopyImageInfo2* pCopyImageInfo);
   void APICALL (*vkCmdCopyBufferToImage2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkCopyBufferToImageInfo2* pCopyBufferToImageInfo);
   void APICALL (*vkCmdCopyImageToBuffer2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkCopyImageToBufferInfo2* pCopyImageToBufferInfo);
   void APICALL (*vkCmdBlitImage2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkBlitImageInfo2* pBlitImageInfo);
   void APICALL (*vkCmdResolveImage2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkResolveImageInfo2* pResolveImageInfo);
   void APICALL (*vkCmdBeginRendering)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkRenderingInfo* pRenderingInfo);
   void APICALL (*vkCmdEndRendering)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer);
   void APICALL (*vkCmdSetCullMode)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkCullModeFlags cullMode);
   void APICALL (*vkCmdSetFrontFace)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkFrontFace frontFace);
   void APICALL (*vkCmdSetPrimitiveTopology)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPrimitiveTopology primitiveTopology);
   void APICALL (*vkCmdSetViewportWithCount)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t viewportCount, const VkViewport* pViewports);
   void APICALL (*vkCmdSetScissorWithCount)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t scissorCount, const VkRect2D* pScissors);
   void APICALL (*vkCmdBindVertexBuffers2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t firstBinding, uint32_t bindingCount, const VkBuffer* pBuffers, const VkDeviceSize* pOffsets, const VkDeviceSize* pSizes, const VkDeviceSize* pStrides);
   void APICALL (*vkCmdSetDepthTestEnable)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBool32 depthTestEnable);
   void APICALL (*vkCmdSetDepthWriteEnable)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBool32 depthWriteEnable);
   void APICALL (*vkCmdSetDepthCompareOp)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkCompareOp depthCompareOp);
   void APICALL (*vkCmdSetDepthBoundsTestEnable)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBool32 depthBoundsTestEnable);
   void APICALL (*vkCmdSetStencilTestEnable)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBool32 stencilTestEnable);
   void APICALL (*vkCmdSetStencilOp)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkStencilFaceFlags faceMask, VkStencilOp failOp, VkStencilOp passOp, VkStencilOp depthFailOp, VkCompareOp compareOp);
   void APICALL (*vkCmdSetRasterizerDiscardEnable)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBool32 rasterizerDiscardEnable);
   void APICALL (*vkCmdSetDepthBiasEnable)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBool32 depthBiasEnable);
   void APICALL (*vkCmdSetPrimitiveRestartEnable)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBool32 primitiveRestartEnable);
   void APICALL (*vkGetDeviceBufferMemoryRequirements)(struct VulkanIFace *Self, VkDevice device, const VkDeviceBufferMemoryRequirements* pInfo, VkMemoryRequirements2* pMemoryRequirements);
   void APICALL (*vkGetDeviceImageMemoryRequirements)(struct VulkanIFace *Self, VkDevice device, const VkDeviceImageMemoryRequirements* pInfo, VkMemoryRequirements2* pMemoryRequirements);
   void APICALL (*vkGetDeviceImageSparseMemoryRequirements)(struct VulkanIFace *Self, VkDevice device, const VkDeviceImageMemoryRequirements* pInfo, uint32_t* pSparseMemoryRequirementCount, VkSparseImageMemoryRequirements2* pSparseMemoryRequirements);
   void APICALL (*vkCmdSetLineStipple)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, uint32_t lineStippleFactor, uint16_t lineStipplePattern);
   VkResult APICALL (*vkMapMemory2)(struct VulkanIFace *Self, VkDevice device, const VkMemoryMapInfo* pMemoryMapInfo, void** ppData);
   VkResult APICALL (*vkUnmapMemory2)(struct VulkanIFace *Self, VkDevice device, const VkMemoryUnmapInfo* pMemoryUnmapInfo);
   void APICALL (*vkCmdBindIndexBuffer2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkBuffer buffer, VkDeviceSize offset, VkDeviceSize size, VkIndexType indexType);
   void APICALL (*vkGetRenderingAreaGranularity)(struct VulkanIFace *Self, VkDevice device, const VkRenderingAreaInfo* pRenderingAreaInfo, VkExtent2D* pGranularity);
   void APICALL (*vkGetDeviceImageSubresourceLayout)(struct VulkanIFace *Self, VkDevice device, const VkDeviceImageSubresourceInfo* pInfo, VkSubresourceLayout2* pLayout);
   void APICALL (*vkGetImageSubresourceLayout2)(struct VulkanIFace *Self, VkDevice device, VkImage image, const VkImageSubresource2* pSubresource, VkSubresourceLayout2* pLayout);
   void APICALL (*vkCmdPushDescriptorSet)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkPipelineBindPoint pipelineBindPoint, VkPipelineLayout layout, uint32_t set, uint32_t descriptorWriteCount, const VkWriteDescriptorSet* pDescriptorWrites);
   void APICALL (*vkCmdPushDescriptorSetWithTemplate)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, VkDescriptorUpdateTemplate descriptorUpdateTemplate, VkPipelineLayout layout, uint32_t set, const void* pData);
   void APICALL (*vkCmdSetRenderingAttachmentLocations)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkRenderingAttachmentLocationInfo* pLocationInfo);
   void APICALL (*vkCmdSetRenderingInputAttachmentIndices)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkRenderingInputAttachmentIndexInfo* pInputAttachmentIndexInfo);
   void APICALL (*vkCmdBindDescriptorSets2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkBindDescriptorSetsInfo* pBindDescriptorSetsInfo);
   void APICALL (*vkCmdPushConstants2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkPushConstantsInfo* pPushConstantsInfo);
   void APICALL (*vkCmdPushDescriptorSet2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkPushDescriptorSetInfo* pPushDescriptorSetInfo);
   void APICALL (*vkCmdPushDescriptorSetWithTemplate2)(struct VulkanIFace *Self, VkCommandBuffer commandBuffer, const VkPushDescriptorSetWithTemplateInfo* pPushDescriptorSetWithTemplateInfo);
   VkResult APICALL (*vkCopyMemoryToImage)(struct VulkanIFace *Self, VkDevice device, const VkCopyMemoryToImageInfo* pCopyMemoryToImageInfo);
   VkResult APICALL (*vkCopyImageToMemory)(struct VulkanIFace *Self, VkDevice device, const VkCopyImageToMemoryInfo* pCopyImageToMemoryInfo);
   VkResult APICALL (*vkCopyImageToImage)(struct VulkanIFace *Self, VkDevice device, const VkCopyImageToImageInfo* pCopyImageToImageInfo);
   VkResult APICALL (*vkTransitionImageLayout)(struct VulkanIFace *Self, VkDevice device, uint32_t transitionCount, const VkHostImageLayoutTransitionInfo* pTransitions);
};

#ifdef __cplusplus
}
#ifdef __USE_AMIGAOS_NAMESPACE__
}
#endif
#endif

#endif /* VULKAN_INTERFACE_DEF_H */
