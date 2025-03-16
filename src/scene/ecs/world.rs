use crate::scene::ecs::*;
use egui::ahash::{HashMap, HashMapExt};
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicU32, Ordering};

pub struct EntityIndex {
    pub index: usize,
    pub generation: usize,
}

pub struct World {
    entity_id_index_map: HashMap<u32, EntityIndex>,
    components_map: HashMap<TypeId, Vec<Option<Box<dyn Any + 'static>>>>,
}

impl World {
    pub fn new() -> World {
        World {
            entity_id_index_map: HashMap::new(),
            components_map: HashMap::new(),
        }
    }

    pub fn add_entity(&mut self) -> Entity {
        static COUNT: AtomicU32 = AtomicU32::new(0);
        let id = COUNT.fetch_add(1, Ordering::Relaxed);
        let index = EntityIndex {
            index: id as usize,
            generation: 0,
        };

        self.entity_id_index_map.insert(id, index);
        Entity::new(id)
    }

    pub fn remove_entity(self: &mut Self, entity: Entity) {
        if let Some(index) = self.entity_id_index_map.get(&entity.id) {
            self.components_map.iter_mut().for_each(|(_, components)| {
                components[index.index] = None;
            });
        }
    }

    pub fn add_entity_comp<T: Comp>(&mut self, entity: Entity, comp: T) {
        if let Some(index) = self.entity_id_index_map.get(&entity.id) {
            let index = index.index;
            let id = TypeId::of::<T>();
            let mut comps = self.components_map.entry(id).or_insert_with(|| {
                let mut data = Vec::new();
                data.resize_with(512, || None);
                data
            });

            comps[index] = Some(Box::new(comp));
        }
    }

    pub fn entity_count(&self) -> usize {
        self.entity_id_index_map.len()
    }

    pub fn get_entity_comp<T>(&self, entity: Entity) -> Option<&T>
    where
        T: Comp,
    {
        let index = self.entity_id_index_map.get(&entity.id)?.index;
        let comp = self.get_comps::<T>()?[index].as_ref()?;
        comp.downcast_ref::<T>()
    }

    pub fn get_entity_comp_mut<T: Comp>(&mut self, entity: Entity) -> Option<&mut T> {
        let index = self.entity_id_index_map.get(&entity.id)?.index;
        let comp = self.get_comps_mut::<T>()?[index].as_mut()?;
        comp.downcast_mut::<T>()
    }

    pub fn has_entity_comp<T: Comp>(&self, entity: Entity) -> bool {
        if let Some(index) = self.entity_id_index_map.get(&entity.id) {
            let index = index.index;
            self.get_comps::<T>()
                .is_some_and(|comps| comps.get(index).is_some())
        } else {
            false
        }
    }

    pub fn get_comps<T: Comp>(&self) -> Option<&Vec<Option<Box<dyn Any>>>> {
        let id = TypeId::of::<T>();
        self.components_map.get(&id)
    }

    pub fn get_comps_mut<T: Comp>(&mut self) -> Option<&mut Vec<Option<Box<dyn Any>>>> {
        let id = TypeId::of::<T>();
        self.components_map.get_mut(&id)
    }

    pub fn dispose(&mut self) {}
}
