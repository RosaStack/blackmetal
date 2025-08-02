use std::borrow::Borrow;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use std::cell::RefCell;
use std::sync::Arc;

use crate::{MTLDevice, MTLTexture};

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use anyhow::Result;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use anyhow::Result;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk;
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::rc::Retained;
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_metal::{
    MTLClearColor as MetalMTLClearColor, MTLLoadAction as MetalMTLLoadAction,
    MTLRenderPassColorAttachmentDescriptor as MetalMTLRenderPassColorAttachmentDescriptor,
    MTLRenderPassDescriptor as MetalMTLRenderPassDescriptor, MTLStoreAction as MetalMTLStoreAction,
};

pub struct MTLRenderPass {
    device: Arc<MTLDevice>,
    descriptor: MTLRenderPassDescriptor,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_render_pass: RefCell<Option<vk::RenderPass>>,
}

impl MTLRenderPass {
    pub fn new(device: Arc<MTLDevice>, descriptor: MTLRenderPassDescriptor) -> Arc<Self> {
        Arc::new(Self {
            device,
            descriptor,
            #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
            vulkan_render_pass: RefCell::new(None),
        })
    }

    pub fn descriptor(&self) -> &MTLRenderPassDescriptor {
        &self.descriptor
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_render_pass(
        &self,
        begin: &MTLBeginRenderPassDescriptor,
    ) -> Result<&RefCell<Option<vk::RenderPass>>> {
        if self.vulkan_render_pass.borrow().is_some() {
            // TODO: Handle more cases in the future other than
            // just checking if the render pass exists.
            return Ok(&self.vulkan_render_pass);
        }

        // I fucking hate this, this is a terrible, TERRIBLE solution,
        // but the borrow checker has forced my hand and i've tried to
        // fix this shit for 8 hours.
        //
        // Does it have an insane performance penalty? Of course it does.
        // But its either this or having to deal with lifetime shenanigans that
        // consist of man-made horrors beyond my comprehension.
        // This is a great example that Rust doesn't free you from having
        // to write shitty code.
        let mut handle = VulkanRenderPassHandler::default();
        let new_handle = self.descriptor.to_vulkan(begin, &mut handle);

        self.vulkan_render_pass.replace(Some(unsafe {
            self.device
                .vulkan_device()
                .logical()
                .create_render_pass(&new_handle.final_render_pass_create_info, None)?
        }));

        Ok(&self.vulkan_render_pass)
    }

    pub fn device(&self) -> &Arc<MTLDevice> {
        &self.device
    }
}

#[derive(Default)]
pub struct MTLRenderPassDescriptor {
    pub color_attachments: Vec<MTLRenderPassColorAttachment>,
}

impl<'a> MTLRenderPassDescriptor {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn to_vulkan(
        &self,
        begin: &MTLBeginRenderPassDescriptor,
        handle: &'a mut VulkanRenderPassHandler<'a>,
    ) -> VulkanRenderPassHandler<'a> {
        handle.color_attachments = self.vulkan_color_attachments(begin);

        handle
            .attachment_descriptions
            .extend_from_slice(&handle.color_attachments);

        let mut ref_count = 0_u32;

        for _i in &handle.color_attachments {
            handle.color_attachment_refs.push(
                vk::AttachmentReference::default()
                    .attachment(ref_count)
                    .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
            );
            ref_count += 1;
        }

        handle.subpass_descriptions = vec![
            vk::SubpassDescription::default()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&handle.color_attachment_refs),
        ];

        handle.subpass_dependencies = vec![
            vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(
                    vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                ),
        ];

        handle.final_render_pass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&handle.attachment_descriptions)
            .subpasses(&handle.subpass_descriptions)
            .dependencies(&handle.subpass_dependencies);

        handle.clone()
    }
}

impl MTLRenderPassDescriptor {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn to_metal(
        &self,
        begin: &MTLBeginRenderPassDescriptor,
    ) -> Retained<MetalMTLRenderPassDescriptor> {
        let mut result = unsafe { MetalMTLRenderPassDescriptor::new() };

        let mut count = 0;
        for i in &self.color_attachments {
            i.set_in_metal(&mut result, begin, count);
            count += 1;
        }

        result
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_color_attachments(
        &self,
        begin: &MTLBeginRenderPassDescriptor,
    ) -> Vec<vk::AttachmentDescription> {
        let mut result: Vec<vk::AttachmentDescription> = vec![];
        let mut count = 0;
        for i in &self.color_attachments {
            result.push(i.to_vulkan(begin, count));
            count += 1;
        }

        result
    }
}

#[derive(Default, Clone)]
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanRenderPassHandler<'a> {
    color_attachments: Vec<vk::AttachmentDescription>,
    attachment_descriptions: Vec<vk::AttachmentDescription>,
    color_attachment_refs: Vec<vk::AttachmentReference>,
    subpass_descriptions: Vec<vk::SubpassDescription<'a>>,
    subpass_dependencies: Vec<vk::SubpassDependency>,
    final_render_pass_create_info: vk::RenderPassCreateInfo<'a>,
}

#[derive(Default)]
pub struct MTLBeginRenderPassDescriptor {
    pub color_attachments: Vec<MTLBeginRenderPassColorAttachment>,
}

impl MTLBeginRenderPassDescriptor {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_framebuffer_check(
        &self,
        render_pass: &vk::RenderPass,
        device: Arc<MTLDevice>,
    ) -> Result<()> {
        for i in &self.color_attachments {
            if !i.texture.vulkan_is_framebuffer() {
                unsafe {
                    i.texture
                        .vulkan_create_framebuffer(render_pass, device.clone())?;
                }
            }
        }

        Ok(())
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_clear_color_values(&self) -> Vec<vk::ClearValue> {
        let mut result: Vec<vk::ClearValue> = vec![];

        for i in &self.color_attachments {
            result.push(i.clear_color.to_vulkan());
        }

        result
    }
}

pub struct MTLBeginRenderPassColorAttachment {
    pub clear_color: MTLClearColor,
    pub texture: Arc<MTLTexture>,
}

pub struct MTLRenderPassColorAttachment {
    pub load_action: MTLLoadAction,
    pub store_action: MTLStoreAction,
}

impl MTLRenderPassColorAttachment {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn set_in_metal(
        &self,
        result: &Retained<MetalMTLRenderPassDescriptor>,
        begin_descriptor: &MTLBeginRenderPassDescriptor,
        count: usize,
    ) -> Result<()> {
        let color_result = MetalMTLRenderPassColorAttachmentDescriptor::new();

        color_result.setClearColor(
            begin_descriptor.color_attachments[count]
                .clear_color
                .to_metal(),
        );
        color_result.setLoadAction(self.load_action.to_metal());
        color_result.setStoreAction(self.store_action.to_metal());

        unsafe {
            // TODO: Add Cross-platform options in the future.
            color_result.setTexture(Some(
                begin_descriptor.color_attachments[count]
                    .texture
                    .to_metal()?
                    .as_ref(),
            ));

            result
                .colorAttachments()
                .setObject_atIndexedSubscript(color_result.downcast_ref(), count);
        }

        Ok(())
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn to_vulkan(
        &self,
        begin: &MTLBeginRenderPassDescriptor,
        count: usize,
    ) -> vk::AttachmentDescription {
        let format = begin.color_attachments[count]
            .texture
            .pixel_format()
            .to_vulkan();

        vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(self.load_action.to_vulkan())
            .store_op(self.store_action.to_vulkan())
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        //.final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    }
}

pub struct MTLClearColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl MTLClearColor {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn to_metal(&self) -> MetalMTLClearColor {
        MetalMTLClearColor {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha: self.alpha,
        }
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn to_vulkan(&self) -> vk::ClearValue {
        vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [
                    self.red as f32,
                    self.green as f32,
                    self.blue as f32,
                    self.alpha as f32,
                ],
            },
        }
    }
}

pub enum MTLLoadAction {
    DontCare,
    Load,
    Clear,
}

impl MTLLoadAction {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn to_metal(&self) -> MetalMTLLoadAction {
        match self {
            MTLLoadAction::DontCare => MetalMTLLoadAction::DontCare,
            MTLLoadAction::Load => MetalMTLLoadAction::Load,
            MTLLoadAction::Clear => MetalMTLLoadAction::Clear,
        }
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn to_vulkan(&self) -> vk::AttachmentLoadOp {
        match self {
            Self::Clear => vk::AttachmentLoadOp::CLEAR,
            Self::Load => vk::AttachmentLoadOp::LOAD,
            Self::DontCare => vk::AttachmentLoadOp::DONT_CARE,
        }
    }
}

pub enum MTLStoreAction {
    DontCare,
    Store,
    Unknown,
}

impl MTLStoreAction {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn to_metal(&self) -> MetalMTLStoreAction {
        match self {
            MTLStoreAction::DontCare => MetalMTLStoreAction::DontCare,
            MTLStoreAction::Store => MetalMTLStoreAction::Store,
            MTLStoreAction::Unknown => MetalMTLStoreAction::Unknown,
        }
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn to_vulkan(&self) -> vk::AttachmentStoreOp {
        match self {
            Self::DontCare => vk::AttachmentStoreOp::DONT_CARE,
            Self::Store => vk::AttachmentStoreOp::STORE,
            Self::Unknown => vk::AttachmentStoreOp::NONE,
        }
    }
}
