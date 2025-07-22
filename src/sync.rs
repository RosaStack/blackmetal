use std::sync::Arc;

use anyhow::Result;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk;

use crate::MTLDevice;

pub struct MTLEvent {
    device: Arc<MTLDevice>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_semaphore: vk::Semaphore,
}

impl MTLEvent {
    pub fn make(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        todo!("Metal Support");

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_make(device);
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_make(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let vulkan_semaphore = unsafe {
            device
                .vulkan_device()
                .logical()
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };

        Ok(Arc::new(Self {
            device,
            vulkan_semaphore,
        }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_semaphore(&self) -> &vk::Semaphore {
        &self.vulkan_semaphore
    }

    pub fn device(&self) -> &Arc<MTLDevice> {
        &self.device
    }
}

pub struct MTLFence {
    device: Arc<MTLDevice>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_fence: vk::Fence,
}

impl MTLFence {
    pub fn make(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        todo!("Metal Support");

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_make(device);
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_make(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let vulkan_fence = unsafe {
            device.vulkan_device().logical().create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        Ok(Arc::new(Self {
            device,
            vulkan_fence,
        }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_fence(&self) -> &vk::Fence {
        &self.vulkan_fence
    }

    pub fn device(&self) -> &Arc<MTLDevice> {
        &self.device
    }
}
