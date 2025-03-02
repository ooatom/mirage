use super::asset_impl::AssetImpl;

#[derive(Debug, Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub pixels: Vec<u8>,
}

impl Texture {}

impl AssetImpl for Texture {
    fn load(data: &[u8]) -> Option<Self> {
        let image = image::load_from_memory(data).expect("failed to load image!");
        let image_rgba8 = image.to_rgba8();
        let width = image_rgba8.width();
        let height = image_rgba8.height();
        let mip_levels = ((width.min(height) as f32).log2().floor() + 1.0) as u32;
        let pixels = image_rgba8.into_raw();

        Some(Self {
            width,
            height,
            pixels,
            mip_levels,
        })
    }
}
