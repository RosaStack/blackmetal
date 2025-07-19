use crate::{MTLDevice, MTLDrawable, MTLRenderPassDescriptor};
use anyhow::{Result, anyhow};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::{rc::Retained, runtime::ProtocolObject};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_metal::{
    MTLCommandBuffer as MetalMTLCommandBuffer, MTLCommandEncoder as MetalMTLCommandEncoder,
    MTLCommandQueue as MetalMTLCommandQueue, MTLDevice as MetalMTLDevice,
    MTLRenderCommandEncoder as MetalMTLRenderCommandEncoder,
};
use std::sync::Arc;

pub struct MTLCommandQueue {
    device: Arc<MTLDevice>,

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_command_queue: Retained<ProtocolObject<dyn MetalMTLCommandQueue>>,
}

impl MTLCommandQueue {
    pub fn new(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_new(device);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support")
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
}

impl MTLCommandBuffer {
    pub fn new(queue: Arc<MTLCommandQueue>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_new(queue);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
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
