use crate::{Ray, Scene};
use glam::Vec3A as Vec3;
use rand::{rngs::StdRng, SeedableRng};

pub fn path_trace(ray: &mut Ray, first_reflection: bool) {
    let hit: bool = false;
    let mut rng = StdRng::from_entropy();
    let ray_origin = ray.origin;
    let ray_direction = ray.direction;

    let mut air_distance: f32 = 0.0;

    loop {}
}

pub fn next_intersection(scene: Scene, ray: &mut Ray) -> bool {
    ray.hit.previous_material = ray.hit.current_material;
}
