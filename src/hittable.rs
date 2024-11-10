use std::{str, sync::Arc};

use crate::{interval::Interval, material::Material, ray::Ray, sphere::Sphere, vec3::Vec3};
#[derive(Debug, Default)]
pub struct HitRecord {
    pub point: Vec3,
    pub normal: Vec3,
    pub t: f64,
    pub front_face: bool,
    pub material: Arc<Material>,
}

impl HitRecord {
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
    pub fn hit(&self, ray: &Ray, ray_t: Interval) -> Option<HitRecord> {
        match self {
            Hittable::Sphere(sphere) => sphere.hit(ray, ray_t),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HitList {
    pub objects: Vec<Hittable>,
}

impl HitList {
    pub fn new() -> Self {
        Self { objects: vec![] }
    }

    pub fn new_with(object: Hittable) -> Self {
        Self {
            objects: vec![object],
        }
    }

    pub fn add(&mut self, object: Hittable) {
        self.objects.push(object);
    }

    pub fn clear(&mut self) {
        self.objects.clear();
    }

    pub fn hit(&self, ray: &Ray, ray_t: Interval) -> Option<HitRecord> {
        let mut temp_record = None;
        let mut closest_hit = ray_t.max;

        for object in &self.objects {
            match object.hit(ray, Interval::new(ray_t.min, closest_hit)) {
                Some(rec) => {
                    closest_hit = rec.t;
                    temp_record = Some(rec);
                }
                None => {}
            }
        }

        temp_record
    }
}
