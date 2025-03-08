use crate::assets::{Assets, Material};
use crate::gpu::GPU;
use crate::renderer::forward_renderer::ObjectData;
use crate::renderer::vertex::Vertex;
use crate::renderer::{ForwardRenderer, Shading};
use ash::vk;
use std::ffi::CStr;
use std::io;

#[derive(Debug, Copy, Clone)]
pub struct GPUPipeline {
    pub descriptor_set_layout: vk::DescriptorSetLayout,

    pub shader_module: vk::ShaderModule,
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,

    descriptor_sets: [Option<vk::DescriptorSet>; 5],
}

impl GPUPipeline {
    pub fn new(gpu: &GPU, material: &Material, renderer: &ForwardRenderer) -> Self {
        // The Vulkan SDK includes libshaderc, which is a library to compile GLSL code to SPIR-V from within your program.
        // https://github.com/google/shaderc
        // little endian
        // let mut buffer = Cursor::new(Shaders::get("simple.vert.spv").unwrap().data);
        // let vert_shader_code = ash::util::read_spv(&mut buffer).unwrap();
        // let mut buffer = Cursor::new(Shaders::get("simple.frag.spv").unwrap().data);
        // let frag_shader_code = ash::util::read_spv(&mut buffer).unwrap();

        // let vert_shader_module = device.create_shader_module(&vert_shader_code);
        // let frag_shader_module = device.create_shader_module(&frag_shader_code);
        let data = Assets::load_raw(material.shading.path).unwrap();
        let mut buffer = io::Cursor::new(&data);
        let shader_code = ash::util::read_spv(&mut buffer).unwrap();
        let shader_module = gpu.create_shader_module(&shader_code);

        let descriptor_set_layout = gpu.create_descriptor_set_layout(&material.shading.bindings);
        let (pipeline, pipeline_layout) = Self::create_pipeline(
            gpu,
            renderer,
            &material.shading,
            shader_module,
            descriptor_set_layout,
        );

        let mut descriptor_sets = [None; 5];
        gpu.create_descriptor_sets(&vec![
            descriptor_set_layout;
            ForwardRenderer::FRAMES_IN_FLIGHT.min(5) as usize
        ])
        .into_iter()
        .enumerate()
        .for_each(|(index, set)| {
            descriptor_sets[index] = Some(set);
        });

        Self {
            descriptor_set_layout,
            shader_module,
            pipeline,
            pipeline_layout,
            descriptor_sets,
        }
    }

    pub fn get_descriptor_set(&self, frame_index: usize) -> vk::DescriptorSet {
        self.descriptor_sets[frame_index].unwrap()
    }

    fn create_pipeline(
        gpu: &GPU,
        renderer: &ForwardRenderer,
        shading: &Shading,
        shader_module: vk::ShaderModule,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        unsafe {
            let vert_shader_stage = vk::PipelineShaderStageCreateInfo::default()
                .module(shader_module)
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(CStr::from_bytes_with_nul_unchecked(b"vs\0"));
            // It allows you to specify values for shader constants. You can use a single shader module where its behavior can be configured
            // at pipeline creation by specifying different values for the constants used in it. This is more efficient than configuring
            // the shader using variables at render time, because the compiler can do optimizations like eliminating if statements that
            // depend on these values. If you don't have any constants like that, then you can set the member to nullptr,
            // which our struct initialization does automatically.
            // .specialization_info()

            let frag_shader_stage = vk::PipelineShaderStageCreateInfo::default()
                .module(shader_module)
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .name(CStr::from_bytes_with_nul_unchecked(b"fs\0"));

            let shader_stages = [vert_shader_stage, frag_shader_stage];

            let input_bindings = [Vertex::get_binding_description()];
            let input_attributes = Vertex::get_attribute_descriptions();

            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&input_bindings)
                .vertex_attribute_descriptions(&input_attributes);

            let input_assembly_stage = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                // used with Indexed drawing + Triangle Fan/Strip topologies. This is more efficient than explicitly
                // ending the current primitive and explicitly starting a new primitive of the same type.
                // A special “index” indicates that the primitive should start over.
                //   If VkIndexType is VK_INDEX_TYPE_UINT16, special index is 0xFFFF
                //   If VkIndexType is VK_INDEX_TYPE_UINT32, special index is 0xFFFFFFFF
                // One Really Good use of Restart Enable is in Drawing Terrain Surfaces with Triangle Strips.
                .primitive_restart_enable(false);

            let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

            let viewport_state = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);

            let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .rasterizer_discard_enable(false)
                .depth_clamp_enable(false)
                .depth_bias_enable(false)
                .depth_bias_clamp(0.0)
                .depth_bias_slope_factor(0.0)
                .depth_bias_constant_factor(0.0);

            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .sample_shading_enable(true)
                .min_sample_shading(0.2)
                .rasterization_samples(gpu.device_context.msaa_samples)
                .sample_mask(&[])
                .alpha_to_coverage_enable(false)
                .alpha_to_one_enable(false);

            let color_attachments = [vk::PipelineColorBlendAttachmentState {
                blend_enable: false.into(),
                src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
                dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::SRC_ALPHA,
                dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                alpha_blend_op: vk::BlendOp::ADD,
                color_write_mask: vk::ColorComponentFlags::RGBA,
            }];
            let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
                // corresponding to renderPass subPass pColorAttachments
                .attachments(&color_attachments)
                .blend_constants([0.0, 0.0, 0.0, 0.0])
                .logic_op_enable(false)
                .logic_op(vk::LogicOp::COPY);

            let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_write_enable(shading.depth_write)
                .depth_test_enable(shading.depth_test)
                .depth_compare_op(if renderer.depth_reverse_z {
                    vk::CompareOp::GREATER
                } else {
                    vk::CompareOp::LESS
                })
                .stencil_test_enable(false)
                .front(vk::StencilOpState::default())
                .back(vk::StencilOpState::default())
                // only keep fragments that fall within the specified depth range
                .depth_bounds_test_enable(false)
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0);

            let push_constant_ranges = [vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS)
                .offset(0)
                .size(size_of::<ObjectData>() as u32)];
            let descriptor_set_layouts =
                vec![renderer.descriptor_set_layout, descriptor_set_layout];
            let layout_create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_set_layouts)
                .push_constant_ranges(&push_constant_ranges);

            let pipeline_layout = gpu
                .device_context
                .device
                .create_pipeline_layout(&layout_create_info, None)
                .expect("failed to create pipeline layout!");

            let create_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_stage)
                .dynamic_state(&dynamic_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample)
                .color_blend_state(&color_blend)
                .depth_stencil_state(&depth_stencil)
                .layout(pipeline_layout)
                .render_pass(renderer.render_pass)
                .subpass(0)
                .base_pipeline_handle(vk::Pipeline::null())
                .base_pipeline_index(0);

            let pipeline = gpu
                .device_context
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
                .expect("failed to create graphics pipeline!")[0];

            (pipeline, pipeline_layout)
        }
    }

    pub fn drop(&mut self, gpu: &GPU) {
        unsafe {
            let device = &gpu.device_context.device;
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_shader_module(self.shader_module, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            // device
            //     .free_descriptor_sets(gpu.descriptor_pool, self.descriptor_sets.as_slice())
            //     .expect("TODO: panic message");
        }
    }
}
