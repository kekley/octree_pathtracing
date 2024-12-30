use std::f32::INFINITY;

use fastrand::Rng;

use crate::{
    near_zero,
    ray::Ray,
    texture::Texture,
    util::{random_float, random_unit_vec},
};
use glam::Vec3A as Vec3;

#[derive(Debug, Clone)]
pub enum Material {
    Lambertian { texture: Texture },
    Metal { texture: Texture, fuzz: f32 },
    Dielectric { refraction_index: f32 },
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
    pub fn scatter(&self, rng: &mut Rng, ray_in: &mut Ray) -> Option<Vec3> {
        match self {
            Material::Lambertian { texture } => {
                let mut scatter_direction = ray_in.hit.outward_normal + random_unit_vec(rng);
                if near_zero(&scatter_direction) {
                    scatter_direction = ray_in.hit.outward_normal;
                }
                let point = ray_in.at(ray_in.hit.t);
                let color = texture.value(ray_in.hit.u, ray_in.hit.v, &point);

                ray_in.origin = point;
                ray_in.direction = scatter_direction;
                ray_in.inv_dir = 1.0 / scatter_direction;
                ray_in.hit.t = INFINITY;

                Some(color)
            }
            Material::Metal { texture, fuzz } => {
                let mut reflected_direction = ray_in.direction.reflect(ray_in.hit.outward_normal);
                reflected_direction =
                    reflected_direction.normalize() + (fuzz * random_unit_vec(rng));
                let point = ray_in.at(ray_in.hit.t);
                ray_in.origin = point;
                ray_in.direction = reflected_direction;
                let color = texture.value(ray_in.hit.u, ray_in.hit.v, &point);

                if ray_in.direction.dot(ray_in.hit.outward_normal) > 0f32 {
                    Some(color)
                } else {
                    None
                }
            }
            Material::Dielectric { refraction_index } => {
                let attenuation = Vec3::ONE;
                let ri = if ray_in.direction.dot(ray_in.hit.outward_normal) > 0.0 {
                    1.0 / refraction_index
                } else {
                    *refraction_index
                };

                let unit_dir = ray_in.direction.normalize();

                let cos_theta = f32::min((-unit_dir).dot(ray_in.hit.outward_normal), 1.0);
                let sin_theta = f32::sqrt(1.0 - cos_theta * cos_theta);

                let cannot_refract = ri * sin_theta > 1.0;

                let direction =
                    if cannot_refract || Self::reflectance(cos_theta, ri) > random_float(rng) {
                        unit_dir.reflect(ray_in.hit.outward_normal)
                    } else {
                        unit_dir.refract(ray_in.hit.outward_normal, ri)
                    };
                ray_in.origin = ray_in.at(ray_in.hit.t);
                ray_in.direction = direction;
                Some(attenuation)
            }
        }
    }

    #[inline]
    fn reflectance(cosine: f32, refraction_index: f32) -> f32 {
        let mut r0 = (1f32 - refraction_index) / (1f32 + refraction_index);
        r0 = r0 * r0;
        r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
    }
}
