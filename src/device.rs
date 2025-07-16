use crate::BMLInstance;
use anyhow::{Result, anyhow};
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk;
use std::sync::Arc;

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::{rc::Retained, runtime::ProtocolObject};

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice as MetalMTLDevice};

pub struct MTLDevice {
    name: String,
    pub instance: Arc<BMLInstance>,

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_device: Retained<ProtocolObject<dyn MetalMTLDevice>>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_device: VulkanMTLDevice,
}

impl MTLDevice {
    pub fn create(instance: Arc<BMLInstance>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_create(instance);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_create(instance);
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_create(instance: Arc<BMLInstance>) -> Result<Arc<Self>> {
        use std::ffi::CStr;

        let devices = unsafe { instance.vulkan_instance().enumerate_physical_devices()? };

        // TODO: Check if the device is suitable.
        // DON'T MAKE THIS A PERMANENT SOLUTION ME
        // FROM THE FUTURE I BEG YOU!!!!!
        let physical_device = devices[0];

        let properties = unsafe {
            instance
                .vulkan_instance()
                .get_physical_device_properties(physical_device)
        };

        let name = unsafe {
            CStr::from_ptr(properties.device_name.as_ptr())
                .to_str()?
                .to_string()
        };

        Ok(Arc::new(Self {
            name,
            instance,
            vulkan_device: VulkanMTLDevice { physical_device },
        }))
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

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_device(&self) -> &VulkanMTLDevice {
        &self.vulkan_device
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanMTLDevice {
    physical_device: vk::PhysicalDevice,
}
