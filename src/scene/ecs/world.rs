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
    systems: Vec<Box<dyn Fn(&mut World, &SystemState)>>,
}

impl World {
    pub fn new() -> World {
        World {
            entity_id_index_map: HashMap::new(),
            components_map: HashMap::new(),
            systems: vec![],
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
            let id = TypeId::of::<T>();
            let mut comps = self.components_map.get_mut(&id);
            if comps.is_none() {
                let mut data = Vec::<Option<Box<dyn Any>>>::with_capacity(512);
                unsafe {
                    data.set_len(512);
                }
                self.components_map.insert(id, data);

                comps = self.components_map.get_mut(&id);
            }

            comps.unwrap()[index.index] = Some(Box::new(comp));
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
        let id = TypeId::of::<T>();
        let comp = self.components_map.get(&id)?[index].as_ref()?;
        comp.downcast_ref::<T>()
    }

    pub fn get_entity_comp_mut<T: Comp>(&mut self, entity: Entity) -> Option<&mut T> {
        let index = self.entity_id_index_map.get(&entity.id)?.index;
        let id = TypeId::of::<T>();
        let comp = self.components_map.get_mut(&id)?[index].as_mut()?;
        comp.downcast_mut::<T>()
    }

    pub fn get_comps<T: Comp>(&mut self) -> Option<&Vec<Option<Box<dyn Any>>>> {
        let id = TypeId::of::<T>();
        Some(self.components_map.get(&id)?)
    }

    pub fn get_comps_by_type_id(
        &mut self,
        type_id: TypeId,
    ) -> Option<&mut Vec<Option<Box<dyn Any>>>> {
        Some(self.components_map.get_mut(&type_id)?)
    }

    pub fn has_entity_comp<T: Comp>(&self, entity: Entity) -> bool {
        if let Some(index) = self.entity_id_index_map.get(&entity.id) {
            let id = TypeId::of::<T>();

            let comps = self.components_map.get(&id);
            if comps.is_none() {
                return false;
            }

            comps.unwrap().get(index.index).is_some()
        } else {
            false
        }
    }

    pub fn add_system<F>(&mut self, system: F)
    where
        F: Fn(&mut World, &SystemState) + 'static,
    {
        self.systems.push(Box::new(system));
    }

    pub fn tick(&mut self, delta_time: f32) {
        let state = SystemState { delta: delta_time };
        let world = (&mut *self) as *mut World;
        unsafe {
            (*world).systems.iter().for_each(|system| {
                system(self, &state);
            });
        }
    }

    pub fn dispose(&mut self) {}
}
