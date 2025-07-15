#[cfg(any(target_os = "macos", target_os = "ios"))]
use objc2::runtime::ProtocolObject;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use objc2_quartz_core::CAMetalLayer;
use raw_window_metal::Layer;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use objc2::rc::Retained;
use objc2_metal::{
    MTLClearColor as MetalMTLClearColor, MTLCommandBuffer as MetalMTLCommandBuffer,
    MTLCommandEncoder as MetalMTLCommandEncoder, MTLCommandQueue as MetalMTLCommandQueue,
    MTLCreateSystemDefaultDevice, MTLDevice as MetalMTLDevice, MTLLoadAction as MetalMTLLoadAction,
    MTLRenderCommandEncoder as MetalMTLRenderCommandEncoder,
    MTLRenderPassColorAttachmentDescriptor as MetalMTLRenderPassColorAttachmentDescriptor,
    MTLRenderPassDescriptor as MetalMTLRenderPassDescriptor, MTLStoreAction as MetalMTLStoreAction,
};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub struct BMLInstance {
    layer: Option<BMLLayer>,
}

impl BMLInstance {
    pub fn new(layer: Option<BMLLayer>) -> Result<Arc<Self>> {
        Ok(Arc::new(Self { layer }))
    }
}

pub struct BMLLayer {
    pub window_display: RawDisplayHandle,
    pub window_handle: RawWindowHandle,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct MTLRenderPassDescriptor {
    pub color_attachments: Vec<MTLRenderPassColorAttachment>,
}

impl MTLRenderPassDescriptor {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
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
    pub view: Arc<MTLView>,
}

impl MTLRenderPassColorAttachment {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn set_in_metal(&self, result: &Retained<MetalMTLRenderPassDescriptor>, count: usize) {
        use objc2_quartz_core::CAMetalDrawable;

        let color_result = MetalMTLRenderPassColorAttachmentDescriptor::new();

        color_result.setClearColor(self.clear_color.to_metal());
        color_result.setLoadAction(self.load_action.to_metal());
        color_result.setStoreAction(self.store_action.to_metal());

        unsafe {
            // TODO: Add Cross-platform options in the future.
            color_result.setTexture(Some(
                self.view
                    .ca_metal_layer
                    .nextDrawable()
                    .unwrap()
                    .texture()
                    .as_ref(),
            ));

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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
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

pub struct MTLRenderCommandEncoder {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal_render_command_encoder: Retained<ProtocolObject<dyn MetalMTLRenderCommandEncoder>>,
}

impl MTLRenderCommandEncoder {
    pub fn new(
        command_buffer: Arc<MTLCommandBuffer>,
        render_pass: MTLRenderPassDescriptor,
    ) -> Result<Self> {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        return Self::metal_new(command_buffer, render_pass);

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        todo!("Vulkan Support")
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
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
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        return self.metal_end_encoding();

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        todo!("Vulkan Support")
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn metal_end_encoding(&self) {
        self.metal_render_command_encoder.endEncoding();
    }
}

pub struct MTLView {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    ca_metal_layer: Retained<CAMetalLayer>,
}

impl MTLView {
    pub fn request(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let bml_layer = match &device.instance.layer {
            Some(l) => l,
            None => {
                return Err(anyhow!(
                    "Can't request on a headless instance. Use `MTKView::init()` instead."
                ));
            }
        };

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        return Self::metal_request(bml_layer, device.clone());

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        todo!("Vulkan Support")
    }

    pub fn metal_request(bml_layer: &BMLLayer, device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let ca_metal_layer = match bml_layer.window_handle {
            RawWindowHandle::AppKit(handle) => unsafe { Layer::from_ns_view(handle.ns_view) },
            RawWindowHandle::UiKit(handle) => unsafe { Layer::from_ui_view(handle.ui_view) },
            _ => return Err(anyhow!("Unsupported handle.")),
        };

        let ca_metal_layer: *mut CAMetalLayer = ca_metal_layer.into_raw().as_ptr().cast();

        let ca_metal_layer = unsafe { Retained::from_raw(ca_metal_layer).unwrap() };

        unsafe {
            ca_metal_layer.setDevice(Some(device.metal_device.as_ref()));
        }

        Ok(Arc::new(Self { ca_metal_layer }))
    }
}

impl MTLDevice {
    pub fn create(instance: Arc<BMLInstance>) -> Result<Arc<Self>> {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        return Self::metal_create(instance);

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        todo!("Vulkan Support")
    }

    pub fn metal_create(instance: Arc<BMLInstance>) -> Result<Arc<Self>> {
        let metal_device = MTLCreateSystemDefaultDevice();

        let metal_device = match metal_device {
            Some(m) => m,
            None => return Err(anyhow!("No device found.")),
        };

        let name = metal_device.name().to_string();

        Ok(Arc::new(Self {
            name,
            instance,
            metal_device,
        }))
    }
}

pub struct MTLDevice {
    name: String,
    instance: Arc<BMLInstance>,

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal_device: Retained<ProtocolObject<dyn MetalMTLDevice>>,
}

pub struct MTLCommandQueue {
    device: Arc<MTLDevice>,

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal_command_queue: Retained<ProtocolObject<dyn MetalMTLCommandQueue>>,
}

impl MTLCommandQueue {
    pub fn new(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        return Self::metal_new(device);

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        todo!("Vulkan Support")
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn metal_new(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let metal_command_queue = device.metal_device.newCommandQueue();

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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal_command_buffer: Retained<ProtocolObject<dyn MetalMTLCommandBuffer>>,
}

impl MTLCommandBuffer {
    pub fn new(queue: Arc<MTLCommandQueue>) -> Result<Arc<Self>> {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        return Self::metal_new(queue);

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        todo!("Vulkan Support")
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
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
