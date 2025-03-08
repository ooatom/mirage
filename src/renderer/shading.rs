use super::*;
use ash::vk;
use egui::ahash::HashMap;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ShadingMode {
    Unlit,
}

#[derive(Debug, Clone)]
pub struct Shading {
    pub id: u32,
    pub name: &'static str,
    pub path: &'static str,
    pub mode: ShadingMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub bindings: Vec<vk::DescriptorSetLayoutBinding<'static>>,
    // pub inputs: HashMap<&str, ?>
}

impl Shading {
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

        Shading {
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
