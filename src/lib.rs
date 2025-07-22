pub mod command;
pub mod device;
pub mod drawable;
pub mod instance;
pub mod sync;

pub use command::*;
pub use device::*;
pub use drawable::*;
pub use instance::*;
pub use sync::*;

use std::sync::Arc;

use anyhow::{Result, anyhow};

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::rc::Retained;
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::runtime::ProtocolObject;
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_metal::{
    MTLClearColor as MetalMTLClearColor, MTLCommandBuffer as MetalMTLCommandBuffer,
    MTLCommandEncoder as MetalMTLCommandEncoder, MTLCommandQueue as MetalMTLCommandQueue,
    MTLDevice as MetalMTLDevice, MTLLoadAction as MetalMTLLoadAction,
    MTLRenderCommandEncoder as MetalMTLRenderCommandEncoder,
    MTLRenderPassColorAttachmentDescriptor as MetalMTLRenderPassColorAttachmentDescriptor,
    MTLRenderPassDescriptor as MetalMTLRenderPassDescriptor, MTLStoreAction as MetalMTLStoreAction,
};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};

pub use device::MTLDevice;
pub use instance::{BMLInstance, BMLLayer};

pub use ash::vk;

#[derive(Default)]
pub struct MTLRenderPassDescriptor {
    pub color_attachments: Vec<MTLRenderPassColorAttachment>,
}

impl MTLRenderPassDescriptor {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn to_metal(&self) -> Retained<MetalMTLRenderPassDescriptor> {
        let mut result = unsafe { MetalMTLRenderPassDescriptor::new() };

        let mut count = 0;
        for i in &self.color_attachments {
            i.set_in_metal(&mut result, count);
            count += 1;
        }

        result
    }
}

pub struct MTLRenderPassColorAttachment {
    pub clear_color: MTLClearColor,
    pub load_action: MTLLoadAction,
    pub store_action: MTLStoreAction,
    pub drawable: Arc<MTLDrawable>,
}

impl MTLRenderPassColorAttachment {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn set_in_metal(&self, result: &Retained<MetalMTLRenderPassDescriptor>, count: usize) {
        use objc2_quartz_core::CAMetalDrawable;

        let color_result = MetalMTLRenderPassColorAttachmentDescriptor::new();

        color_result.setClearColor(self.clear_color.to_metal());
        color_result.setLoadAction(self.load_action.to_metal());
        color_result.setStoreAction(self.store_action.to_metal());

        unsafe {
            // TODO: Add Cross-platform options in the future.
            color_result.setTexture(Some(self.drawable.ca_metal_drawable().texture().as_ref()));

            result
                .colorAttachments()
                .setObject_atIndexedSubscript(color_result.downcast_ref(), count);
        }
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
}

pub enum MTLStoreAction {
    DontCare,
    Store,
    MultisampleResolve,
    StoreAndMultisampleResolve,
    Unknown,
    CustomSampleDepthStore,
}

impl MTLStoreAction {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn to_metal(&self) -> MetalMTLStoreAction {
        match self {
            MTLStoreAction::CustomSampleDepthStore => MetalMTLStoreAction::CustomSampleDepthStore,
            MTLStoreAction::DontCare => MetalMTLStoreAction::DontCare,
            MTLStoreAction::MultisampleResolve => MetalMTLStoreAction::MultisampleResolve,
            MTLStoreAction::Store => MetalMTLStoreAction::Store,
            MTLStoreAction::StoreAndMultisampleResolve => {
                MetalMTLStoreAction::StoreAndMultisampleResolve
            }
            MTLStoreAction::Unknown => MetalMTLStoreAction::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;

    use super::*;

    #[test]
    fn headless_environment() -> Result<()> {
        let instance = BMLInstance::new(None)?;

        let device = MTLDevice::create(instance)?;

        Ok(())
    }
}
