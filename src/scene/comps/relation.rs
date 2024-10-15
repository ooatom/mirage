use crate::math::{Euler, Mat4, Vec3};
use crate::scene::comps::*;
use crate::scene::ecs::*;
use std::collections::HashMap;

pub struct Relation {
    pub owner: Entity,
    pub target: Option<Entity>,
    pub location: Option<Vec3>,
    pub rotation: Option<Euler>,
    pub scale: Option<Vec3>,
    pub soft_location: bool,
    pub soft_rotation: bool,
    pub soft_scale: bool,
}
impl Comp for Relation {}

impl Relation {
    pub fn new(owner: Entity, target: Entity) -> Self {
        Self {
            owner,
            target: Some(target),
            location: None,
            rotation: None,
            scale: None,
            soft_location: false,
            soft_rotation: false,
            soft_scale: false,
        }
    }

    pub fn relink(&mut self) {
        self.location = None;
        self.rotation = None;
        self.scale = None;
    }
}

// static flag to skip recalculation
fn relation_system(
    world: &mut World,
    relations: Vec<&mut Relation>,
    transforms: Vec<&mut Transform>,
) {
    // world.query()

    let relations_map = &mut HashMap::new();
    for (relation, transform) in relations.into_iter().zip(transforms.into_iter()) {
        let mut indices = relations_map.get_mut(&relation.target);
        if let None = indices {
            relations_map.insert(relation.target, vec![]);
            indices = relations_map.get_mut(&relation.target);
        }
        indices.unwrap().push((relation, transform));
    }

    fn update_related_matrix(
        relation: &mut Relation,
        transform: &mut Transform,
        relative_transform: Option<&Transform>,
        relations_map: &mut HashMap<Option<Entity>, Vec<(&mut Relation, &mut Transform)>>,
    ) {
        match (relation.location, relation.rotation, relation.scale) {
            (Some(location), Some(rotation), Some(scale)) => {
                let matrix = if relative_transform.is_none() {
                    Mat4::compose(location, rotation, scale)
                } else {
                    relative_transform.unwrap().matrix() * Mat4::compose(location, rotation, scale)
                };

                transform.matrix_mut(matrix);
            }
            (None, None, None) => {
                if relative_transform.is_none() {
                    relation.location = Some(Vec3::zero());
                    relation.rotation = Some(Euler::default());
                    relation.scale = Some(Vec3::zero());
                } else {
                    let (location, rotation, scale) =
                        Mat4::decompose(transform.matrix() / relative_transform.unwrap().matrix());
                    relation.location = Some(location);
                    relation.rotation = Some(rotation);
                    relation.scale = Some(scale);
                };
            }
            _ => {}
        }

        if let Some(relations) = relations_map.remove(&Some(relation.owner)) {
            relations.into_iter().for_each(|(relation2, transform2)| {
                update_related_matrix(relation2, transform2, Some(transform), relations_map);
            });
        }
    }

    if let Some(relations) = relations_map.remove(&None) {
        relations.into_iter().for_each(|(relation, transform)| {
            update_related_matrix(relation, transform, None, relations_map);
        });
    }
}
