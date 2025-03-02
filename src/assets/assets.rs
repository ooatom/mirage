use super::asset_handle::AssetHandle;
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
    pool: HashMap<u32, Box<dyn Any>>,
    // me: Weak<RefCell<Assets>>,
}

impl Assets {
    pub fn new() -> Self {
        Assets {
            pool: HashMap::new(),
            // me: me.clone(),
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

    pub fn load<T: AssetImpl>(&self, handle: &AssetHandle<T>) -> &T {
        let asset = self.pool.get(&handle.id).unwrap();
        asset
            .downcast_ref::<T>()
            .expect(&format!("Asset load failed. {:?}", handle.id))
    }

    pub fn load_mut<T: AssetImpl>(&mut self, handle: &AssetHandle<T>) -> &mut T {
        let asset = self.pool.get_mut(&handle.id).unwrap();
        asset
            .downcast_mut::<T>()
            .expect(&format!("Asset load failed. {:?}", handle.id))
    }

    // pub fn add<T: AssetImpl>(&mut self, key: &'static str, asset: T) -> Rc<AssetHandle<T>> {
    //     let asset = Rc::new(RefCell::new(asset));
    //     let output = Rc::clone(&asset);
    //     let type_key = T::type_id();
    //
    //     if !self.pool.contains_key(&type_key) {
    //         self.pool.insert(type_key, HashMap::new());
    //     }
    //     let mut type_pool = self.pool.get_mut(&type_key).unwrap();
    //     type_pool.insert(key, asset);
    //
    //     output
    // }

    //
    // pub fn add<T: AssetImpl>(&mut self, key: &'static str, asset: T) -> Rc<RefCell<T>> {
    //     let asset = Rc::new(RefCell::new(asset));
    //     let output = Rc::clone(&asset);
    //     let type_key = T::type_id();
    //
    //     if !self.pool.contains_key(&type_key) {
    //         self.pool.insert(type_key, HashMap::new());
    //     }
    //     let mut type_pool = self.pool.get_mut(&type_key).unwrap();
    //     type_pool.insert(key, asset);
    //
    //     output
    // }

    // pub fn get<T: AssetImpl>(&self, key: &str) -> Option<Rc<RefCell<T>>> {
    //     let type_pool = self.pool.get(&T::type_id());
    //     match type_pool {
    //         None => None,
    //         Some(type_pool) => match type_pool.get(key) {
    //             None => None,
    //             Some(asset) => {
    //                 let a2 = Rc::clone(asset);
    //                 Rc::downcast::<RefCell<T>>(a2).ok()
    //             }
    //         },
    //     }
    // }
}
