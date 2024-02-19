use rust_embed::RustEmbed;
mod mirage;

#[derive(RustEmbed)]
#[folder = "assets"]
pub struct Assets;

#[derive(RustEmbed)]
#[folder = "$OUT_DIR/shaders"]
pub struct Shaders;

fn main() {
    let mut mirage = mirage::Mirage::initialize();
    mirage.main_loop();
}
