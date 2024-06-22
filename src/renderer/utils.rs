use crate::math::{Vec3, Vec4};
use crate::renderer::VkDeviceContext;
use ash::vk;

pub unsafe fn create_image(
    device: &VkDeviceContext,
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

    let create_info = vk::ImageCreateInfo::default()
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
        .samples(samples);
    // There are some optional flags for images that are related to sparse images. Sparse images are images where only certain regions are actually backed by memory.
    // If you were using a 3D texture for a voxel terrain, for example, then you could use this to avoid allocating memory to store large volumes of "air" values.
    // .flags()

    let image = device
        .device
        .create_image(&create_info, None)
        .expect("failed to create image!");

    let memory_requirements = device.device.get_image_memory_requirements(image);

    let allocate_info = vk::MemoryAllocateInfo {
        allocation_size: memory_requirements.size,
        memory_type_index: find_memory_type_index(
            device,
            memory_requirements.memory_type_bits,
            memory_properties,
        ),
        ..Default::default()
    };

    let image_memory = device
        .device
        .allocate_memory(&allocate_info, None)
        .expect("failed to allocate memory!");
    device
        .device
        .bind_image_memory(image, image_memory, 0)
        .expect("failed to bind image memory!");

    (image, image_memory)
}

pub unsafe fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
    aspect_flags: vk::ImageAspectFlags,
    mips: u32,
) -> vk::ImageView {
    let create_info = vk::ImageViewCreateInfo::default()
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
        });

    device
        .create_image_view(&create_info, None)
        .expect("failed to create image view!")
}

pub unsafe fn create_buffer(
    device: &VkDeviceContext,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    memory_properties: vk::MemoryPropertyFlags,
) -> (vk::Buffer, vk::DeviceMemory, vk::DeviceSize) {
    let create_info = vk::BufferCreateInfo::default()
        // The flags parameter is used to configure sparse buffer memory,
        // which is not relevant right now. We'll leave it at the default value of 0.
        // .flags()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = device
        .device
        .create_buffer(&create_info, None)
        .expect("failed to create buffer!");

    let requirements = device.device.get_buffer_memory_requirements(buffer);
    let allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(find_memory_type_index(
            device,
            requirements.memory_type_bits,
            memory_properties,
        ));

    let buffer_memory = device
        .device
        .allocate_memory(&allocate_info, None)
        .expect("failed to allocate memory!");

    // If the offset is non-zero, then it is required to be divisible by memRequirements.alignment.
    device
        .device
        .bind_buffer_memory(buffer, buffer_memory, 0)
        .expect("failed to bind buffer memory!");

    (buffer, buffer_memory, requirements.size)
}

pub fn find_memory_type_index(
    device: &VkDeviceContext,
    type_bits: u32,
    property_flags: vk::MemoryPropertyFlags,
) -> u32 {
    for i in 0..device.physical_device_memory_properties.memory_type_count {
        if type_bits & (1 << i) == 0 {
            continue;
        }
        if !device.physical_device_memory_properties.memory_types[i as usize]
            .property_flags
            .contains(property_flags)
        {
            continue;
        }

        return i;
    }

    panic!("failed to find suitable memory type!")
}
