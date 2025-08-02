use crate::{
    MTLBeginRenderPassDescriptor, MTLDevice, MTLRenderPass, MTLRenderPassDescriptor, MTLTexture,
};
use anyhow::{Result, anyhow};
use crossbeam::queue::SegQueue;
use std::sync::Arc;

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk::Device;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use std::sync::RwLock;

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
        use crossbeam::queue::SegQueue;

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
                command_buffers: SegQueue::new(),
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
    schedule_handler_queue: SegQueue<MTLCommandBufferHandler>,

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

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_new(queue: Arc<MTLCommandQueue>) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            queue: queue.clone(),
            schedule_handler_queue: SegQueue::new(),
            vulkan_command_buffer: {
                if queue.vulkan_command_queue.command_buffers().is_empty() {
                    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
                        .command_pool(queue.vulkan_command_queue.command_pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1);

                    let buffers = unsafe {
                        queue
                            .device
                            .vulkan_device()
                            .logical()
                            .allocate_command_buffers(&command_buffer_allocate_info)?
                    };

                    queue
                        .vulkan_command_queue
                        .command_buffers()
                        .push(buffers[0]);
                }

                queue.vulkan_command_queue.command_buffers().pop().unwrap()
            },
        }))
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
            schedule_handler_queue: SegQueue::new(),
            metal_command_buffer,
        }))
    }

    pub fn present(&self, drawable: Arc<MTLTexture>) {
        self.add_scheduled_handler(MTLCommandBufferHandler::Present(drawable));
    }

    pub fn add_scheduled_handler(&self, handler: MTLCommandBufferHandler) {
        self.schedule_handler_queue.push(handler);
    }

    pub fn commit(&self) -> Result<()> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return self.metal_commit();

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return self.vulkan_commit();
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_commit(&self) -> Result<()> {
        while !self.schedule_handler_queue.is_empty() {
            use objc2_metal::MTLDrawable;

            let handle = self.schedule_handler_queue.pop().unwrap();

            match handle {
                MTLCommandBufferHandler::Present(d) => {
                    d.ca_metal_drawable().as_ref().unwrap().present()
                }
            }
        }

        self.metal_command_buffer.commit();

        Ok(())
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_commit(&self) -> Result<()> {
        let device = self.queue.device.vulkan_device().logical();

        while !self.schedule_handler_queue.is_empty() {
            let handle = self.schedule_handler_queue.pop().unwrap();

            match handle {
                MTLCommandBufferHandler::Present(d) => {
                    use std::sync::atomic::Ordering;

                    let sync_object = d.vulkan_sync_object().read().unwrap();
                    let sync_object = sync_object.as_ref().unwrap();

                    let wait_semaphores = [*sync_object.image_available_event().vulkan_semaphore()];
                    let signal_semaphores =
                        [*sync_object.render_finished_event().vulkan_semaphore()];

                    let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                    let command_buffers = [self.vulkan_command_buffer];

                    let submit_info = vk::SubmitInfo::default()
                        .wait_semaphores(&wait_semaphores)
                        .wait_dst_stage_mask(&wait_stages)
                        .command_buffers(&command_buffers)
                        .signal_semaphores(&signal_semaphores);

                    unsafe {
                        device.queue_submit(
                            self.queue.vulkan_command_queue.graphics_queue,
                            &[submit_info],
                            *sync_object.fence().vulkan_fence(),
                        )?
                    };

                    let (swapchain_instance, swapchain_khr) =
                        d.vulkan_swapchain().as_ref().unwrap();

                    let swapchains = [*swapchain_khr.as_ref()];
                    let image_indices = [d.vulkan_image_index().load(Ordering::Relaxed)];

                    let present_info = vk::PresentInfoKHR::default()
                        .wait_semaphores(&signal_semaphores)
                        .swapchains(&swapchains)
                        .image_indices(&image_indices);

                    let result = unsafe {
                        swapchain_instance.queue_present(
                            self.queue.vulkan_command_queue.present_queue,
                            &present_info,
                        )
                    };

                    match result {
                        Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                            todo!("Handle Out of Date Swapchains.");
                        }
                        Err(error) => {
                            return Err(anyhow!("Failed to present queue. Cause: {}", error));
                        }
                        _ => {}
                    }

                    unsafe {
                        device.queue_wait_idle(self.queue.vulkan_command_queue.graphics_queue)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for MTLCommandBuffer {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    fn drop(&mut self) {
        let buffer = self.vulkan_command_buffer;

        unsafe {
            self.queue
                .device
                .vulkan_device()
                .logical()
                .reset_command_buffer(buffer, vk::CommandBufferResetFlags::empty())
                .unwrap();
        }

        let buffer_queue = self.queue.vulkan_command_queue.command_buffers();

        buffer_queue.push(buffer);
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    fn drop(&mut self) {
        // TODO: Metal Support.
    }
}

pub struct MTLRenderCommandEncoder {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_render_command_encoder: Retained<ProtocolObject<dyn MetalMTLRenderCommandEncoder>>,

    command_buffer: Arc<MTLCommandBuffer>,
}

impl MTLRenderCommandEncoder {
    pub fn new(
        command_buffer: Arc<MTLCommandBuffer>,
        render_pass: Arc<MTLRenderPass>,
        begin_descriptor: MTLBeginRenderPassDescriptor,
    ) -> Result<Self> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_new(command_buffer, render_pass, begin_descriptor);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_new(command_buffer, render_pass, begin_descriptor);
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_new(
        command_buffer: Arc<MTLCommandBuffer>,
        render_pass: Arc<MTLRenderPass>,
        begin_descriptor: MTLBeginRenderPassDescriptor,
    ) -> Result<Self> {
        let vk_render_pass = render_pass
            .vulkan_render_pass(&begin_descriptor)?
            .borrow()
            .unwrap();

        begin_descriptor.vulkan_framebuffer_check(&vk_render_pass, render_pass.device().clone())?;

        let device = command_buffer.queue.device.vulkan_device().logical();

        unsafe {
            device.begin_command_buffer(
                command_buffer.vulkan_command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE),
            )?;
        }

        let clear_color_values = begin_descriptor.vulkan_clear_color_values();

        let texture = begin_descriptor.color_attachments[0].texture.clone();

        let begin_render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(vk_render_pass)
            .framebuffer(texture.vulkan_framebuffer().read().unwrap().unwrap())
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: texture.width(),
                    height: texture.height(),
                },
            })
            .clear_values(&clear_color_values);

        unsafe {
            device.cmd_begin_render_pass(
                command_buffer.vulkan_command_buffer,
                &begin_render_pass_info,
                vk::SubpassContents::INLINE,
            );
        }

        Ok(Self { command_buffer })
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_new(
        command_buffer: Arc<MTLCommandBuffer>,
        render_pass: Arc<MTLRenderPass>,
        begin_descriptor: MTLBeginRenderPassDescriptor,
    ) -> Result<Self> {
        let metal_render_command_encoder = command_buffer
            .metal_command_buffer
            .renderCommandEncoderWithDescriptor(
                render_pass
                    .descriptor()
                    .to_metal(&begin_descriptor)
                    .as_ref(),
            );

        let metal_render_command_encoder = match metal_render_command_encoder {
            Some(c) => c,
            None => return Err(anyhow!("Render Command Encoder creation failed.")),
        };

        Ok(Self {
            command_buffer,
            metal_render_command_encoder,
        })
    }

    pub fn end_encoding(&self) -> Result<()> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return self.metal_end_encoding();

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return self.vulkan_end_encoding();
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_end_encoding(&self) -> Result<()> {
        let device = self.command_buffer.queue.device.vulkan_device().logical();

        unsafe {
            device.cmd_end_render_pass(self.command_buffer.vulkan_command_buffer);
            device.end_command_buffer(self.command_buffer.vulkan_command_buffer)?;
        }

        Ok(())
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_end_encoding(&self) -> Result<()> {
        self.metal_render_command_encoder.endEncoding();

        Ok(())
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanMTLCommandQueue {
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    command_pool: vk::CommandPool,
    command_buffers: SegQueue<vk::CommandBuffer>,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanMTLCommandQueue {
    pub fn command_buffers(&self) -> &SegQueue<vk::CommandBuffer> {
        &self.command_buffers
    }
}

pub enum MTLCommandBufferHandler {
    Present(Arc<MTLTexture>),
}
