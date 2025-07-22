use crate::{MTLDevice, MTLDrawable, MTLRenderPassDescriptor};
use anyhow::{Result, anyhow};
use std::sync::Arc;

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk;

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::{rc::Retained, runtime::ProtocolObject};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_metal::{
    MTLCommandBuffer as MetalMTLCommandBuffer, MTLCommandEncoder as MetalMTLCommandEncoder,
    MTLCommandQueue as MetalMTLCommandQueue, MTLDevice as MetalMTLDevice,
    MTLRenderCommandEncoder as MetalMTLRenderCommandEncoder,
};

pub struct MTLCommandQueue {
    device: Arc<MTLDevice>,

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_command_queue: Retained<ProtocolObject<dyn MetalMTLCommandQueue>>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_command_queue: VulkanMTLCommandQueue,
}

impl MTLCommandQueue {
    pub fn new(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_new(device);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_new(device);
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_new(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let logical_device = device.vulkan_device().logical();
        let queue_families = device.vulkan_device().queue_families();

        let graphics_queue =
            unsafe { logical_device.get_device_queue(queue_families.graphics_queue, 0) };
        let present_queue =
            unsafe { logical_device.get_device_queue(queue_families.present_queue, 0) };

        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_families.graphics_queue);

        let command_pool = unsafe { logical_device.create_command_pool(&command_pool_info, None)? };

        Ok(Arc::new(Self {
            device,
            vulkan_command_queue: VulkanMTLCommandQueue {
                graphics_queue,
                present_queue,
                command_pool,
            },
        }))
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_new(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let metal_command_queue = device.metal_device().newCommandQueue();

        let metal_command_queue = match metal_command_queue {
            Some(c) => c,
            None => return Err(anyhow!("Command queue creation failed.")),
        };

        Ok(Arc::new(Self {
            device,
            metal_command_queue,
        }))
    }
}

pub struct MTLCommandBuffer {
    queue: Arc<MTLCommandQueue>,
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_command_buffer: Retained<ProtocolObject<dyn MetalMTLCommandBuffer>>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_command_buffer: vk::CommandBuffer,
}

impl MTLCommandBuffer {
    pub fn new(queue: Arc<MTLCommandQueue>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_new(queue);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_new(queue);
    }

    pub fn vulkan_new(queue: Arc<MTLCommandQueue>) -> Result<Arc<Self>> {
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_new(queue: Arc<MTLCommandQueue>) -> Result<Arc<Self>> {
        let metal_command_buffer = queue.metal_command_queue.commandBuffer();

        let metal_command_buffer = match metal_command_buffer {
            Some(b) => b,
            None => return Err(anyhow!("Command buffer creation failed.")),
        };

        Ok(Arc::new(Self {
            queue,
            metal_command_buffer,
        }))
    }

    pub fn present(&self, drawable: Arc<MTLDrawable>) {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        self.metal_present(drawable);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_present(&self, drawable: Arc<MTLDrawable>) {
        self.metal_command_buffer
            .presentDrawable(drawable.ca_metal_drawable().as_ref());
    }

    pub fn commit(&self) {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        self.metal_commit();

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support");
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_commit(&self) {
        self.metal_command_buffer.commit();
    }
}

pub struct MTLRenderCommandEncoder {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_render_command_encoder: Retained<ProtocolObject<dyn MetalMTLRenderCommandEncoder>>,
}

impl MTLRenderCommandEncoder {
    pub fn new(
        command_buffer: Arc<MTLCommandBuffer>,
        render_pass: MTLRenderPassDescriptor,
    ) -> Result<Self> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_new(command_buffer, render_pass);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_new(
        command_buffer: Arc<MTLCommandBuffer>,
        render_pass: MTLRenderPassDescriptor,
    ) -> Result<Self> {
        let metal_render_command_encoder = command_buffer
            .metal_command_buffer
            .renderCommandEncoderWithDescriptor(render_pass.to_metal().as_ref());

        let metal_render_command_encoder = match metal_render_command_encoder {
            Some(c) => c,
            None => return Err(anyhow!("Render Command Encoder creation failed.")),
        };

        Ok(Self {
            metal_render_command_encoder,
        })
    }

    pub fn end_encoding(&self) {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return self.metal_end_encoding();

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_end_encoding(&self) {
        self.metal_render_command_encoder.endEncoding();
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanMTLCommandQueue {
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    command_pool: vk::CommandPool,
}
