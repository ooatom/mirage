use ash::vk;

pub struct Shading {
    pub descriptor_set_layout: vk::DescriptorSetLayout,

    pub shader_module: vk::ShaderModule,
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,

    pub descriptor_sets: Vec<vk::DescriptorSet>,
}

// impl Shading {
//     pub fn new(
//         shader_module: vk::ShaderModule,
//         descriptor_set_layout: vk::DescriptorSetLayout,
//         pipeline: vk::Pipeline,
//         pipeline_layout: vk::PipelineLayout,
//         descriptor_sets: Vec<vk::DescriptorSet>,
//     ) -> Self {
//         Self {
//             shader_module,
//             descriptor_set_layout,
//             pipeline,
//             pipeline_layout,
//             descriptor_sets,
//         }
//     }
// }
