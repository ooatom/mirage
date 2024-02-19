use ash::vk;
use std::collections::{BTreeMap, HashSet};
use std::ffi::CStr;
use std::rc::Rc;

const DEVICE_EXTENSIONS: &[&CStr] = &[
    // The Vulkan spec states: If the VK_KHR_portability_subset extension is included in pProperties
    // of vkEnumerateDeviceExtensionProperties, ppEnabledExtensionNames must include "VK_KHR_portability_subset"
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    vk::KhrPortabilitySubsetFn::name(),
    vk::KhrSwapchainFn::name(),
    // vk::ExtShaderAtomicFloatFn::name()
];

pub struct Device {
    instance: Rc<ash::Instance>,
    pub physical_device: vk::PhysicalDevice,
    pub physical_device_properties: vk::PhysicalDeviceProperties,
    pub physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub device: ash::Device,

    pub graphic_queue_family: Option<u32>,
    pub present_queue_family: Option<u32>,
    pub compute_queue_family: Option<u32>,
    pub graphic_queue: Option<vk::Queue>,
    pub present_queue: Option<vk::Queue>,
    pub compute_queue: Option<vk::Queue>,

    pub surface_capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub surface_present_modes: Vec<vk::PresentModeKHR>,

    pub msaa_samples: vk::SampleCountFlags,
}

impl Device {
    pub fn new(
        instance: Rc<ash::Instance>,
        surface_loader: &ash::extensions::khr::Surface,
        surface: vk::SurfaceKHR,
    ) -> Self {
        unsafe {
            let physical_device = Device::pick_physical_device(&instance, &surface_loader, surface);
            let physical_device_properties =
                instance.get_physical_device_properties(physical_device);
            let physical_device_memory_properties =
                instance.get_physical_device_memory_properties(physical_device);

            let msaa_samples = Self::get_max_usable_sample_count(&physical_device_properties);

            let (graphic_queue_family, present_queue_family, compute_queue_family) =
                Self::find_queue_families(&instance, &surface_loader, surface, physical_device);
            let (device, graphic_queue, present_queue, compute_queue) = Self::create_logical_device(
                &instance,
                physical_device,
                graphic_queue_family,
                present_queue_family,
                compute_queue_family,
            );
            let (surface_capabilities, surface_formats, surface_present_modes) =
                Self::query_surface_support(&surface_loader, surface, physical_device);

            Self {
                instance,
                physical_device,
                physical_device_properties,
                physical_device_memory_properties,
                device,
                graphic_queue_family,
                present_queue_family,
                compute_queue_family,
                graphic_queue,
                present_queue,
                compute_queue,
                surface_capabilities,
                surface_formats,
                surface_present_modes,
                msaa_samples,
            }
        }
    }

    pub unsafe fn create_image(
        &self,
        width: u32,
        height: u32,
        mip_levels: u32,
        samples: vk::SampleCountFlags,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
    ) -> (vk::Image, vk::DeviceMemory) {
        // https://www.reddit.com/r/vulkan/comments/48cvzq/image_layouts/
        // Image tiling is the addressing layout of texels within an image. This is currently opaque, and it is not defined when you access it using the CPU.
        // The reason GPUs like image tiling to be "OPTIMAL" is for texel filtering. Consider a simple linear filter, the resulting value will have four texels contributing from a 2x2 quad.
        // If the texels were in "LINEAR" tiling, the two texels on the lower row would be very far away in memory from the two texels on the upper row.
        // In "OPTIMAL" tiling texel addresses are closer based on x and y distance.
        //
        // Image layouts are likely (though they don't have to be) used for internal transparent compression of images when in use by the GPU.
        // This is NOT a lossy block compressed format, it is an internal format that is used by the GPU to save bandwidth! It is unlikely there will be a "standard" compression format that can be exposed to the CPU.
        // The reason you need to transition your images from one layout to another is some hardware may only be able to access the compressed data from certain hardware blocks.
        // As a not-real example, imagine I could render to this compressed format and sample to it, but I could not perform image writes to it -
        // if you keep the image in IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL or IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL the driver knows that it can keep the image compressed and the GPU gets a big win.
        // If you transition the image to IMAGE_LAYOUT_GENERAL the driver cannot guarantee the image can be compressed and may have to decompress it in place.

        let create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .format(format)
            .mip_levels(mip_levels)
            .array_layers(1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            // .queue_family_indices()
            // VK_IMAGE_TILING_LINEAR: Texels are laid out in row-major order like our pixels array
            // VK_IMAGE_TILING_OPTIMAL: Texels are laid out in an implementation defined order for optimal access
            .tiling(tiling)
            // VK_IMAGE_LAYOUT_UNDEFINED: Not usable by the GPU and the very first transition will discard the texels.
            // VK_IMAGE_LAYOUT_PREINITIALIZED: Not usable by the GPU, but the first transition will preserve the texels.
            //      One example, however, would be if you wanted to use an image as a staging image in combination with the VK_IMAGE_TILING_LINEAR layout.
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(samples)
            // There are some optional flags for images that are related to sparse images. Sparse images are images where only certain regions are actually backed by memory.
            // If you were using a 3D texture for a voxel terrain, for example, then you could use this to avoid allocating memory to store large volumes of "air" values.
            // .flags()
            .build();

        let image = self
            .device
            .create_image(&create_info, None)
            .expect("failed to create image!");

        let memory_requirements = self.device.get_image_memory_requirements(image);

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: self
                .find_memory_type_index(memory_requirements.memory_type_bits, memory_properties),
            ..Default::default()
        };

        let image_memory = self
            .device
            .allocate_memory(&allocate_info, None)
            .expect("failed to allocate memory!");
        self.device
            .bind_image_memory(image, image_memory, 0)
            .expect("failed to bind image memory!");

        (image, image_memory)
    }

    pub unsafe fn create_image_view(
        &self,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
        mips: u32,
    ) -> vk::ImageView {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                // https://github.com/KhronosGroup/Vulkan-Guide/blob/main/chapters/formats.adoc
                // The VkImageAspectFlagBits values are used to represent which part of the data is being accessed
                // for operations such as clears and copies.
                // Color - Format with a R, G, B or A component and accessed with the VK_IMAGE_ASPECT_COLOR_BIT
                // Depth and Stencil - Formats with a D or S component. These formats are considered opaque and have
                // special rules when it comes to copy to and from depth/stencil images.
                // Some formats have both a depth and stencil component and can be accessed separately with
                // VK_IMAGE_ASPECT_DEPTH_BIT and VK_IMAGE_ASPECT_STENCIL_BIT.
                aspect_mask: aspect_flags,
                base_array_layer: 0,
                layer_count: 1,
                base_mip_level: 0,
                level_count: mips,
            })
            .build();

        self.device
            .create_image_view(&create_info, None)
            .expect("failed to create image view!")
    }

    pub unsafe fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
    ) -> (vk::Buffer, vk::DeviceMemory, vk::DeviceSize) {
        let create_info = vk::BufferCreateInfo::builder()
            // The flags parameter is used to configure sparse buffer memory,
            // which is not relevant right now. We'll leave it at the default value of 0.
            // .flags()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        let buffer = self
            .device
            .create_buffer(&create_info, None)
            .expect("failed to create buffer!");

        let requirements = self.device.get_buffer_memory_requirements(buffer);
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(
                self.find_memory_type_index(requirements.memory_type_bits, memory_properties),
            )
            .build();

        let buffer_memory = self
            .device
            .allocate_memory(&allocate_info, None)
            .expect("failed to allocate memory!");

        // If the offset is non-zero, then it is required to be divisible by memRequirements.alignment.
        self.device
            .bind_buffer_memory(buffer, buffer_memory, 0)
            .expect("failed to bind buffer memory!");

        (buffer, buffer_memory, requirements.size)
    }

    pub unsafe fn create_shader_module(&self, code: &[u32]) -> vk::ShaderModule {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(code).build();

        self.device
            .create_shader_module(&create_info, None)
            .expect("failed to create shader module!")
    }

    pub unsafe fn get_format_properties(&self, format: vk::Format) -> vk::FormatProperties {
        self.instance
            .get_physical_device_format_properties(self.physical_device, format)
    }

    fn find_memory_type_index(
        &self,
        type_bits: u32,
        property_flags: vk::MemoryPropertyFlags,
    ) -> u32 {
        for i in 0..self.physical_device_memory_properties.memory_type_count {
            if type_bits & (1 << i) == 0 {
                continue;
            }
            if !self.physical_device_memory_properties.memory_types[i as usize]
                .property_flags
                .contains(property_flags)
            {
                continue;
            }

            return i;
        }

        panic!("failed to find suitable memory type!")
    }

    unsafe fn create_logical_device(
        instance: &ash::Instance,
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
            let info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(family_index)
                .queue_priorities(&[1.0])
                .build();

            queue_infos.push(info);
        });

        let features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .sample_rate_shading(true)
            .build();

        let extension_names = DEVICE_EXTENSIONS
            .iter()
            .cloned()
            .map(|extension| extension.as_ptr())
            .collect::<Vec<_>>();

        let create_info = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(&extension_names)
            .enabled_features(&features)
            .queue_create_infos(&queue_infos)
            .build();

        let device = instance
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

    unsafe fn pick_physical_device(
        instance: &ash::Instance,
        surface_loader: &ash::extensions::khr::Surface,
        surface: vk::SurfaceKHR,
    ) -> vk::PhysicalDevice {
        let physical_devices = instance
            .enumerate_physical_devices()
            .expect("failed to find GPUs with vulkan support!");

        let point_map: BTreeMap<u32, vk::PhysicalDevice> = physical_devices
            .into_iter()
            .map(|physical_device| {
                (
                    Self::rate_physical_device_suitability(
                        &instance,
                        &surface_loader,
                        surface,
                        physical_device,
                    ),
                    physical_device,
                )
            })
            .collect();

        match point_map.first_key_value() {
            Some((count, physical_device)) if *count > 0 => *physical_device,
            _ => panic!("failed to find a suitable device!"),
        }
    }

    unsafe fn rate_physical_device_suitability(
        instance: &ash::Instance,
        surface_loader: &ash::extensions::khr::Surface,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> u32 {
        let mut score = 0;
        let properties = instance.get_physical_device_properties(physical_device);
        let features = instance.get_physical_device_features(physical_device);

        match properties.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => score += 10000,
            vk::PhysicalDeviceType::INTEGRATED_GPU => score += 1000,
            vk::PhysicalDeviceType::VIRTUAL_GPU => score += 100,
            vk::PhysicalDeviceType::CPU => score += 10,
            _ => (),
        }

        score += properties.limits.max_image_dimension2_d;

        let (graphic_queue_family, present_queue_family, compute_queue_family) =
            Self::find_queue_families(&instance, &surface_loader, surface, physical_device);

        if graphic_queue_family.is_none()
            || present_queue_family.is_none()
            || compute_queue_family.is_none()
            || !Self::check_device_extension_support(&instance, physical_device)
            || features.sampler_anisotropy == vk::FALSE
        {
            score = 0;
        } else {
            let (_, formats, present_modes) =
                Self::query_surface_support(&surface_loader, surface, physical_device);
            if formats.is_empty() || present_modes.is_empty() {
                score = 0;
            }
        }

        return score;
    }

    unsafe fn find_queue_families(
        instance: &ash::Instance,
        surface_loader: &ash::extensions::khr::Surface,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> (Option<u32>, Option<u32>, Option<u32>) {
        let mut graphic_queue_family: Option<u32> = None;
        let mut present_queue_family: Option<u32> = None;
        let mut compute_queue_family: Option<u32> = None;

        let properties = instance.get_physical_device_queue_family_properties(physical_device);

        // Any queue family with VK_QUEUE_GRAPHICS_BIT or VK_QUEUE_COMPUTE_BIT capabilities already implicitly support VK_QUEUE_TRANSFER_BIT operations.
        for (index, property) in properties.iter().enumerate() {
            if property.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                if graphic_queue_family.is_none() {
                    graphic_queue_family = Some(index as u32);
                }

                let is_support_surface = surface_loader
                    .get_physical_device_surface_support(physical_device, index as u32, surface)
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
                let is_support_surface = surface_loader
                    .get_physical_device_surface_support(physical_device, index as u32, surface)
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
        surface_loader: &ash::extensions::khr::Surface,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> (
        vk::SurfaceCapabilitiesKHR,
        Vec<vk::SurfaceFormatKHR>,
        Vec<vk::PresentModeKHR>,
    ) {
        let capabilities = surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap();
        let formats = surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap();
        let present_modes = surface_loader
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
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}
