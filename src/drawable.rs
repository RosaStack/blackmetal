use crate::MTLDevice;
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub struct MTLDrawable {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    ca_metal_drawable: Retained<ProtocolObject<dyn CAMetalDrawable>>,
}

impl MTLDrawable {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn from_metal(ca_metal_drawable: Retained<ProtocolObject<dyn CAMetalDrawable>>) -> Self {
        Self { ca_metal_drawable }
    }
}

pub struct MTLView {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    ca_metal_layer: Retained<CAMetalLayer>,
}

impl MTLView {
    pub fn request(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let bml_layer = match device.instance.layer() {
            Some(l) => l,
            None => {
                return Err(anyhow!(
                    "Can't request on a headless instance. Use `MTKView::init()` instead."
                ));
            }
        };

        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_request(bml_layer, device.clone());

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_request(bml_layer: &BMLLayer, device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        use raw_window_handle::RawWindowHandle;

        let ca_metal_layer = match bml_layer.window_handle {
            RawWindowHandle::AppKit(handle) => unsafe { Layer::from_ns_view(handle.ns_view) },
            RawWindowHandle::UiKit(handle) => unsafe { Layer::from_ui_view(handle.ui_view) },
            _ => return Err(anyhow!("Unsupported handle.")),
        };

        let ca_metal_layer: *mut CAMetalLayer = ca_metal_layer.into_raw().as_ptr().cast();

        let ca_metal_layer = unsafe { Retained::from_raw(ca_metal_layer).unwrap() };

        unsafe {
            ca_metal_layer.setDevice(Some(device.metal_device().as_ref()));
        }

        Ok(Arc::new(Self { ca_metal_layer }))
    }

    pub fn next_drawable(&self) -> Result<Arc<MTLDrawable>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return self.metal_next_drawable();

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_next_drawable(&self) -> Result<Arc<MTLDrawable>> {
        let ca_metal_drawable = unsafe { self.ca_metal_layer.nextDrawable() };

        let ca_metal_drawable = match ca_metal_drawable {
            Some(d) => d,
            None => {
                return Err(anyhow!(
                    "Failed to get the next `MTLDrawable` in the sweapchain."
                ));
            }
        };

        Ok(Arc::new(MTLDrawable::from_metal(ca_metal_drawable)))
    }
}
