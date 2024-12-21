use std::sync::Mutex;
use crate::scene::ecs::{SystemState, World};

pub struct Scheduler {
    systems: Vec<Box<dyn Fn(&mut World, &SystemState)>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler { systems: vec![] }
    }

    pub fn add_system<F>(&mut self, system: F)
    where
        F: Fn(&mut World, &SystemState) + 'static,
    {
        self.systems.push(Box::new(system));
    }

    pub fn tick(&mut self, world: &mut World, delta_time: f32) {
        static ELAPSED_TIME: Mutex<f32> = Mutex::new(0.0);

        let mut time = ELAPSED_TIME.lock().unwrap();
        *time += delta_time;
        let elapsed_time = time.clone();

        let state = SystemState {
            delta_time,
            elapsed_time,
        };
        unsafe {
            self.systems.iter().for_each(|system| {
                system(world, &state);
            });
        }
    }
}
