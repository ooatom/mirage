use crate::scene::ecs::{Query, World};

pub struct CollideEvent {}

pub struct SystemState {
    pub delta_time: f32,
    pub elapsed_time: f32,
}
