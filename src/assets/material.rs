use crate::assets::asset_impl::AssetImpl;
use crate::assets::{AssetHandle, Texture};
use crate::renderer::Shading;
use egui::ahash::{HashMap, HashMapExt};

#[derive(Debug, Clone)]
pub struct Material {
    pub shading: Shading,
    props: HashMap<&'static str, Option<AssetHandle<Texture>>>,
}

impl Material {
    pub fn new(shading: Shading) -> Self {
        Self {
            shading,
            props: HashMap::new(),
        }
    }

    pub fn set_texture(&mut self, key: &'static str, value: Option<AssetHandle<Texture>>) {
        self.props.insert(key, value);
    }

    pub fn get_texture(&self, key: &str) -> Option<AssetHandle<Texture>> {
        match self.props.get(key) {
            None => None,
            Some(value) => match value {
                None => None,
                Some(tex) => Some(tex.to_owned()),
            },
        }
    }
}

impl AssetImpl for Material {}
