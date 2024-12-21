mod math;
mod mirage;
mod scene;
mod app;
mod renderer;
mod gpu;
mod loaders;

use rust_embed::RustEmbed;
use winit::event_loop::{ControlFlow, EventLoop};
use app::Application;

#[derive(RustEmbed)]
#[folder = "assets"]
pub struct Assets;

#[derive(RustEmbed)]
#[folder = "$OUT_DIR/shaders"]
pub struct Shaders;

fn main() {
    let mut app = Application::new();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
