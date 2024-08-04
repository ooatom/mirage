use super::*;
use crate::gpu::LayoutDesc;
use crate::Shaders;
use ash::vk;
use std::borrow::Cow;

pub enum ShadingMode {
    Unlit,
}

pub struct ShadingDef {
    pub name: &'static str,
    pub path: &'static str,
    pub data: Cow<'static, [u8]>,
    pub mode: ShadingMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub layouts: Vec<LayoutDesc>,
}

impl ShadingDef {
    pub fn load(path: &'static str) -> Self {
        ShadingDef {
            name: "Simple",
            path,
            data: Shaders::get(path).unwrap().data,
            mode: ShadingMode::Unlit,
            depth_test: false,
            depth_write: false,
            layouts: vec![
                // ShadingDesc {
                //     name: "model",
                //     binding: 1,
                //     desc_type: vk::DescriptorType::,
                //     count: 1,
                //     stage: vk::ShaderStageFlags::ALL,
                // },
                LayoutDesc {
                    name: "image",
                    binding: 0,
                    desc_type: vk::DescriptorType::SAMPLED_IMAGE,
                    count: 1,
                    stage: vk::ShaderStageFlags::FRAGMENT,
                },
                LayoutDesc {
                    name: "sampler",
                    binding: 1,
                    desc_type: vk::DescriptorType::SAMPLER,
                    count: 1,
                    stage: vk::ShaderStageFlags::FRAGMENT,
                },
            ],
        }
    }
}
