use crate::{BMLLayer, MTLDevice, device};
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
    MTLPixelFormat as MetalMTLPixelFormat, MTLRenderCommandEncoder as MetalMTLRenderCommandEncoder,
    MTLRenderPassColorAttachmentDescriptor as MetalMTLRenderPassColorAttachmentDescriptor,
    MTLRenderPassDescriptor as MetalMTLRenderPassDescriptor, MTLStoreAction as MetalMTLStoreAction,
    MTLTexture as MetalMTLTexture,
};

pub struct MTLTexture {
    device: Arc<MTLDevice>,
    pixel_format: MTLPixelFormat,
    width: u32,
    height: u32,
    depth: u32,

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    metal_texture: Option<Retained<ProtocolObject<dyn MetalMTLTexture>>>,
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    ca_metal_drawable: Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_image: vk::Image,
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_image_view: vk::ImageView,
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_framebuffer: RwLock<Option<vk::Framebuffer>>,
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_swapchain: Option<(Arc<ash::khr::swapchain::Device>, Arc<vk::SwapchainKHR>)>,
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_sync_object: RwLock<Option<VulkanSyncObject>>,
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_image_index: AtomicU32,
}

impl MTLTexture {
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn from_vulkan(
        device: Arc<MTLDevice>,
        vulkan_image: vk::Image,
        pixel_format: MTLPixelFormat,
        width: u32,
        height: u32,
        depth: u32,
        vulkan_swapchain_instance: Arc<ash::khr::swapchain::Device>,
        vulkan_swapchain_khr: Arc<vk::SwapchainKHR>,
    ) -> Result<Arc<Self>> {
        let vulkan_image_view = unsafe {
            device.vulkan_device().logical().create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(vulkan_image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(pixel_format.to_vulkan())
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    }),
                None,
            )?
        };

        Ok(Arc::new(Self {
            device,
            pixel_format,
            width,
            height,
            depth,
            vulkan_image,
            vulkan_image_view,
            vulkan_swapchain: Some((vulkan_swapchain_instance, vulkan_swapchain_khr)),
            vulkan_image_index: AtomicU32::new(0),
            vulkan_framebuffer: RwLock::new(None),
            vulkan_sync_object: RwLock::new(None),
        }))
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn pixel_format(&self) -> &MTLPixelFormat {
        &self.pixel_format
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_image(&self) -> &vk::Image {
        &self.vulkan_image
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_image_view(&self) -> &vk::ImageView {
        &self.vulkan_image_view
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_framebuffer(&self) -> &RwLock<Option<vk::Framebuffer>> {
        &self.vulkan_framebuffer
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_is_framebuffer(&self) -> bool {
        self.vulkan_framebuffer.read().unwrap().is_some()
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_swapchain(
        &self,
    ) -> &Option<(Arc<ash::khr::swapchain::Device>, Arc<vk::SwapchainKHR>)> {
        &self.vulkan_swapchain
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub unsafe fn vulkan_create_framebuffer(
        &self,
        render_pass: &vk::RenderPass,
        device: Arc<MTLDevice>,
    ) -> Result<()> {
        let mut framebuffer = self.vulkan_framebuffer().write().unwrap();

        framebuffer.replace(unsafe {
            device.vulkan_device().logical().create_framebuffer(
                &vk::FramebufferCreateInfo::default()
                    .render_pass(*render_pass)
                    .attachments(&[*self.vulkan_image_view()])
                    .width(self.width)
                    .height(self.height)
                    .layers(1),
                None,
            )?
        });

        Ok(())
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_image_index(&self) -> &AtomicU32 {
        &self.vulkan_image_index
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_sync_object(&self) -> &RwLock<Option<VulkanSyncObject>> {
        &self.vulkan_sync_object
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn from_metal(
        device: Arc<MTLDevice>,
        ca_metal_drawable: Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>,
        metal_texture: Option<Retained<ProtocolObject<dyn MetalMTLTexture>>>,
    ) -> Arc<Self> {
        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let mut depth: u32 = 0;
        let mut pixel_format: MTLPixelFormat = MTLPixelFormat::Bgra8Unorm;

        match &ca_metal_drawable {
            Some(d) => unsafe {
                let texture = d.texture();

                width = texture.width() as u32;
                height = texture.height() as u32;
                depth = texture.depth() as u32;
                pixel_format = MTLPixelFormat::from_metal(texture.pixelFormat());
            },
            None => {}
        }

        match &metal_texture {
            Some(t) => {
                width = t.width() as u32;
                height = t.height() as u32;
                depth = t.depth() as u32;
                pixel_format = MTLPixelFormat::from_metal(t.pixelFormat());
            }
            None => {}
        }

        Arc::new(Self {
            device,
            ca_metal_drawable,
            metal_texture,
            width,
            height,
            depth,
            pixel_format,
        })
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn to_metal(&self) -> Result<Retained<ProtocolObject<dyn MetalMTLTexture>>> {
        Ok(unsafe {
            match &self.ca_metal_drawable {
                Some(d) => d.texture(),
                None => match &self.metal_texture {
                    Some(t) => t.clone(),
                    None => return Err(anyhow!("No Metal Texture Found.")),
                },
            }
        })
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn ca_metal_drawable(&self) -> &Option<Retained<ProtocolObject<dyn CAMetalDrawable>>> {
        &self.ca_metal_drawable
    }
}

#[derive(Default)]
pub struct MTLViewSettings {
    pub vsync: AtomicBool,
}

pub struct MTLView {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    ca_metal_layer: Retained<CAMetalLayer>,

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    vulkan_view: VulkanMTLView,

    device: Arc<MTLDevice>,
    pixel_format: MTLPixelFormat,
}

impl MTLView {
    pub fn request(device: Arc<MTLDevice>, settings: Option<MTLViewSettings>) -> Result<Arc<Self>> {
        let settings = match settings {
            Some(s) => s,
            None => MTLViewSettings::default(),
        };

        let bml_layer = match device.instance.layer() {
            Some(l) => l,
            None => {
                return Err(anyhow!("Can't request on a headless instance."));
            }
        };

        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return Self::metal_request(bml_layer, &device);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return Self::vulkan_request(bml_layer, &device, settings);
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

        let pixel_format = unsafe { ca_metal_layer.pixelFormat() };

        Ok(Arc::new(Self {
            device: device.clone(),
            ca_metal_layer,
            pixel_format: MTLPixelFormat::from_metal(pixel_format),
        }))
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    pub fn vulkan_get_surface_details(
        surface: &VulkanSurface,
        device: &Arc<MTLDevice>,
        bml_layer: &BMLLayer,
        settings: &MTLViewSettings,
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

        let surface_present_mode = if present_modes.contains(&vk::PresentModeKHR::FIFO)
            && settings.vsync.load(Ordering::Relaxed)
        {
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
    pub fn vulkan_request(
        bml_layer: &BMLLayer,
        device: &Arc<MTLDevice>,
        settings: MTLViewSettings,
    ) -> Result<Arc<Self>> {
        //
        // =======================================================
        // TODO: This currently sets a lot of things by default.
        // But in the future all of this should be converted to
        // Metal types for finer granular control.
        // =======================================================
        //
        let surface = device.instance.vulkan_surface().as_ref().unwrap();

        let surface_details =
            Self::vulkan_get_surface_details(surface, device, bml_layer, &settings)?;

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

        let swapchain_instance = Arc::new(ash::khr::swapchain::Device::new(
            device.instance.vulkan_instance(),
            device.vulkan_device().logical(),
        ));

        let swapchain_khr =
            Arc::new(unsafe { swapchain_instance.create_swapchain(&swapchain_create_info, None)? });

        let swapchain_images =
            unsafe { swapchain_instance.get_swapchain_images(*swapchain_khr.as_ref())? };
        let mut textures: Vec<Arc<MTLTexture>> = vec![];
        for i in swapchain_images {
            textures.push(MTLTexture::from_vulkan(
                device.clone(),
                i,
                pixel_format,
                surface_details.extent.width,
                surface_details.extent.height,
                0,
                swapchain_instance.clone(),
                swapchain_khr.clone(),
            )?);
        }

        let in_flight_frames = Self::vulkan_create_in_flight_frames(&device)?;

        Ok(Arc::new(Self {
            vulkan_view: VulkanMTLView {
                swapchain: VulkanSwapchain {
                    surface_details,
                    instance: swapchain_instance,
                    khr: RwLock::new(swapchain_khr),
                    textures: RwLock::new(textures),
                    in_flight_frames,
                    image_count,
                },
                render_pass_index: AtomicU32::new(0),
            },
            device: device.clone(),
            pixel_format,
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
}

pub trait MTLViewArc {
    fn next_drawable(&self, device: Arc<MTLDevice>) -> Result<Arc<MTLTexture>>;
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    fn metal_next_drawable(&self, device: Arc<MTLDevice>) -> Result<Arc<MTLTexture>>;
    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    fn vulkan_next_drawable(&self) -> Result<Arc<MTLTexture>>;
}

impl MTLViewArc for Arc<MTLView> {
    fn next_drawable(&self, device: Arc<MTLDevice>) -> Result<Arc<MTLTexture>> {
        #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
        return self.metal_next_drawable(device);

        #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
        return self.vulkan_next_drawable();
    }

    #[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
    fn vulkan_next_drawable(&self) -> Result<Arc<MTLTexture>> {
        let swapchain_khr = self
            .vulkan_view()
            .swapchain()
            .khr()
            .read()
            .unwrap()
            .to_owned();
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
                    *swapchain_khr.as_ref(),
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

        let texture =
            self.vulkan_view().swapchain().textures.read().unwrap()[image_index as usize].clone();

        texture
            .vulkan_sync_object
            .write()
            .unwrap()
            .replace(sync_object.clone());

        texture
            .vulkan_image_index
            .store(image_index, Ordering::Relaxed);

        Ok(texture)
    }

    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    fn metal_next_drawable(&self, device: Arc<MTLDevice>) -> Result<Arc<MTLTexture>> {
        let ca_metal_drawable = unsafe { self.ca_metal_layer.nextDrawable() };

        let ca_metal_drawable = match ca_metal_drawable {
            Some(d) => d,
            None => {
                return Err(anyhow!(
                    "Failed to get the next `MTLDrawable` in the swapchain."
                ));
            }
        };

        Ok(MTLTexture::from_metal(
            device,
            Some(ca_metal_drawable),
            None,
        ))
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanMTLView {
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
    surface_details: VulkanSurfaceDetails,
    instance: Arc<ash::khr::swapchain::Device>,
    khr: RwLock<Arc<vk::SwapchainKHR>>,
    textures: RwLock<Vec<Arc<MTLTexture>>>,
    in_flight_frames: VulkanInFlightFrames,
    image_count: u32,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanSwapchain {
    pub fn surface_details(&self) -> &VulkanSurfaceDetails {
        &self.surface_details
    }

    pub fn in_flight_frames(&self) -> &VulkanInFlightFrames {
        &self.in_flight_frames
    }

    pub fn image_count(&self) -> &u32 {
        &self.image_count
    }

    pub fn khr(&self) -> &RwLock<Arc<vk::SwapchainKHR>> {
        &self.khr
    }

    pub fn instance(&self) -> &ash::khr::swapchain::Device {
        &self.instance
    }

    pub fn textures(&self) -> &RwLock<Vec<Arc<MTLTexture>>> {
        &self.textures
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
impl VulkanSyncObject {
    pub fn image_available_event(&self) -> &Arc<MTLEvent> {
        &self.image_available_event
    }

    pub fn render_finished_event(&self) -> &Arc<MTLEvent> {
        &self.render_finished_event
    }

    pub fn fence(&self) -> &Arc<MTLFence> {
        &self.fence
    }
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
pub struct VulkanSurfaceDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
    extent: vk::Extent2D,
}

#[cfg(any(not(any(target_os = "macos", target_os = "ios")), feature = "moltenvk"))]
impl VulkanSurfaceDetails {
    pub fn extent(&self) -> &vk::Extent2D {
        &self.extent
    }
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

#[derive(Clone, Copy)]
pub enum MTLPixelFormat {
    Bgra8Unorm,
}

impl MTLPixelFormat {
    #[cfg(all(any(target_os = "macos", target_os = "ios"), not(feature = "moltenvk")))]
    pub fn from_metal(metal_format: MetalMTLPixelFormat) -> Self {
        match metal_format {
            MetalMTLPixelFormat::BGRA8Unorm => MTLPixelFormat::Bgra8Unorm,
            _ => todo!("Format not yet handled."),
        }
    }

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
