use std::f32::INFINITY;

pub type HittableIdx = u32;
use crate::{
    aabb::AABB, bvh::BVHTree, cuboid::Cuboid, interval::Interval, ray::Ray, sphere::Sphere,
    Material,
};
use glam::Vec3A as Vec3;

#[derive(Debug, Clone)]
pub struct HittableBVH {
    bvh: Box<BVHTree>,
}

impl HittableBVH {
    pub fn new(bvh: BVHTree) -> Self {
        let bbox = *bvh.bbox();

        Self {
            bvh: Box::from(bvh),
        }
    }

    pub fn hit(&self, ray: &mut Ray) -> bool {
        self.bvh.hit(ray);
    }
    pub fn bbox(&self) -> AABB {
        *self.bvh.bbox()
    }
}

#[derive(Debug, Clone)]
pub struct HittableHitList {
    hit_list: Box<HitList>,
}

impl HittableHitList {
    pub fn new(hitlist: HitList) -> Self {
        let bbox = hitlist.bbox.clone();
        Self {
            hit_list: Box::from(hitlist),
        }
    }

    pub fn hit(&self, ray: &mut Ray, ray_t: Interval) {
        self.hit_list.hit(ray, ray_t);
    }
    pub fn bbox(&self) -> AABB {
        self.hit_list.bbox
    }
}

#[derive(Debug, Clone)]
pub struct HitRecord {
    pub t: f32,
    pub u: f32,
    pub v: f32,
    pub current_material: u32,
    pub outward_normal: Vec3,
    pub geom_normal: Vec3,
    pub previous_material: u32,
}

impl Default for HitRecord {
    fn default() -> Self {
        Self {
            t: INFINITY,
            u: 0.0,
            v: 0.0,
            current_material: 0,
            outward_normal: Vec3::ZERO,
            geom_normal: todo!(),
            previous_material: todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Hittable {
    Sphere(Sphere),
    Box(Cuboid),
    BVHTree(HittableBVH),
    HitList(HittableHitList),
}

impl Hittable {
    #[inline]
    pub fn hit(&self, ray: &mut Ray) -> bool {
        match self {
            Hittable::Sphere(sphere) => sphere.hit(ray),
            Hittable::Box(cuboid) => cuboid.hit(ray),
            Hittable::BVHTree(bvh) => bvh.hit(ray),
            Hittable::HitList(hittable_hit_list) => hittable_hit_list.hit(ray),
        }
    }
    #[inline]
    pub fn get_bbox(&self) -> AABB {
        match self {
            Hittable::Sphere(sphere) => sphere.bbox(),
            Hittable::Box(cuboid) => cuboid.bbox.clone(),
            Hittable::BVHTree(bvh) => bvh.bbox(),
            Hittable::HitList(hittable_hit_list) => hittable_hit_list.bbox(),
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
        self.bbox = AABB::from_aabb(&self.bbox, &object.get_bbox());
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
