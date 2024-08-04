use super::{VkContext, VkDeviceContext};
use ash::vk;
use ash::vk::{Fence, Semaphore};

pub struct SwapChain {
    pub swap_chain_fn: Option<ash::khr::swapchain::Device>,
    pub swap_chain: Option<vk::SwapchainKHR>,

    pub format: vk::Format,
    pub color_space: vk::ColorSpaceKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl SwapChain {
    pub fn new(context: &VkContext, device_context: &VkDeviceContext) -> Self {
        unsafe {
            let (swap_chain_fn, swap_chain, surface_format, present_mode, extent) =
                Self::create_swap_chain(&context, device_context);
            //delay
            let (images, image_views) = Self::get_swap_chain_images(
                device_context,
                &swap_chain_fn,
                swap_chain,
                surface_format.format,
            );

            Self {
                swap_chain_fn: Some(swap_chain_fn),
                swap_chain: Some(swap_chain),

                extent,
                format: surface_format.format,
                color_space: surface_format.color_space,
                present_mode,
                images,
                image_views,
            }
        }
    }

    pub fn acquire_image(
        &self,
        timeout: u64,
        semaphore: Option<Semaphore>,
        fence: Option<Fence>,
    ) -> u32 {
        unsafe {
            let acquire_result = self.swap_chain_fn.as_ref().unwrap().acquire_next_image(
                self.swap_chain.unwrap(),
                timeout,
                semaphore.unwrap_or_default(),
                fence.unwrap_or_default(),
            );

            let (image_index, _) = match acquire_result {
                Ok(result) => result,
                Err(err_code) => {
                    if err_code == vk::Result::ERROR_OUT_OF_DATE_KHR {
                        // self.recreate_swap_chain();
                        // return;
                    }
                    panic!("failed to acquire swap chain image!");
                }
            };

            image_index
        }
    }

    pub(crate) unsafe fn query_surface_support(
        context: &VkContext,
        physical_device: vk::PhysicalDevice,
    ) -> (
        vk::SurfaceCapabilitiesKHR,
        Vec<vk::SurfaceFormatKHR>,
        Vec<vk::PresentModeKHR>,
    ) {
        let surface_fn = context.surface_fn.as_ref().unwrap();
        let surface = context.surface.unwrap();

        let capabilities = surface_fn
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap();
        let formats = surface_fn
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap();
        let present_modes = surface_fn
            .get_physical_device_surface_present_modes(physical_device, surface)
            .unwrap();

        (capabilities, formats, present_modes)
    }

    unsafe fn create_swap_chain(
        context: &VkContext,
        device: &VkDeviceContext,
    ) -> (
        ash::khr::swapchain::Device,
        vk::SwapchainKHR,
        vk::SurfaceFormatKHR,
        vk::PresentModeKHR,
        vk::Extent2D,
    ) {
        let (surface_capabilities, surface_formats, surface_present_modes) =
            Self::query_surface_support(context, device.physical_device);

        let surface_format = Self::choose_surface_format(&surface_formats);
        let present_mode = Self::choose_surface_present_mode(&surface_present_modes);
        let extent = Self::choose_surface_extent(context, &surface_capabilities);

        let image_count = (surface_capabilities.min_image_count + 1).clamp(
            surface_capabilities.min_image_count,
            surface_capabilities.max_image_count,
        );

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(context.surface.unwrap())
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);
        // .old_swapchain(None)

        if device.graphic_queue_family == device.present_queue_family {
            create_info.image_sharing_mode = vk::SharingMode::EXCLUSIVE;
            create_info.queue_family_index_count = 0;
            create_info.p_queue_family_indices = std::ptr::null();
        } else {
            create_info.image_sharing_mode = vk::SharingMode::CONCURRENT;
            create_info.queue_family_index_count = 2;
            create_info.p_queue_family_indices = [
                device.graphic_queue_family.unwrap(),
                device.present_queue_family.unwrap(),
            ]
            .as_ptr();
        }

        let swap_chain_fn = ash::khr::swapchain::Device::new(&context.instance, &device.device);
        let swap_chain = swap_chain_fn
            .create_swapchain(&create_info, None)
            .expect("failed to create swap chain!");

        (
            swap_chain_fn,
            swap_chain,
            surface_format,
            present_mode,
            extent,
        )
    }

    unsafe fn get_swap_chain_images(
        device: &VkDeviceContext,
        swap_chain_fn: &ash::khr::swapchain::Device,
        swap_chain: vk::SwapchainKHR,
        format: vk::Format,
    ) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
        let images = swap_chain_fn
            .get_swapchain_images(swap_chain)
            .expect("failed to get swap chain images!");

        let image_views = images
            .iter()
            .cloned()
            .map(|image| device.create_image_view(image, format, vk::ImageAspectFlags::COLOR, 1))
            .collect::<Vec<_>>();

        (images, image_views)
    }

    fn choose_surface_format(surface_formats: &Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
        surface_formats
            .iter()
            .cloned()
            .find(|&format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(surface_formats[0])
    }

    fn choose_surface_present_mode(present_modes: &Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
        // VK_PRESENT_MODE_IMMEDIATE_KHR: Images submitted by your application are transferred to the screen right away, which may result in tearing.
        // VK_PRESENT_MODE_FIFO_KHR: The swap chain is a queue where the display takes an image from the front of the queue when the display is refreshed
        //  and the program inserts rendered images at the back of the queue. If the queue is full then the program has to wait. This is most similar to
        //  vertical sync as found in modern games. The moment that the display is refreshed is known as "vertical blank".
        // VK_PRESENT_MODE_FIFO_RELAXED_KHR: This mode only differs from the previous one if the application is late and the queue was empty at the last
        //  vertical blank. Instead of waiting for the next vertical blank, the image is transferred right away when it finally arrives. This may result
        //  in visible tearing.
        // VK_PRESENT_MODE_MAILBOX_KHR: This is another variation of the second mode. Instead of blocking the application when the queue is full, the
        //  images that are already queued are simply replaced with the newer ones. This mode can be used to render frames as fast as possible while
        //  still avoiding tearing, resulting in fewer latency issues than standard vertical sync. This is commonly known as "triple buffering",
        //  although the existence of three buffers alone does not necessarily mean that the framerate is unlocked.
        present_modes
            .iter()
            .cloned()
            .find(|&present_mode| present_mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO)
    }

    fn choose_surface_extent(
        context: &VkContext,
        capabilities: &vk::SurfaceCapabilitiesKHR,
    ) -> vk::Extent2D {
        match capabilities.current_extent.width {
            u32::MAX => {
                let inner_size = context.window.inner_size();
                vk::Extent2D {
                    width: inner_size.width.clamp(
                        capabilities.min_image_extent.width,
                        capabilities.max_image_extent.width,
                    ),
                    height: inner_size.height.clamp(
                        capabilities.min_image_extent.height,
                        capabilities.max_image_extent.height,
                    ),
                }
            }
            _ => capabilities.current_extent,
        }
    }
}
