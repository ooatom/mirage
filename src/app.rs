use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use crate::mirage::Mirage;

pub struct Application {
    pub window: Option<Rc<Window>>,
    pub mirage: Option<Mirage>,
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title("Mirage")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

        self.window = Some(Rc::new(event_loop.create_window(attributes).unwrap()));
        let window = self.window.as_ref().unwrap();
        if self.mirage.is_none() {
            self.mirage = Some(Mirage::initialize(window));
        } else {
            self.mirage.as_ref().unwrap().update_window(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = self.window.as_ref().unwrap();
        if window_id != window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.mirage.as_ref().unwrap().render();
            }
            // WindowEvent::Resized(size) => {
            //
            // }
            // WindowEvent::ScaleFactorChanged => {
            //
            // }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }
        self.window.as_ref().unwrap().request_redraw();
    }
}
