use raw_window_handle;
use std::rc::Rc;

use super::renderer::*;
use winit::window::Window;

pub struct Mirage {
    context: Rc<VkContext>,
    device_context: Rc<VkDeviceContext>,
    forward_renderer: Rc<ForwardRenderer>,
    simple_pass: SimplePass,
}

impl Mirage {
    pub fn initialize(window: &Rc<Window>) -> Self {
        let context = Rc::new(VkContext::new(window));
        let device_context = Rc::new(VkDeviceContext::new(Rc::clone(&context)));
        let forward_renderer = Rc::new(ForwardRenderer::new(Rc::clone(&device_context)));

        let mut simple_pass = SimplePass::new(Rc::clone(&device_context), Rc::clone(&forward_renderer));
        simple_pass.add_object(SimplePassObject::new(&simple_pass));

        Self {
            context,
            device_context,
            forward_renderer,
            simple_pass,
        }
    }

    pub fn update_window(&self, window: &Rc<Window>) {}

    pub fn render(&self) {
        self.forward_renderer.render(&self.simple_pass);
    }
}
