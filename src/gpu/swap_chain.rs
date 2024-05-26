use crate::gpu;
use ash::{Entry, vk};
use std::rc::Rc;
use winit::window::Window;

pub struct SwapChain {
    device: Rc<gpu::Device>,
    pub swap_chain_fn: ash::khr::swapchain::Device,
    pub swap_chain: vk::SwapchainKHR,
    pub format: vk::Format,
    pub color_space: vk::ColorSpaceKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,

    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl SwapChain {
    pub fn new(
        instance: &ash::Instance,
        window: &Window,
        device: Rc<gpu::Device>,
        surface: vk::SurfaceKHR,
    ) -> Self {
        unsafe {
            let (swap_chain_loader, swap_chain, surface_format, present_mode, extent) =
                Self::create_swap_chain(&instance, &window, &device, surface);
            let (images, image_views) = Self::get_swap_chain_images(
                &device,
                &swap_chain_loader,
                swap_chain,
                surface_format.format,
            );

            Self {
                device,
                swap_chain_fn: swap_chain_loader,
                swap_chain,
                format: surface_format.format,
                color_space: surface_format.color_space,
                present_mode,
                extent,

                images,
                image_views,
            }
        }
    }

    unsafe fn create_swap_chain(
        instance: &ash::Instance,
        window: &Window,
        device: &gpu::Device,
        surface: vk::SurfaceKHR,
    ) -> (
        ash::khr::swapchain::Device,
        vk::SwapchainKHR,
        vk::SurfaceFormatKHR,
        vk::PresentModeKHR,
        vk::Extent2D,
    ) {
        let surface_format = Self::choose_surface_format(&device.surface_formats);
        let present_mode = Self::choose_surface_present_mode(&device.surface_present_modes);
        let extent = Self::choose_surface_extent(&device.surface_capabilities, &window);

        let image_count = (device.surface_capabilities.min_image_count + 1).clamp(
            device.surface_capabilities.min_image_count,
            device.surface_capabilities.max_image_count,
        );

        let pre_transform = if device
            .surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            device.surface_capabilities.current_transform
        };

        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
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
        
        let swap_chain_loader = ash::khr::swapchain::Device::new(&instance, &device.device);
        let swap_chain = swap_chain_loader
            .create_swapchain(&create_info, None)
            .expect("failed to create swap chain!");

        (
            swap_chain_loader,
            swap_chain,
            surface_format,
            present_mode,
            extent,
        )
    }

    unsafe fn get_swap_chain_images(
        device: &gpu::Device,
        swap_chain_loader: &ash::khr::swapchain::Device,
        swap_chain: vk::SwapchainKHR,
        format: vk::Format,
    ) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
        let images = swap_chain_loader
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
        capabilities: &vk::SurfaceCapabilitiesKHR,
        window: &Window,
    ) -> vk::Extent2D {
        match capabilities.current_extent.width {
            u32::MAX => {
                let inner_size = window.inner_size();
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

impl Drop for SwapChain {
    fn drop(&mut self) {
        unsafe {
            for &image_view in self.image_views.iter() {
                self.device.device.destroy_image_view(image_view, None);
            }
            self.swap_chain_fn.destroy_swapchain(self.swap_chain, None);
        }
    }
}
