use crate::assets::asset_impl::AssetImpl;
use crate::assets::{AssetHandle, Texture};

#[derive(Debug, Clone)]
pub struct Material {
    pub def_name: &'static str,
    pub tex: Option<AssetHandle<Texture>>,
}

impl Material {
    pub fn new(def_name: &'static str) -> Self {
        Self {
            def_name,
            tex: None,
        }
    }
}

impl AssetImpl for Material {}
