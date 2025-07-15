use anyhow::{Result, anyhow};

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::{Entry, Instance};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::sync::Arc;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use std::{ffi::CString, os::raw::c_char, sync::LazyLock};

pub struct BMLInstance {
    layer: Option<BMLLayer>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_instance: ash::Instance,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub static VULKAN_ENTRY: LazyLock<Entry> = LazyLock::new(|| unsafe { Entry::load().unwrap() });

impl BMLInstance {
    pub fn new(layer: Option<BMLLayer>) -> Result<Arc<Self>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_new(layer);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_new(layer);
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_new(layer: Option<BMLLayer>) -> Result<Arc<Self>> {
        Ok(Arc::new(Self { layer }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_new(layer: Option<BMLLayer>) -> Result<Arc<Self>> {
        let vulkan_instance = Self::vulkan_create_instance(&layer)?;

        Ok(Arc::new(Self {
            layer,
            vulkan_instance,
        }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_create_instance(layer: &Option<BMLLayer>) -> Result<Instance> {
        use ash::vk;
        use std::ffi::CString;

        let api_version = vk::make_api_version(
            env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>()?,
            env!("CARGO_PKG_VERSION_MINOR").parse::<u32>()?,
            env!("CARGO_PKG_VERSION_PATCH").parse::<u32>()?,
            0,
        );

        let app_name = CString::new("BlackMetal")?;
        let engine_name = CString::new("BlackMetal")?;
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name.as_c_str())
            .application_version(api_version)
            .engine_name(engine_name.as_c_str())
            .engine_version(api_version)
            .api_version(api_version);

        let mut extension_names = match layer {
            Some(l) => ash_window::enumerate_required_extensions(l.window_display)?.to_vec(),
            None => vec![],
        };

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
            extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
        }

        let (_layer_names, layer_names_ptrs) = Self::vulkan_get_layer_names_and_pointers();

        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

        // TODO: Add debug info.

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names_ptrs)
            .flags(create_flags);

        Ok(unsafe { VULKAN_ENTRY.create_instance(&instance_create_info, None)? })
    }

    pub const REQUIRED_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

    /// Get the pointers to the validation layers names.
    /// Also return the corresponding `CString` to avoid dangling pointers.
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_get_layer_names_and_pointers() -> (Vec<CString>, Vec<*const c_char>) {
        let layer_names = Self::REQUIRED_LAYERS
            .iter()
            .map(|name| CString::new(*name).unwrap())
            .collect::<Vec<_>>();
        let layer_names_ptrs = layer_names
            .iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();
        (layer_names, layer_names_ptrs)
    }

    pub fn layer(&self) -> &Option<BMLLayer> {
        &self.layer
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanBMLInstance {}

pub struct BMLLayer {
    pub window_display: RawDisplayHandle,
    pub window_handle: RawWindowHandle,
    pub width: u32,
    pub height: u32,
}
