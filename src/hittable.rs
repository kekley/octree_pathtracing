use std::f32::INFINITY;

pub type HittableIdx = u32;
use crate::{aabb::AABB, bvh::BVHTree, cuboid::Cuboid, ray::Ray, sphere::Sphere};
use glam::{Vec3A, Vec4};

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
        self.bvh.hit(ray)
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

    pub fn hit(&self, ray: &mut Ray) -> bool {
        self.hit_list.hit(ray)
    }
    pub fn bbox(&self) -> AABB {
        self.hit_list.bbox
    }
}

#[derive(Debug, Clone)]
pub struct HitRecord<T> {
    pub t: f32, // closest hit
    pub t_next: f32,
    pub u: f32,
    pub v: f32,
    pub current_material: T,
    pub normal: Vec3A,
    //    pub geom_normal: Vec3A,
    pub previous_material: T,
    pub color: Vec4,
    pub depth: u32,
    pub specular: bool,
}

impl<T: Default> Default for HitRecord<T> {
    fn default() -> Self {
        Self {
            t: INFINITY,
            u: 0.0,
            v: 0.0,
            current_material: Default::default(),
            normal: Vec3A::ZERO,
            //     geom_normal: Vec3A::ZERO,
            previous_material: Default::default(),
            t_next: INFINITY,
            color: Vec4::ZERO,
            depth: 0,
            specular: true,
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
    pub fn hit(&self, ray: &mut Ray) -> bool {
        //FIXME: This is not the correct way to do this
        let mut closest_hit = ray.hit.t;
        let mut hit = false;
        for object in &self.objects {
            hit = object.hit(ray);
            if ray.hit.t < closest_hit {
                closest_hit = ray.hit.t;
            }
        }
        hit
    }
}
