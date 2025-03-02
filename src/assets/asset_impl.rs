pub trait AssetImpl
where
    Self: 'static + Sized,
{
    fn load(data: &[u8]) -> Option<Self> {
        None
    }
}
