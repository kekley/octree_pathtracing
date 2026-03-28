pub mod hit_record;
pub mod path_tracer;
pub mod ray;
pub mod vector_extensions;

use crate::path_tracing::{hit_record::AABBHitRecord, ray::Ray};

pub trait Intersect {
    type HitRecord;
    fn intersect(&self, ray: &Ray) -> Option<Self::HitRecord>;
}
