use crate::assets::asset_impl::AssetImpl;
use std::hash::Hash;
use std::marker::PhantomData;

pub type AssetId = u32;

#[derive(Debug, Copy, Clone, Hash)]
pub struct AssetHandle<T: AssetImpl> {
    pub id: AssetId,
    _phantom: PhantomData<T>,
}

impl<T: AssetImpl> AssetHandle<T> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }
}
