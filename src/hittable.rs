use crate::{
    aabb::AABB,
    bvh::{BVHNode, BVHTree},
    interval::Interval,
    material::Material,
    ray::Ray,
    sphere::Sphere,
    vec3::Vec3,
};
#[derive(Debug)]
pub struct HitRecord<'a> {
    pub point: Vec3,
    pub normal: Vec3,
    pub t: f64,
    pub u: f64,
    pub v: f64,
    pub front_face: bool,
    pub material: &'a Material,
}

impl HitRecord<'_> {
    pub fn set_face_normal(&mut self, ray: &Ray, outward_normal: Vec3) {
        self.front_face = ray.direction.dot(outward_normal) < 0f64;
        self.normal = match self.front_face {
            true => outward_normal,
            false => -outward_normal,
        }
    }
}
#[derive(Debug, Clone)]
pub enum Hittable {
    Sphere(Sphere),
}

impl Hittable {
    #[inline]
    pub fn hit(&self, ray: &Ray, ray_t: Interval) -> Option<HitRecord> {
        match self {
            Hittable::Sphere(sphere) => sphere.hit(ray, ray_t),
        }
    }
    #[inline]
    pub fn get_bbox(&self) -> &AABB {
        match self {
            Hittable::Sphere(sphere) => &sphere.bbox,
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
    pub fn hit(&self, ray: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let mut ret_val = None;
        let mut closest_hit = ray_t.max;

        for object in &self.objects {
            let rec = object.hit(ray, Interval::new(ray_t.min, closest_hit));
            match rec {
                Some(rec) => {
                    closest_hit = rec.t;
                    ret_val = Some(rec);
                }
                None => {}
            }
        }

        ret_val
    }
}
