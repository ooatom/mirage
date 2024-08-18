use crate::gpu::*;
use crate::math::{Euler, Mat4, Vec3};
use crate::renderer::*;
use ash::vk;
use raw_window_handle;
use std::cell::Cell;
use std::f32::consts::PI;
use std::rc::Rc;
use std::time::Instant;
use winit::window::Window;

pub struct Mirage {
    gpu: Rc<GPU>,
    // pub ui_state: egui_winit::State,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    frame_index: Cell<usize>,

    timer: Instant,
    elapsed_time: f32,
    forward_renderer: ForwardRenderer,
    objects: Vec<Object>,
}

impl Mirage {
    pub fn initialize(window: &Rc<Window>) -> Self {
        let gpu = Rc::new(GPU::new(&window));
        // let egui_context = egui::Context::default();
        // let egui_state = egui_winit::State::new(
        //     egui_context,
        //     egui::ViewportId::ROOT,
        //     &gpu.context.window,
        //     Some(gpu.context.window.scale_factor() as f32),
        //     None
        // );

        let command_pool = Self::create_command_pools(&gpu);

        let mut forward_renderer = ForwardRenderer::new(&gpu);
        forward_renderer.depth_reverse_z = true;
        let command_buffers =
            Self::create_command_buffers(&gpu, command_pool, ForwardRenderer::FRAMES_IN_FLIGHT);
        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) =
            Self::create_sync_objects(&gpu, ForwardRenderer::FRAMES_IN_FLIGHT);

        let (vertices, indices) = Geom::model();
        let geom = Geom::new(&gpu, vertices, indices);
        let mut material = Material::new("Simple");
        material.tex = Some(Texture::load(&gpu, "assets/texture.jpg"));
        let mut objects = vec![Object::new(geom, material.clone())];

        material.tex = Some(Texture::load(&gpu, "assets/viking_room.png"));
        objects.push(Object::new(geom, material.clone()));

        objects.push(Object::new(geom, material.clone()));

        Self {
            gpu,
            // ui_state: egui_state,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            frame_index: Cell::new(0),

            timer: Instant::now(),
            elapsed_time: 0.0,
            forward_renderer,
            objects,
        }
    }

    pub fn update_window(&self, window: &Rc<Window>) {}

    pub fn update_system(&mut self) {
        let elapsed_time = self.timer.elapsed().as_secs_f32();
        let delta_time = elapsed_time - self.elapsed_time;
        self.elapsed_time = elapsed_time;

        // let aspect = self.swapchain_properties.extent.width as f32
        //     / self.swapchain_properties.extent.height as f32;
        self.forward_renderer.view = Mat4::look_at_rh(
            Vec3::new(0.0, 10.0, 10.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // self.projection = Mat4::orthographic_rh(-2.0, 2.0, -2.0, 2.0, 0.01, 100.0);
        self.forward_renderer.projection =
            Mat4::perspective_reversed_z_infinite_rh(PI / 2.0, 1.0, 0.01);

        self.objects
            .iter_mut()
            .enumerate()
            .for_each(|(index, obj)| {
                obj.model = Mat4::translate(Vec3::new(index as f32 * 3.0, 0.0, -0.9))
                    * Mat4::from(Euler::new(0.0, self.elapsed_time, 0.0))
                    * Mat4::scale(Vec3::new(2.0, 2.0, 2.0));
            });
    }

    pub fn render(&mut self) {
        unsafe {
            let frame_index = self.frame_index.get();

            let fence = self.in_flight_fences[frame_index];
            let image_available_semaphore = self.image_available_semaphores[frame_index];
            let render_finished_semaphore = self.render_finished_semaphores[frame_index];

            // There happens to be two kinds of semaphores in Vulkan, binary and timeline. We use binary semaphores here.
            // A fence has a similar purpose, in that it is used to synchronize execution, but it is for ordering the execution on the CPU, otherwise known as the host.
            self.gpu
                .device_context
                .device
                .wait_for_fences(&[fence], true, u64::MAX)
                .expect("failed to wait fence!");

            let image_index =
                self.gpu
                    .swap_chain
                    .acquire_image(u64::MAX, Some(image_available_semaphore), None);

            self.gpu
                .device_context
                .device
                .reset_fences(&[fence])
                .expect("failed to reset fence!");

            let command_buffer = self.command_buffers[frame_index];
            self.gpu
                .device_context
                .device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("failed to reset command buffer!");

            let begin_info = vk::CommandBufferBeginInfo::default()
                // ONE_TIME_SUBMIT_BIT: The command buffer will be rerecorded right after executing it once.
                // RENDER_PASS_CONTINUE_BIT: This is a secondary command buffer that will be entirely within a single render pass.
                // SIMULTANEOUS_USE_BIT: The command buffer can be resubmitted while it is also already pending execution.
                .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);
            // Only relevant for secondary command buffers. It specifies which state to inherit from the calling primary command buffers.
            // .inheritance_info()

            self.gpu
                .device_context
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .expect("failed to begin command buffer!");

            {
                self.update_system();
                self.forward_renderer.render(
                    command_buffer,
                    &self.objects,
                    image_index as usize,
                    frame_index,
                );
            }

            self.gpu
                .device_context
                .device
                .end_command_buffer(command_buffer)
                .expect("failed to end command buffer!");

            let wait_semaphores = [image_available_semaphore];
            let signal_semaphores = [render_finished_semaphore];
            let command_buffers = [command_buffer];
            let stage_masks = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers)
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&stage_masks)
                .signal_semaphores(&signal_semaphores);
            self.gpu
                .device_context
                .device
                .queue_submit(
                    self.gpu.device_context.graphic_queue.unwrap(),
                    &[submit_info],
                    fence,
                )
                .unwrap();

            let image_indices = [image_index];
            let swap_chains = [self.gpu.swap_chain.swap_chain.unwrap()];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .image_indices(&image_indices)
                .swapchains(&swap_chains);

            // Queueing an image for presentation defines a set of queue operations, including waiting on the semaphores and submitting a presentation
            // request to the presentation engine. However, the scope of this set of queue operations does not include the actual processing of the
            // image by the presentation engine.
            // vkQueuePresentKHR releases the acquisition of the image, which signals imageAvailableSemaphores for that image in later frames.
            let present_result = self
                .gpu
                .swap_chain
                .swap_chain_fn
                .as_ref()
                .unwrap()
                .queue_present(
                    self.gpu.device_context.present_queue.unwrap(),
                    &present_info,
                );

            let is_suboptimal = present_result.unwrap_or_else(|err_code| {
                if err_code == vk::Result::ERROR_OUT_OF_DATE_KHR {
                    true
                } else {
                    panic!("failed to submit present queue!");
                }
            });
            if is_suboptimal {
                // framebufferResized = false;
                // self.recreate_swap_chain();
            }

            self.frame_index
                .set((frame_index + 1) % (self.in_flight_fences.len()));
        }
    }

    fn create_command_pools(gpu: &GPU) -> vk::CommandPool {
        unsafe {
            // VK_COMMAND_POOL_CREATE_TRANSIENT_BIT:
            //   Hint that command buffers are rerecorded with new commands very often (may change memory allocation behavior)
            // VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT:
            //   Allow command buffers to be rerecorded individually, without this flag they all have to be reset together
            let create_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(gpu.device_context.graphic_queue_family.unwrap());
            let command_pool = gpu
                .device_context
                .device
                .create_command_pool(&create_info, None)
                .expect("failed to create command pool!");

            command_pool
        }
    }

    fn create_command_buffers(
        gpu: &GPU,
        command_pool: vk::CommandPool,
        count: u32,
    ) -> Vec<vk::CommandBuffer> {
        unsafe {
            // VK_COMMAND_BUFFER_LEVEL_PRIMARY: Can be submitted to a queue for execution, but cannot be called from other command buffers.
            // VK_COMMAND_BUFFER_LEVEL_SECONDARY: Cannot be submitted directly, but can be called from primary command buffers.
            let allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .command_buffer_count(count)
                .level(vk::CommandBufferLevel::PRIMARY);

            gpu.device_context
                .device
                .allocate_command_buffers(&allocate_info)
                .expect("failed to allocate command buffers!")
        }
    }

    fn create_sync_objects(
        gpu: &GPU,
        count: u32,
    ) -> (Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>) {
        unsafe {
            let semaphore_create_info = vk::SemaphoreCreateInfo::default();

            let image_available_semaphores = (0..count)
                .map(|_| {
                    gpu.device_context
                        .device
                        .create_semaphore(&semaphore_create_info, None)
                        .expect("failed to create image available semaphore!")
                })
                .collect::<Vec<vk::Semaphore>>();

            let render_finished_semaphores = (0..count)
                .map(|_| {
                    gpu.device_context
                        .device
                        .create_semaphore(&semaphore_create_info, None)
                        .expect("failed to create render finished semaphore!")
                })
                .collect::<Vec<vk::Semaphore>>();

            let fence_create_info =
                vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
            let in_flight_fences: Vec<vk::Fence> = (0..count)
                .map(|_| {
                    gpu.device_context
                        .device
                        .create_fence(&fence_create_info, None)
                        .expect("failed to create in-flight fence!")
                })
                .collect::<Vec<vk::Fence>>();

            (
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
            )
        }
    }
}

impl Drop for Mirage {
    fn drop(&mut self) {
        unsafe {
            let device = &self.gpu.device_context.device;
            device.device_wait_idle().unwrap();

            self.objects.iter().for_each(|obj| {
                if let Some(tex) = obj.material.tex {
                    device.destroy_image(tex.image, None);
                    device.destroy_sampler(tex.image_sampler, None);
                    device.destroy_image_view(tex.image_view, None);
                    device.free_memory(tex.image_memory, None);
                }

                device.destroy_buffer(obj.geom.vertex_buffer, None);
                device.free_memory(obj.geom.vertex_buffer_memory, None);
                device.destroy_buffer(obj.geom.index_buffer, None);
                device.free_memory(obj.geom.index_buffer_memory, None);
            });

            self.image_available_semaphores
                .iter()
                .for_each(|&semaphore| device.destroy_semaphore(semaphore, None));
            self.render_finished_semaphores
                .iter()
                .for_each(|&semaphore| device.destroy_semaphore(semaphore, None));
            self.in_flight_fences
                .iter()
                .for_each(|&fence| device.destroy_fence(fence, None));

            device.destroy_command_pool(self.command_pool, None);
        }
    }
}
