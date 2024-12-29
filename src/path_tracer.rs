use crate::{Ray, Scene, Vec3};
use fastrand::Rng;

pub fn path_trace(ray: &mut Ray, first_reflection: bool) {
    let hit: bool = false;
    let mut rng = Rng::new();
    let ray_origin = ray.origin;
    let ray_direction = ray.direction;

    let mut air_distance: f32 = 0.0;

    loop {}
}

pub fn next_intersection(scene: Scene, ray: &mut Ray) -> bool {
    let ray = 
}
