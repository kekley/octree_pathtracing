use std::f32::INFINITY;

use crate::{
    aabb::AABB,
    bvh::{BVHNode, BVHTree},
    cuboid::{self, Cuboid},
    interval::Interval,
    ray::Ray,
    sphere::Sphere,
    vec3::Vec3,
};
#[derive(Debug, Clone)]
pub struct HitRecord {
    pub t: f32,
    pub u: f32,
    pub v: f32,
    pub mat_idx: u16,
    pub outward_normal: Vec3,
}

impl Default for HitRecord {
    fn default() -> Self {
        Self {
            t: INFINITY,
            u: 0.0,
            v: 0.0,
            mat_idx: 0,
            outward_normal: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Hittable {
    Sphere(Sphere),
    BVH(BVHTree),
    Box(Cuboid),
}

impl Hittable {
    #[inline]
    pub fn hit(&self, ray: &mut Ray, ray_t: Interval) {
        match self {
            Hittable::Sphere(sphere) => sphere.hit(ray, ray_t),
            Hittable::BVH(bvhtree) => bvhtree.hit(ray, ray_t),
            Hittable::Box(cuboid) => cuboid.hit(ray, ray_t),
        }
    }
    #[inline]
    pub fn get_bbox(&self) -> &AABB {
        match self {
            Hittable::Sphere(sphere) => &sphere.bbox,
            Hittable::BVH(bvhtree) => bvhtree.bbox(),
            Hittable::Box(r#box) => &r#box.bbox,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HitList {
    pub objects: Vec<Hittable>,
    pub bbox: AABB,
}

impl HitList {
    pub fn new() -> Self {
        Self {
            objects: vec![],
            bbox: AABB::default(),
        }
    }

    pub fn add(&mut self, object: Hittable) {
        self.bbox = AABB::from_boxes(&self.bbox, object.get_bbox());
        self.objects.push(object);
    }

    pub fn clear(&mut self) {
        self.objects.clear();
    }
    pub fn hit(&self, ray: &mut Ray, ray_t: Interval) {
        let mut closest_hit = ray_t.max;

        for object in &self.objects {
            object.hit(ray, Interval::new(ray_t.min, closest_hit));
            if ray.hit.t < closest_hit {
                closest_hit = ray.hit.t;
            }
        }
    }
}
