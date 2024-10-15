use std::hash::{Hash};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Entity {
    pub id: u32,
}

impl Entity {
    pub fn new(id: u32) -> Self {
        Self { id }
    }

}
