use crate::mirage::Mirage;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub struct Application {
    pub window: Option<Rc<Window>>,
    pub mirage: Option<Mirage>,
}

impl Application {
    pub fn new() -> Self {
        Self {
            window: None,
            mirage: None,
        }
    }

    fn init(&mut self, window: Window) {
        let rc_window = Rc::new(window);

        if let Some(mirage) = &self.mirage {
            mirage.update_window(Rc::clone(&rc_window));
        } else {
            let mut mirage = Mirage::new(Rc::clone(&rc_window));
            self.mirage = Some(mirage);
        }

        self.window = Some(rc_window);
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title("Mirage")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

        match event_loop.create_window(attributes) {
            Ok(window) => self.init(window),
            Err(_) => {}
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
                self.mirage.as_mut().unwrap().render();
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
