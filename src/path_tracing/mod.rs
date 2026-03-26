pub mod hit_record;
pub mod path_tracer;
pub mod ray;
pub mod vector_extensions;

use crate::path_tracing::{hit_record::HitRecord, ray::Ray};

pub trait Intersect {
    fn intersect(&self, ray: Ray) -> HitRecord;
}
