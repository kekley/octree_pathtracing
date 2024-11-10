use crate::{hittable::HitRecord, ray::Ray, vec3::Vec3};

#[derive(Debug, Clone)]
pub enum Material {
    Lambertian { albedo: Vec3 }, // vec3:color
    Metal { albedo: Vec3, fuzz: f64 },
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
    pub fn scatter(&self, ray_in: &Ray, hit_record: &HitRecord) -> Option<Scatter> {
        match self {
            Material::Lambertian { albedo } => {
                let scatter_direction = hit_record.normal + Vec3::random_unit_vec();
                let scattered_ray = match scatter_direction.near_zero() {
                    true => Ray::new(hit_record.point, hit_record.normal),
                    false => Ray::new(hit_record.point, scatter_direction),
                };

                let color = *albedo;

                Some(Scatter::new(scattered_ray, color))
            }
            Material::Metal { albedo, fuzz } => {
                let mut reflected_direction = ray_in.direction.reflect(hit_record.normal);
                reflected_direction =
                    reflected_direction.normalize() + (fuzz * Vec3::random_unit_vec());
                let scattered_ray = Ray::new(hit_record.point, reflected_direction);
                let color = *albedo;

                if scattered_ray.direction.dot(hit_record.normal) > 0f64 {
                    Some(Scatter::new(scattered_ray, color))
                } else {
                    None
                }
            }
        }
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::Lambertian {
            albedo: Vec3::splat(0f64),
        }
    }
}
