mod gpu;
mod math;
mod mirage;

use std::rc::Rc;
use mirage::Mirage;
use rust_embed::RustEmbed;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[derive(RustEmbed)]
#[folder = "assets"]
pub struct Assets;

#[derive(RustEmbed)]
#[folder = "$OUT_DIR/shaders"]
pub struct Shaders;

struct Application {
    window: Option<Rc<Window>>,
    mirage: Option<Mirage>,
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

fn main() {
    let mut app = Application {
        window: None,
        mirage: None,
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
