use crate::BMLInstance;
use anyhow::{Result, anyhow};
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk;
use std::ffi::CStr;
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
        let devices = unsafe { instance.vulkan_instance().enumerate_physical_devices()? };

        let physical_device = devices
            .into_iter()
            .find(|device| Self::vulkan_device_check(&instance, device).is_ok())
            .expect("No suitable Physical Device.");

        let queue_families = Self::vulkan_find_queue_families(&instance, &physical_device)?;

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

        let logical_device =
            Self::vulkan_create_logical_device(&instance, &physical_device, &queue_families)?;

        Ok(Arc::new(Self {
            name,
            instance,
            vulkan_device: VulkanMTLDevice {
                physical_device,
                logical_device,
                queue_families,
            },
        }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_create_logical_device(
        instance: &Arc<BMLInstance>,
        device: &vk::PhysicalDevice,
        queue_families: &VulkanQueueFamilies,
    ) -> Result<ash::Device> {
        let mut indices = vec![queue_families.graphics_queue, queue_families.present_queue];
        indices.dedup();

        let queue_info = indices
            .iter()
            .map(|index| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(*index)
                    .queue_priorities(&[1.0_f32])
            })
            .collect::<Vec<_>>();

        let device_extensions = Self::vulkan_required_extensions()
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extensions);

        Ok(unsafe {
            instance
                .vulkan_instance()
                .create_device(*device, &device_create_info, None)?
        })
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_device_check(
        instance: &Arc<BMLInstance>,
        device: &vk::PhysicalDevice,
    ) -> Result<()> {
        Self::vulkan_find_queue_families(instance, device)?;

        Self::vulkan_check_extension_support(instance, device)?;

        Ok(())
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_find_queue_families(
        instance: &Arc<BMLInstance>,
        device: &vk::PhysicalDevice,
    ) -> Result<VulkanQueueFamilies> {
        let mut graphics: Option<u32> = None;
        let mut present: Option<u32> = None;

        let properties = unsafe {
            instance
                .vulkan_instance()
                .get_physical_device_queue_family_properties(*device)
        };

        for (index, family) in properties.iter().filter(|f| f.queue_count > 0).enumerate() {
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
                graphics = Some(index as u32);
            }

            // TODO: Present Support so far has to be done with a surface.
            // Find a way to get present support without it.
            //
            // By finding out, we could also follow the Metal way of requesting
            // a surface without having to instantiate it in `BMLInstance`.
            let present_support = unsafe {
                match instance.vulkan_surface() {
                    Some(v) => v.instance().get_physical_device_surface_support(
                        *device,
                        index as u32,
                        *v.khr(),
                    )?,
                    // Fallback to the graphics queue.
                    // Not ideal, but it'll work for now...
                    None => graphics.is_some(),
                }
            };

            if present_support && present.is_none() {
                present = Some(index as u32);
            }

            if graphics.is_some() && present.is_some() {
                break;
            }
        }

        if graphics.is_some() && present.is_some() {
            return Ok(VulkanQueueFamilies {
                graphics_queue: graphics.unwrap(),
                present_queue: present.unwrap(),
            });
        }

        Err(anyhow!("No suitable Graphics and Present queue found."))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_required_extensions() -> Vec<&'static CStr> {
        if cfg!(any(target_os = "macos", target_os = "ios")) {
            return vec![
                ash::khr::swapchain::NAME,
                ash::khr::portability_subset::NAME,
            ];
        }

        vec![ash::khr::swapchain::NAME]
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_check_extension_support(
        instance: &Arc<BMLInstance>,
        device: &vk::PhysicalDevice,
    ) -> Result<()> {
        let required_extensions = Self::vulkan_required_extensions();

        let extension_properties = unsafe {
            instance
                .vulkan_instance()
                .enumerate_device_extension_properties(*device)?
        };

        for required in required_extensions.iter() {
            let found = extension_properties.iter().any(|ext| {
                let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                required == &name
            });

            if !found {
                return Err(anyhow!("No required extension found."));
            }
        }

        Ok(())
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
    logical_device: ash::Device,
    queue_families: VulkanQueueFamilies,
}

pub struct VulkanQueueFamilies {
    graphics_queue: u32,
    present_queue: u32,
}
