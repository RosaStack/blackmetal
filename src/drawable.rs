use crate::{BMLLayer, MTLDevice, VulkanSurface};
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use crate::{MTLEvent, MTLFence};
use anyhow::{Result, anyhow};
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use std::sync::{Arc, Mutex};
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use std::{cell::RefCell, sync::atomic::AtomicU32};

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk;

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::{rc::Retained, runtime::ProtocolObject};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use raw_window_metal::Layer;

pub struct MTLDrawable {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    ca_metal_drawable: Retained<ProtocolObject<dyn CAMetalDrawable>>,
}

impl MTLDrawable {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn from_metal(ca_metal_drawable: Retained<ProtocolObject<dyn CAMetalDrawable>>) -> Self {
        Self { ca_metal_drawable }
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn ca_metal_drawable(&self) -> &Retained<ProtocolObject<dyn CAMetalDrawable>> {
        &self.ca_metal_drawable
    }
}

pub struct MTLTexture {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_image: vk::Image,
}

impl MTLTexture {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn from_vulkan(vulkan_image: vk::Image) -> Arc<Self> {
        Arc::new(Self { vulkan_image })
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_image(&self) -> &vk::Image {
        &self.vulkan_image
    }
}

pub struct MTLView {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    ca_metal_layer: Retained<CAMetalLayer>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_view: VulkanMTLView,
}

impl MTLView {
    pub fn request(device: Arc<MTLDevice>) -> Result<Arc<Self>> {
        let bml_layer = match device.instance.layer() {
            Some(l) => l,
            None => {
                return Err(anyhow!("Can't request on a headless instance."));
            }
        };

        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_request(bml_layer, &device);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_request(bml_layer, &device);
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_request(bml_layer: &BMLLayer, device: &Arc<MTLDevice>) -> Result<Arc<Self>> {
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

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_get_surface_details(
        surface: &VulkanSurface,
        device: &Arc<MTLDevice>,
        bml_layer: &BMLLayer,
    ) -> Result<VulkanSurfaceDetails> {
        let capabilities = unsafe {
            surface
                .instance()
                .get_physical_device_surface_capabilities(
                    *device.vulkan_device().physical(),
                    *surface.khr(),
                )?
        };

        let formats = unsafe {
            surface.instance().get_physical_device_surface_formats(
                *device.vulkan_device().physical(),
                *surface.khr(),
            )?
        };

        let present_modes = unsafe {
            surface
                .instance()
                .get_physical_device_surface_present_modes(
                    *device.vulkan_device().physical(),
                    *surface.khr(),
                )?
        };

        let surface_format = if formats.len() == 1 && formats[0].format == vk::Format::UNDEFINED {
            vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            }
        } else {
            *formats
                .iter()
                .find(|format| {
                    format.format == vk::Format::B8G8R8A8_UNORM
                        && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                })
                .unwrap_or(&formats[0])
        };

        let surface_present_mode = if present_modes.contains(&vk::PresentModeKHR::FIFO) {
            vk::PresentModeKHR::FIFO
        } else {
            vk::PresentModeKHR::IMMEDIATE
        };

        let surface_extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            let min = capabilities.min_image_extent;
            let max = capabilities.max_image_extent;

            let width = bml_layer.width.min(max.width).max(min.width);
            let height = bml_layer.height.min(max.height).max(min.height);

            vk::Extent2D { width, height }
        };

        Ok(VulkanSurfaceDetails {
            capabilities,
            format: surface_format,
            present_mode: surface_present_mode,
            extent: surface_extent,
        })
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_request(bml_layer: &BMLLayer, device: &Arc<MTLDevice>) -> Result<Arc<Self>> {
        //
        // =======================================================
        // TODO: This currently sets a lot of things by default.
        // But in the future all of this should be converted to
        // Metal types for finer granular control.
        // =======================================================
        //
        let surface = device.instance.vulkan_surface().as_ref().unwrap();

        let surface_details = Self::vulkan_get_surface_details(surface, device, bml_layer)?;

        let image_count = {
            let max = surface_details.capabilities.max_image_count;
            let mut preferred = surface_details.capabilities.min_image_count + 1;
            if max > 0 && preferred > max {
                preferred = max;
            }
            preferred
        };

        let queue_family_indices = [
            device.vulkan_device().queue_families().graphics_queue,
            device.vulkan_device().queue_families().present_queue,
        ];

        let swapchain_create_info = {
            let mut builder = vk::SwapchainCreateInfoKHR::default()
                .surface(*surface.khr())
                .min_image_count(image_count)
                .image_format(surface_details.format.format)
                .image_color_space(surface_details.format.color_space)
                .image_extent(surface_details.extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

            builder = if queue_family_indices[0] != queue_family_indices[1] {
                builder
                    .image_sharing_mode(vk::SharingMode::CONCURRENT)
                    .queue_family_indices(&queue_family_indices)
            } else {
                builder.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            };

            builder
                .pre_transform(surface_details.capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(surface_details.present_mode)
                .clipped(true)
        };

        let swapchain_instance = ash::khr::swapchain::Device::new(
            device.instance.vulkan_instance(),
            device.vulkan_device().logical(),
        );

        let swapchain_khr =
            unsafe { swapchain_instance.create_swapchain(&swapchain_create_info, None)? };

        let swapchain_images = unsafe { swapchain_instance.get_swapchain_images(swapchain_khr)? };
        let textures = swapchain_images
            .iter()
            .map(|image| MTLTexture::from_vulkan(*image))
            .collect::<Vec<_>>();

        let in_flight_frames = Self::vulkan_create_in_flight_frames(&device)?;

        Ok(Arc::new(Self {
            vulkan_view: VulkanMTLView {
                surface_details,
                swapchain: VulkanSwapchain {
                    instance: swapchain_instance,
                    khr: RefCell::new(swapchain_khr),
                    textures: RefCell::new(textures),
                    in_flight_frames,
                },
            },
        }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_create_in_flight_frames(
        device: &Arc<MTLDevice>,
    ) -> Result<(Vec<VulkanSyncObject>, AtomicU32)> {
        let mut sync_objects: Vec<VulkanSyncObject> = vec![];

        // TODO: This supports double buffering only.
        // Implement option to use triple buffering (recommended by Apple)
        // in the future.
        for _ in 0..2 {
            let image_available_event = MTLEvent::make(device.clone())?;
            let render_finished_event = MTLEvent::make(device.clone())?;

            let fence = MTLFence::make(device.clone())?;

            sync_objects.push(VulkanSyncObject {
                image_available_event,
                render_finished_event,
                fence,
            });
        }

        Ok((sync_objects, AtomicU32::new(0)))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_view(&self) -> &VulkanMTLView {
        &self.vulkan_view
    }

    pub fn next_drawable(&self) -> Result<Arc<MTLDrawable>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return self.metal_next_drawable();

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return self.vulkan_next_drawable();
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_next_drawable(&self) -> Result<Arc<MTLDrawable>> {
        todo!("Vulkan Support")
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn metal_next_drawable(&self) -> Result<Arc<MTLDrawable>> {
        let ca_metal_drawable = unsafe { self.ca_metal_layer.nextDrawable() };

        let ca_metal_drawable = match ca_metal_drawable {
            Some(d) => d,
            None => {
                return Err(anyhow!(
                    "Failed to get the next `MTLDrawable` in the swapchain."
                ));
            }
        };

        Ok(Arc::new(MTLDrawable::from_metal(ca_metal_drawable)))
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanMTLView {
    surface_details: VulkanSurfaceDetails,
    swapchain: VulkanSwapchain,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanMTLView {
    pub fn swapchain(&self) -> &VulkanSwapchain {
        &self.swapchain
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanSwapchain {
    instance: ash::khr::swapchain::Device,
    khr: RefCell<vk::SwapchainKHR>,
    textures: RefCell<Vec<Arc<MTLTexture>>>, // <- I'm so sorry for this abomination, but i have no choice ;_;.
    in_flight_frames: (Vec<VulkanSyncObject>, AtomicU32),
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanSwapchain {
    pub fn textures(&self) -> &RefCell<Vec<Arc<MTLTexture>>> {
        &self.textures
    }

    pub fn khr(&self) -> &RefCell<vk::SwapchainKHR> {
        &self.khr
    }

    pub fn instance(&self) -> &ash::khr::swapchain::Device {
        &self.instance
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanSyncObject {
    image_available_event: Arc<MTLEvent>,
    render_finished_event: Arc<MTLEvent>,
    fence: Arc<MTLFence>,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanSurfaceDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
    extent: vk::Extent2D,
}
