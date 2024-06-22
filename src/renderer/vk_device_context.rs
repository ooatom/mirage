use super::VkContext;
use ash::vk;
use std::collections::{BTreeMap, HashSet};
use std::ffi::CStr;
use std::rc::{Rc, Weak};
use crate::renderer::utils::create_image_view;

const DEVICE_EXTENSIONS: &[&CStr] = &[
    // The Vulkan spec states: If the VK_KHR_portability_subset extension is included in pProperties
    // of vkEnumerateDeviceExtensionProperties, ppEnabledExtensionNames must include "VK_KHR_portability_subset"
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    vk::KHR_PORTABILITY_SUBSET_NAME,
    vk::KHR_SWAPCHAIN_NAME,
    // vk::ExtShaderAtomicFloatFn::name()
];

pub struct VkDeviceContext {
    context: Rc<VkContext>,

    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub swap_chain_fn: Option<ash::khr::swapchain::Device>,
    pub swap_chain: Option<vk::SwapchainKHR>,

    pub physical_device_properties: vk::PhysicalDeviceProperties,
    pub physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub graphic_queue_family: Option<u32>,
    pub present_queue_family: Option<u32>,
    pub compute_queue_family: Option<u32>,
    pub graphic_queue: Option<vk::Queue>,
    pub present_queue: Option<vk::Queue>,
    pub compute_queue: Option<vk::Queue>,

    pub msaa_samples: vk::SampleCountFlags,
    pub format: vk::Format,
    pub color_space: vk::ColorSpaceKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl VkDeviceContext {
    pub fn new(context: Rc<VkContext>) -> Self {
        unsafe {
            let physical_device = Self::pick_physical_device(&context);
            let physical_device_properties = context
                .instance
                .get_physical_device_properties(physical_device);
            let physical_device_memory_properties = context
                .instance
                .get_physical_device_memory_properties(physical_device);

            let msaa_samples = Self::get_max_usable_sample_count(&physical_device_properties);

            let (graphic_queue_family, present_queue_family, compute_queue_family) =
                Self::find_queue_families(&context, physical_device);
            let (device, graphic_queue, present_queue, compute_queue) = Self::create_logical_device(
                &context,
                physical_device,
                graphic_queue_family,
                present_queue_family,
                compute_queue_family,
            );

            let (swap_chain_fn, swap_chain, surface_format, present_mode, extent) =
                Self::create_swap_chain(
                    &context,
                    &device,
                    physical_device,
                    graphic_queue_family,
                    present_queue_family,
                    compute_queue_family,
                );
            //delay
            let (images, image_views) = Self::get_swap_chain_images(
                &device,
                &swap_chain_fn,
                swap_chain,
                surface_format.format,
            );

            Self {
                context,
                physical_device,
                device,
                swap_chain_fn: Some(swap_chain_fn),
                swap_chain: Some(swap_chain),
                physical_device_properties,
                physical_device_memory_properties,

                graphic_queue_family,
                present_queue_family,
                compute_queue_family,
                graphic_queue,
                present_queue,
                compute_queue,

                extent,
                format: surface_format.format,
                color_space: surface_format.color_space,
                present_mode,
                images,
                image_views,
                msaa_samples,
            }
        }
    }

    pub unsafe fn create_shader_module(&self, code: &[u32]) -> vk::ShaderModule {
        let create_info = vk::ShaderModuleCreateInfo::default().code(code);

        self.device
            .create_shader_module(&create_info, None)
            .expect("failed to create shader module!")
    }

    pub unsafe fn get_format_properties(&self, format: vk::Format) -> vk::FormatProperties {
        self.context
            .instance
            .get_physical_device_format_properties(self.physical_device, format)
    }

    unsafe fn create_logical_device(
        context: &VkContext,
        physical_device: vk::PhysicalDevice,
        graphic_queue_family: Option<u32>,
        present_queue_family: Option<u32>,
        compute_queue_family: Option<u32>,
    ) -> (
        ash::Device,
        Option<vk::Queue>,
        Option<vk::Queue>,
        Option<vk::Queue>,
    ) {
        let queue_families = [
            graphic_queue_family,
            present_queue_family,
            compute_queue_family,
        ]
        .iter()
        .filter(|family| family.is_some())
        .map(|family| family.unwrap())
        .collect::<HashSet<_>>();

        let mut queue_infos: Vec<vk::DeviceQueueCreateInfo> = vec![];
        queue_families.into_iter().for_each(|family_index| {
            let info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family_index)
                .queue_priorities(&[1.0]);

            queue_infos.push(info);
        });

        let features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .sample_rate_shading(true);

        let extension_names = DEVICE_EXTENSIONS
            .iter()
            .cloned()
            .map(|extension| extension.as_ptr())
            .collect::<Vec<_>>();

        let create_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&extension_names)
            .enabled_features(&features)
            .queue_create_infos(&queue_infos);

        let device = context
            .instance
            .create_device(physical_device, &create_info, None)
            .expect("failed to create logical device!");

        let graphic_queue = if let Some(queue_family) = graphic_queue_family {
            Some(device.get_device_queue(queue_family, 0))
        } else {
            None
        };

        let present_queue = if graphic_queue_family == present_queue_family {
            graphic_queue
        } else if let Some(queue_family) = present_queue_family {
            Some(device.get_device_queue(queue_family, 0))
        } else {
            None
        };

        let compute_queue = if let Some(queue_family) = compute_queue_family {
            Some(device.get_device_queue(queue_family, 0))
        } else {
            None
        };

        (device, graphic_queue, present_queue, compute_queue)
    }

    unsafe fn pick_physical_device(context: &VkContext) -> vk::PhysicalDevice {
        let physical_devices = context
            .instance
            .enumerate_physical_devices()
            .expect("failed to find GPUs with vulkan support!");

        let score_map: BTreeMap<u32, vk::PhysicalDevice> = physical_devices
            .into_iter()
            .map(|physical_device| {
                (
                    Self::rate_physical_device_suitability(context, physical_device),
                    physical_device,
                )
            })
            .collect();

        match score_map.first_key_value() {
            Some((count, physical_device)) if *count > 0 => *physical_device,
            _ => panic!("failed to find a suitable device!"),
        }
    }

    unsafe fn rate_physical_device_suitability(
        context: &VkContext,
        physical_device: vk::PhysicalDevice,
    ) -> u32 {
        let mut score = 0;
        let properties = context
            .instance
            .get_physical_device_properties(physical_device);
        let features = context
            .instance
            .get_physical_device_features(physical_device);

        match properties.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => score += 10000,
            vk::PhysicalDeviceType::INTEGRATED_GPU => score += 1000,
            vk::PhysicalDeviceType::VIRTUAL_GPU => score += 100,
            vk::PhysicalDeviceType::CPU => score += 10,
            _ => (),
        }

        score += properties.limits.max_image_dimension2_d;

        let (graphic_queue_family, present_queue_family, compute_queue_family) =
            Self::find_queue_families(context, physical_device);

        if graphic_queue_family.is_none()
            || present_queue_family.is_none()
            || compute_queue_family.is_none()
            || !Self::check_device_extension_support(&context.instance, physical_device)
            || features.sampler_anisotropy == vk::FALSE
        {
            score = 0;
        } else {
            let (_, formats, present_modes) = Self::query_surface_support(context, physical_device);
            if formats.is_empty() || present_modes.is_empty() {
                score = 0;
            }
        }

        return score;
    }

    unsafe fn find_queue_families(
        context: &VkContext,
        physical_device: vk::PhysicalDevice,
    ) -> (Option<u32>, Option<u32>, Option<u32>) {
        let mut graphic_queue_family: Option<u32> = None;
        let mut present_queue_family: Option<u32> = None;
        let mut compute_queue_family: Option<u32> = None;

        let properties = context
            .instance
            .get_physical_device_queue_family_properties(physical_device);

        // Any queue family with VK_QUEUE_GRAPHICS_BIT or VK_QUEUE_COMPUTE_BIT capabilities already implicitly support VK_QUEUE_TRANSFER_BIT operations.
        for (index, property) in properties.iter().enumerate() {
            if property.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                if graphic_queue_family.is_none() {
                    graphic_queue_family = Some(index as u32);
                }

                let is_support_surface = context
                    .surface_fn
                    .as_ref()
                    .unwrap()
                    .get_physical_device_surface_support(
                        physical_device,
                        index as u32,
                        context.surface.unwrap(),
                    )
                    .unwrap();

                if is_support_surface {
                    graphic_queue_family = Some(index as u32);
                    present_queue_family = Some(index as u32);
                    break;
                }
            }
        }

        if present_queue_family.is_none() {
            for (index, _property) in properties.iter().enumerate() {
                let is_support_surface = context
                    .surface_fn
                    .as_ref()
                    .unwrap()
                    .get_physical_device_surface_support(
                        physical_device,
                        index as u32,
                        context.surface.unwrap(),
                    )
                    .unwrap();

                if is_support_surface {
                    present_queue_family = Some(index as u32);
                    break;
                }
            }
        }

        for (index, property) in properties.iter().enumerate() {
            if property.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                if compute_queue_family.is_none() {
                    compute_queue_family = Some(index as u32);
                }

                if compute_queue_family != graphic_queue_family {
                    compute_queue_family = Some(index as u32);
                    break;
                }
            }
        }

        (
            graphic_queue_family,
            present_queue_family,
            compute_queue_family,
        )
    }

    unsafe fn check_device_extension_support(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> bool {
        let supported_extensions = instance
            .enumerate_device_extension_properties(physical_device)
            .unwrap()
            .iter()
            .map(|extension| unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) })
            .collect::<Vec<_>>();

        DEVICE_EXTENSIONS
            .iter()
            .all(|extension| supported_extensions.contains(extension))
    }

    unsafe fn query_surface_support(
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

    unsafe fn get_max_usable_sample_count(
        properties: &vk::PhysicalDeviceProperties,
    ) -> vk::SampleCountFlags {
        let count = properties.limits.sampled_image_color_sample_counts
            & properties.limits.sampled_image_depth_sample_counts;

        match count {
            _ if count.contains(vk::SampleCountFlags::TYPE_64) => vk::SampleCountFlags::TYPE_64,
            _ if count.contains(vk::SampleCountFlags::TYPE_32) => vk::SampleCountFlags::TYPE_32,
            _ if count.contains(vk::SampleCountFlags::TYPE_16) => vk::SampleCountFlags::TYPE_16,
            _ if count.contains(vk::SampleCountFlags::TYPE_8) => vk::SampleCountFlags::TYPE_8,
            _ if count.contains(vk::SampleCountFlags::TYPE_4) => vk::SampleCountFlags::TYPE_4,
            _ if count.contains(vk::SampleCountFlags::TYPE_2) => vk::SampleCountFlags::TYPE_2,
            _ => vk::SampleCountFlags::TYPE_1,
        }
    }

    unsafe fn create_swap_chain(
        context: &VkContext,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        graphic_queue_family: Option<u32>,
        present_queue_family: Option<u32>,
        compute_queue_family: Option<u32>,
    ) -> (
        ash::khr::swapchain::Device,
        vk::SwapchainKHR,
        vk::SurfaceFormatKHR,
        vk::PresentModeKHR,
        vk::Extent2D,
    ) {
        let (surface_capabilities, surface_formats, surface_present_modes) =
            Self::query_surface_support(context, physical_device);

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

        if graphic_queue_family == present_queue_family {
            create_info.image_sharing_mode = vk::SharingMode::EXCLUSIVE;
            create_info.queue_family_index_count = 0;
            create_info.p_queue_family_indices = std::ptr::null();
        } else {
            create_info.image_sharing_mode = vk::SharingMode::CONCURRENT;
            create_info.queue_family_index_count = 2;
            create_info.p_queue_family_indices =
                [graphic_queue_family.unwrap(), present_queue_family.unwrap()].as_ptr();
        }

        let swap_chain_fn = ash::khr::swapchain::Device::new(&context.instance, &device);
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
        device: &ash::Device,
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
            .map(|image| {
                create_image_view(device, image, format, vk::ImageAspectFlags::COLOR, 1)
            })
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

impl Drop for VkDeviceContext {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            for &image_view in self.image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swap_chain_fn.as_ref().unwrap().destroy_swapchain(self.swap_chain.unwrap(), None);
            self.device.destroy_device(None);
        }
    }
}
