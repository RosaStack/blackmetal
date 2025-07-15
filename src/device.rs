use crate::BMLInstance;
use anyhow::{Result, anyhow};
use std::sync::Arc;

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::{rc::Retained, runtime::ProtocolObject};

use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice as MetalMTLDevice};

pub struct MTLDevice {
    name: String,
    pub instance: Arc<BMLInstance>,

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_device: Retained<ProtocolObject<dyn MetalMTLDevice>>,
}

impl MTLDevice {
    pub fn create(instance: Arc<BMLInstance>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_create(instance);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
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

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_device(&self) -> &Retained<ProtocolObject<dyn MetalMTLDevice>> {
        &self.metal_device
    }
}
