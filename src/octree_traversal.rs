use glam::Vec3A;

use crate::{Cuboid, Material, Octree, Ray, AABB};

impl Octree<u32> {
    const MAX_STEPS: usize = 1000;
    const MAX_SCALE: u32 = 23;
    const EPSILON: f32 = 0.00000011920929;
    pub fn intersect_octree(
        &self,
        ray: &mut Ray,
        max_dst: f32,
        do_translucency: bool,
        palette: &Vec<Cuboid>,
        materials: &Vec<Material>,
    ) -> bool {
        let (mut t_min, mut tmax) = (0.0, 1.0);
        let mut size = 2f32.powi(self.depth() as i32) as u32;
        let bounds = AABB::new(Vec3A::ZERO, Vec3A::splat(1.0 * size as f32));
        
        let parent = self.root;
    }
}
