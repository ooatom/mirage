use crate::assets::{Assets, Geom, Material, Texture};
use crate::math::{Euler, Vec3};
use crate::renderer::Shading;
use crate::scene::camera::Camera;
use crate::scene::{StaticMesh, Transform, World};
use std::f32::consts::PI;

pub fn load_simple_scene(world: &mut World, assets: &mut Assets) {
    let entity = world.add_entity();
    let geom_handle = assets.handle_path::<Geom>("viking_room.obj");
    let material_handle = assets.handle(Material::new(Shading::load("simple.spv")));
    let texture_handle = assets.handle_path::<Texture>("texture.jpg");

    let material = assets.load_mut(&material_handle).unwrap();
    material.set_texture("texture", texture_handle);

    world.add_entity_comp(
        entity,
        Transform::new(
            Vec3::new(1.0, 0.0, -0.8),
            Euler::default(),
            Vec3::new(2.0, 2.0, 2.0),
        ),
    );
    world.add_entity_comp(
        entity,
        StaticMesh::new(geom_handle.clone(), Some(material_handle)),
    );

    let entity = world.add_entity();
    let material_handle = assets.handle(Material::new(Shading::load("simple.spv")));
    let texture_handle = assets.handle_path::<Texture>("viking_room.png");
    let material = assets.load_mut(&material_handle).unwrap();
    material.set_texture("texture", texture_handle);

    world.add_entity_comp(
        entity,
        Transform::new(
            Vec3::new(3.0, 0.0, 1.2),
            Euler::default(),
            Vec3::new(2.0, 2.0, 2.0),
        ),
    );

    world.add_entity_comp(entity, StaticMesh::new(geom_handle, Some(material_handle)));

    let camera = world.add_entity();
    world.add_entity_comp(
        camera,
        Transform::new(Vec3::new(0.0, 10.0, -10.0), Euler::default(), Vec3::one()),
    );
    world.add_entity_comp(camera, Camera::new(PI / 2.0, 1.0, 0.01));
}
