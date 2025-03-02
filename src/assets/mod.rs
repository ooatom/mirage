mod asset_handle;
mod asset_impl;
mod assets;
mod geom;
mod material;
mod texture;

pub use asset_handle::AssetHandle;
pub use assets::Assets;
pub use geom::Geom;
pub use material::Material;
pub use texture::Texture;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
struct AssetBundle;

#[derive(RustEmbed)]
#[folder = "$OUT_DIR/shaders"]
struct AssetBundle2;
