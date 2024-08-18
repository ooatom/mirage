use ash::vk;

#[derive(Copy, Clone)]
pub struct Pipeline {
    pub def: u32,

    pub descriptor_set_layout: vk::DescriptorSetLayout,

    pub shader_module: vk::ShaderModule,
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
}

pub struct Shading {
    pub pipeline: Pipeline,
    pub pipeline_dirty: bool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
}
