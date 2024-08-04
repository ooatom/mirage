use ash::vk;

#[derive(Copy, Clone)]
pub struct LayoutDesc {
    pub name: &'static str,
    pub desc_type: vk::DescriptorType,
    pub binding: u32,
    pub stage: vk::ShaderStageFlags,
    pub count: u32,
}
