use ash::vk;

pub enum ShaderNode<'a> {
    Texture {
        id: &'a str,
        binding: u32,
        path: &'a str,
        stage: vk::ShaderStageFlags,
    },
    TextureArray {
        id: &'a str,
        binding: u32,
        paths: Vec<&'a str>,
        stage: vk::ShaderStageFlags,
    },
    TextureSample {
        id: &'a str,
        binding: u32,
        texture: &'a str,
        uvs: &'a str,
        stage: vk::ShaderStageFlags,
    },
    UniformBuffer {
        id: &'a str,
        binding: u32,
        // buffer: &'a str,
        stage: vk::ShaderStageFlags,
    },
    Shading {
        id: &'a str,
        base_color: &'a str,
    },
}

pub const SIMPLE_SHADER_NODES: [ShaderNode; 3] = [
    ShaderNode::Texture {
        id: "Texture0",
        binding: 0,
        path: "assets/viking_room.png",
        stage: vk::ShaderStageFlags::FRAGMENT,
    },
    ShaderNode::TextureSample {
        id: "TextureSample1",
        binding: 1,
        texture: "Texture0",
        uvs: "0",
        stage: vk::ShaderStageFlags::FRAGMENT,
    },
    ShaderNode::Shading {
        id: "2",
        base_color: "TextureSample1",
    },
];
