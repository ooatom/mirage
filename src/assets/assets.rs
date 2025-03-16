use super::asset_handle::{AssetHandle, AssetId};
use super::asset_impl::AssetImpl;
use super::{AssetBundle, AssetBundle2};
use egui::ahash::{HashMap, HashMapExt};
use rust_embed::RustEmbed;
use std::any::Any;
use std::borrow::Cow;
use std::hash::Hash;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct Assets {
    pool: HashMap<AssetId, Box<dyn Any>>,
}

impl Assets {
    pub fn new() -> Self {
        Assets {
            pool: HashMap::new(),
        }
    }

    pub fn load_raw(path: &str) -> Option<Cow<'static, [u8]>> {
        if let Some(result) = AssetBundle::get(path) {
            return Some(result.data);
        }

        Some(AssetBundle2::get(path)?.data)
    }

    pub fn handle_path<T: AssetImpl>(self: &mut Self, path: &str) -> Option<AssetHandle<T>> {
        let data = Assets::load_raw(path);
        match data {
            None => None,
            Some(data) => match T::load(data.as_ref()) {
                None => None,
                Some(asset) => Some(self.handle(asset)),
            },
        }
    }

    pub fn handle<T: AssetImpl>(self: &mut Self, asset: T) -> AssetHandle<T> {
        static COUNT: AtomicU32 = AtomicU32::new(1);
        // let mut rng = thread_rng();
        // let rnd: u64 = rng.gen_range(0..1 << 16);
        // let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        // let id = (dur.as_nanos() << 16) as u64 + rnd;
        let id = COUNT.fetch_add(1, Ordering::Relaxed);

        self.pool.insert(id, Box::new(asset));
        AssetHandle::new(id)
    }

    pub fn load<T: AssetImpl>(&self, handle: &AssetHandle<T>) -> Option<&T> {
        let asset = self.pool.get(&handle.id).unwrap();
        asset.downcast_ref::<T>()
    }

    pub fn load_mut<T: AssetImpl>(&mut self, handle: &AssetHandle<T>) -> Option<&mut T> {
        let asset = self.pool.get_mut(&handle.id).unwrap();
        asset.downcast_mut::<T>()
    }
}
