use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Copy)]
pub struct Material {
    id: u32,
    pub def_name: &'static str,
    pub tex: Option<Texture>,
}

impl Material {
    pub fn new(def_name: &'static str) -> Self {
        Self {
            id: allocate_id(),
            def_name,
            tex: None,
        }
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }
}

impl Clone for Material {
    fn clone(&self) -> Self {
        Self {
            id: allocate_id(),
            def_name: self.def_name,
            tex: self.tex,
        }
    }
}

fn allocate_id() -> u32 {
    static COUNT: AtomicU32 = AtomicU32::new(1);
    // let mut rng = thread_rng();
    // let rnd: u64 = rng.gen_range(0..1 << 16);
    // let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    // let id = (dur.as_nanos() << 16) as u64 + rnd;

    COUNT.fetch_add(1, Ordering::Relaxed)
}
