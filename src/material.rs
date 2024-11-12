use fastrand::Rng;

use crate::{
    hittable::HitRecord,
    ray::Ray,
    util::{random_float, random_unit_vec},
    vec3::Vec3,
};

#[derive(Debug, Clone)]
pub enum Material {
    Lambertian { albedo: Vec3 },
    Metal { albedo: Vec3, fuzz: f64 },
    Dielectric { refraction_index: f64 },
}

#[derive(Debug, Default)]
pub struct Scatter {
    pub ray: Ray,
    pub color: Vec3,
}

impl Scatter {
    pub fn new(ray: Ray, color: Vec3) -> Self {
        Self { ray, color }
    }
}

impl Material {
    pub fn scatter(&self, rng: &mut Rng, ray_in: &Ray, hit_record: &HitRecord) -> Option<Scatter> {
        match self {
            Material::Lambertian { albedo } => {
                let mut scatter_direction = hit_record.normal + random_unit_vec(rng);
                if scatter_direction.near_zero() {
                    scatter_direction = hit_record.normal;
                }
                let scattered_ray =
                    Ray::create_at(hit_record.point, scatter_direction, ray_in.time);
                let color = *albedo;

                Some(Scatter::new(scattered_ray, color))
            }
            Material::Metal { albedo, fuzz } => {
                let mut reflected_direction = ray_in.direction.reflect(hit_record.normal);
                reflected_direction =
                    reflected_direction.normalize() + (fuzz * random_unit_vec(rng));
                let scattered_ray =
                    Ray::create_at(hit_record.point, reflected_direction, ray_in.time);
                let color = *albedo;

                if scattered_ray.direction.dot(hit_record.normal) > 0f64 {
                    Some(Scatter::new(scattered_ray, color))
                } else {
                    None
                }
            }
            Material::Dielectric { refraction_index } => {
                let attenuation = Vec3::ONE;
                let ri = match hit_record.front_face {
                    true => 1.0 / refraction_index,
                    false => refraction_index.clone(),
                };

                let unit_dir = ray_in.direction.normalize();

                let cos_theta = f64::min((-unit_dir).dot(hit_record.normal), 1.0);
                let sin_theta = f64::sqrt(1.0 - cos_theta * cos_theta);

                let cannot_refract = ri * sin_theta > 1.0;

                let dir =
                    match cannot_refract || Self::reflectance(cos_theta, ri) > random_float(rng) {
                        true => unit_dir.reflect(hit_record.normal),
                        false => unit_dir.refract(hit_record.normal, ri),
                    };
                let scattered_ray = Ray::create_at(hit_record.point, dir, ray_in.time);
                Some(Scatter::new(scattered_ray, attenuation))
            }
        }
    }
    #[inline]
    fn reflectance(cosine: f64, refraction_index: f64) -> f64 {
        let mut r0 = (1f64 - refraction_index) / (1f64 + refraction_index);
        r0 = r0 * r0;
        r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::Lambertian {
            albedo: Vec3::splat(0f64),
        }
    }
}
