use std::any::TypeId;

pub trait Comp where Self: 'static {
    fn id() -> TypeId where Self: Sized {
        TypeId::of::<Self>()
    }
}

