use crate::{BMLLayer, MTLDevice};
use crate::{MTLEvent, MTLFence};
use anyhow::{Result, anyhow};
use crossbeam::queue::SegQueue;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use std::cell::Ref;
use std::sync::atomic::AtomicBool;
#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};
use std::{cell::RefCell, sync::atomic::AtomicU32};

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use crate::{MTLRenderPassDescriptor, VulkanSurface};

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
use ash::vk;

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2::{rc::Retained, runtime::ProtocolObject};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use raw_window_metal::Layer;

#[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
use objc2_metal::{
    MTLClearColor as MetalMTLClearColor, MTLCommandBuffer as MetalMTLCommandBuffer,
    MTLCommandEncoder as MetalMTLCommandEncoder, MTLCommandQueue as MetalMTLCommandQueue,
    MTLDevice as MetalMTLDevice, MTLLoadAction as MetalMTLLoadAction,
    MTLRenderCommandEncoder as MetalMTLRenderCommandEncoder,
    MTLRenderPassColorAttachmentDescriptor as MetalMTLRenderPassColorAttachmentDescriptor,
    MTLRenderPassDescriptor as MetalMTLRenderPassDescriptor, MTLStoreAction as MetalMTLStoreAction,
    MTLTexture as MetalMTLTexture,
};

pub struct MTLDrawable {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    ca_metal_drawable: Retained<ProtocolObject<dyn CAMetalDrawable>>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_drawable: VulkanMTLDrawable,

    view: Arc<MTLView>,
}

impl MTLDrawable {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn from_metal(
        view: Arc<MTLView>,
        ca_metal_drawable: Retained<ProtocolObject<dyn CAMetalDrawable>>,
    ) -> Self {
        Self {
            ca_metal_drawable,
            view,
        }
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn ca_metal_drawable(&self) -> &Retained<ProtocolObject<dyn CAMetalDrawable>> {
        &self.ca_metal_drawable
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn from_vulkan(view: Arc<MTLView>, vulkan_drawable: VulkanMTLDrawable) -> Self {
        Self {
            vulkan_drawable,
            view,
        }
    }

    pub fn view(&self) -> &Arc<MTLView> {
        &self.view
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanMTLDrawable {
    image_index: AtomicU32,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanMTLDrawable {
    pub fn image_index(&self) -> &AtomicU32 {
        &self.image_index
    }
}

pub struct MTLTexture {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_texture: Retained<ProtocolObject<dyn MetalMTLTexture>>,
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_image: vk::Image,
}

impl MTLTexture {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn from_vulkan(vulkan_image: vk::Image) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self { vulkan_image }))
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

    device: Arc<MTLDevice>,
    pixel_format: MTLPixelFormat,
    is_framebuffer_created: AtomicBool,
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

        Ok(Arc::new(Self {
            device: device.clone(),
            ca_metal_layer,
            is_framebuffer_created: AtomicBool::new(true),
        }))
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

        let pixel_format = MTLPixelFormat::from_vulkan(surface_details.format.format);

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

        let in_flight_frames = Self::vulkan_create_in_flight_frames(&device)?;

        Ok(Arc::new(Self {
            vulkan_view: VulkanMTLView {
                surface_details,
                swapchain: VulkanSwapchain {
                    instance: swapchain_instance,
                    khr: RefCell::new(swapchain_khr),
                    framebuffers: RwLock::new(vec![]),
                    in_flight_frames,
                    image_count,
                },
                render_pass_index: AtomicU32::new(0),
            },
            device: device.clone(),
            pixel_format,
            is_framebuffer_created: AtomicBool::new(false),
        }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_create_in_flight_frames(device: &Arc<MTLDevice>) -> Result<VulkanInFlightFrames> {
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

        Ok(VulkanInFlightFrames {
            sync_objects,
            current_frame: AtomicUsize::new(0),
        })
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_view(&self) -> &VulkanMTLView {
        &self.vulkan_view
    }

    pub fn pixel_format(&self) -> &MTLPixelFormat {
        &self.pixel_format
    }

    pub fn device(&self) -> &Arc<MTLDevice> {
        &self.device
    }

    pub fn is_framebuffer_created(&self) -> bool {
        self.is_framebuffer_created.load(Ordering::Relaxed)
    }
}

pub trait MTLViewArc {
    fn next_drawable(&self) -> Result<Arc<MTLDrawable>>;
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    fn metal_next_drawable(&self) -> Result<Arc<MTLDrawable>>;
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    fn vulkan_next_drawable(&self) -> Result<Arc<MTLDrawable>>;
}

impl MTLViewArc for Arc<MTLView> {
    fn next_drawable(&self) -> Result<Arc<MTLDrawable>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return self.metal_next_drawable();

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return self.vulkan_next_drawable();
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    fn vulkan_next_drawable(&self) -> Result<Arc<MTLDrawable>> {
        if self.is_framebuffer_created.load(Ordering::Relaxed) {
            return Ok(Arc::new(MTLDrawable {
                vulkan_drawable: VulkanMTLDrawable {
                    image_index: AtomicU32::new(0),
                },
                view: self.clone(),
            }));
        }

        let swapchain_khr = self.vulkan_view().swapchain().khr().borrow().to_owned();
        let sync_object = self.vulkan_view().swapchain().in_flight_frames().next();

        let image_available_event = &sync_object.image_available_event;
        let wait_fences = [*sync_object.fence.vulkan_fence()];

        unsafe {
            self.device()
                .vulkan_device()
                .logical()
                .wait_for_fences(&wait_fences, true, u64::MAX)?
        };

        let result = unsafe {
            self.vulkan_view()
                .swapchain()
                .instance()
                .acquire_next_image(
                    swapchain_khr,
                    u64::MAX,
                    *image_available_event.vulkan_semaphore(),
                    vk::Fence::null(),
                )
        };

        let image_index = match result {
            Ok((image_index, _)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => todo!("Handle Out of Date Swapchain"),
            Err(error) => {
                return Err(anyhow!(
                    "Vulkan Error while acquiring next drawable: {}",
                    error
                ));
            }
        };

        unsafe {
            self.device()
                .vulkan_device()
                .logical()
                .reset_fences(&wait_fences)?
        }

        Ok(Arc::new(MTLDrawable {
            vulkan_drawable: VulkanMTLDrawable {
                image_index: AtomicU32::new(image_index),
            },
            view: self.clone(),
        }))
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    fn metal_next_drawable(&self) -> Result<Arc<MTLDrawable>> {
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
    render_pass_index: AtomicU32,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanMTLView {
    pub fn swapchain(&self) -> &VulkanSwapchain {
        &self.swapchain
    }

    pub fn render_pass_index(&self) -> &AtomicU32 {
        &self.render_pass_index
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanSwapchain {
    instance: ash::khr::swapchain::Device,
    khr: RefCell<vk::SwapchainKHR>,
    framebuffers: RwLock<Vec<vk::Framebuffer>>,
    in_flight_frames: VulkanInFlightFrames,
    image_count: u32,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanSwapchain {
    pub fn in_flight_frames(&self) -> &VulkanInFlightFrames {
        &self.in_flight_frames
    }

    pub fn image_count(&self) -> &u32 {
        &self.image_count
    }

    pub fn khr(&self) -> &RefCell<vk::SwapchainKHR> {
        &self.khr
    }

    pub fn instance(&self) -> &ash::khr::swapchain::Device {
        &self.instance
    }

    pub fn framebuffers(&self) -> &RwLock<Vec<vk::Framebuffer>> {
        &self.framebuffers
    }

    pub fn create_framebuffers(&self, render_pass: &vk::RenderPass) {
        // let framebuffers = self.framebuffers().write().unwrap();

        todo!()
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
#[derive(Clone)]
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

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanInFlightFrames {
    sync_objects: Vec<VulkanSyncObject>,
    current_frame: AtomicUsize,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanInFlightFrames {
    pub fn next(&self) -> &VulkanSyncObject {
        let current_frame = self.current_frame.load(Ordering::Relaxed);

        self.current_frame.store(
            (current_frame + 1) % self.sync_objects.len(),
            Ordering::Relaxed,
        );

        &self.sync_objects[current_frame]
    }
}

pub enum MTLTarget {
    Drawable(Arc<MTLDrawable>),
    Texture(Arc<MTLTexture>),
}

impl MTLTarget {
    pub fn pixel_format(&self) -> &MTLPixelFormat {
        match self {
            Self::Drawable(d) => d.view().pixel_format(),
            Self::Texture(_t) => todo!("Handle Textures."),
        }
    }
}

pub enum MTLPixelFormat {
    Bgra8Unorm,
}

impl MTLPixelFormat {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn from_vulkan(vulkan_format: vk::Format) -> Self {
        match vulkan_format {
            vk::Format::B8G8R8A8_UNORM => Self::Bgra8Unorm,
            _ => todo!("Format not yet handled."),
        }
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn to_vulkan(&self) -> vk::Format {
        match self {
            Self::Bgra8Unorm => vk::Format::B8G8R8A8_UNORM,
            _ => todo!("Format not yet handled."),
        }
    }
}
