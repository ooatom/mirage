use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use crate::assets::asset_impl::AssetImpl;

#[derive(Debug, Copy, Clone)]
pub struct AssetHandle<T: AssetImpl> {
    pub id: u32,
    _phantom: PhantomData<T>,
}

impl<T: AssetImpl> AssetHandle<T> {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }
}

impl<T: AssetImpl> Hash for AssetHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
