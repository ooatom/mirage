use super::*;
use ash::vk;

#[derive(Copy, Clone, PartialEq)]
pub enum ShadingMode {
    Unlit,
}

pub struct ShadingDef<'a> {
    pub id: u32,
    pub name: &'a str,
    pub path: &'a str,
    pub mode: ShadingMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub bindings: Vec<vk::DescriptorSetLayoutBinding<'a>>,
}

impl ShadingDef<'_> {
    pub fn load(path: &'static str) -> Self {
        let mut bindings: Vec<vk::DescriptorSetLayoutBinding> = vec![];

        SIMPLE_SHADER_NODES.iter().for_each(|node| match node {
            ShaderNode::Texture { binding, stage, .. } => {
                bindings.push(vk::DescriptorSetLayoutBinding {
                    binding: *binding,
                    descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                    descriptor_count: 1,
                    stage_flags: *stage,
                    ..Default::default()
                });
            }
            ShaderNode::TextureSample { binding, stage, .. } => {
                bindings.push(vk::DescriptorSetLayoutBinding {
                    binding: *binding,
                    descriptor_type: vk::DescriptorType::SAMPLER,
                    descriptor_count: 1,
                    stage_flags: *stage,
                    ..Default::default()
                });
            }
            // ShaderNode::Shading { .. } => {}
            // ShaderNode::TextureArray { .. } => {}
            _ => {}
        });

        ShadingDef {
            id: 0,
            name: "Simple",
            path,
            mode: ShadingMode::Unlit,
            depth_test: true,
            depth_write: true,
            bindings,
        }
    }
}
