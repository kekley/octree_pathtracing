use core::f32;
use std::f32::{consts::PI, INFINITY};

use crate::{angle_distance, random_float};
use rand::rngs::StdRng;

use glam::{Mat3A, Vec3A, Vec4};

use super::{
    hittable::HitRecord,
    material::Material,
    scene::{Scene, Sun},
};

#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub(crate) origin: Vec3A,
    direction: Vec3A,
    inv_dir: Vec3A,
    pub(crate) distance_travelled: f32,
    pub(crate) hit: HitRecord,
}

impl Ray {
    pub const EPSILON: f32 = 0.00000005;
    pub const OFFSET: f32 = 0.000001;

    #[inline]
    pub fn at(&self, t: f32) -> Vec3A {
        self.origin + self.direction * t
    }
    #[inline]
    pub fn new(point: Vec3A, direction: Vec3A) -> Self {
        const EPSILON: f32 = 1e-6;
        let inv_dir = Vec3A::new(
            if direction.x.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.x
            },
            if direction.y.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.y
            },
            if direction.z.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.z
            },
        );
        Self {
            origin: point,
            direction,
            hit: HitRecord::default(),
            distance_travelled: 0.0,
            inv_dir: inv_dir,
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            origin: self.origin,
            direction: self.direction,
            distance_travelled: 0.0,
            hit: HitRecord {
                t: 0.0,
                t_next: 0.0,
                u: self.hit.u,
                v: self.hit.v,
                current_material: self.hit.current_material.clone(),
                normal: self.hit.normal,
                previous_material: self.hit.previous_material.clone(),
                color: Vec4::ZERO,
                depth: self.hit.depth,
                specular: self.hit.specular,
            },
            inv_dir: self.inv_dir.clone(),
        }
    }

    pub fn set_normal(&mut self, normal: Vec3A) {
        self.hit.normal = normal;
    }

    pub fn orient_normal(&mut self, normal: Vec3A) {
        if self.direction.dot(normal) > 0.0 {
            self.hit.normal = -normal;
        } else {
            self.hit.normal = normal;
        }
        //self.hit.geom_normal = normal;
    }

    pub fn get_direction(&self) -> &Vec3A {
        &self.direction
    }
    pub fn get_inverse_direction(&self) -> &Vec3A {
        &self.inv_dir
    }
    pub fn set_direction(&mut self, direction: Vec3A) {
        const EPSILON: f32 = 1e-6;
        let inv_dir = Vec3A::new(
            if direction.x.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.x
            },
            if direction.y.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.y
            },
            if direction.z.abs() < EPSILON {
                1.0 / EPSILON
            } else {
                1.0 / direction.z
            },
        );
        self.direction = direction;
        self.inv_dir = inv_dir;
    }
    pub fn specular_reflection(&self, roughness: f32, rng: &mut StdRng) -> Self {
        let mut tmp = Ray {
            origin: self.origin,
            direction: self.direction,
            distance_travelled: 0.0,
            hit: HitRecord {
                t: INFINITY,
                t_next: INFINITY,
                u: 0.0,
                v: 0.0,
                current_material: self.hit.current_material.clone(),
                normal: self.hit.normal,
                previous_material: self.hit.previous_material.clone(),
                color: Vec4::ZERO,
                depth: self.hit.depth,
                specular: self.hit.specular,
            },
            inv_dir: self.inv_dir,
        };
        tmp.hit.current_material = tmp.hit.previous_material.clone();

        if roughness > Ray::EPSILON {
            let mut specular_dir = self.direction;
            let s = -2.0 * self.direction.dot(self.hit.normal);
            let d = self.hit.normal;
            let o = self.direction;

            specular_dir = s * d + o;

            let x1 = random_float(rng);
            let x2 = random_float(rng);
            let r = x1.sqrt();
            let theta = 2.0 * PI * x2;

            let tx = r * theta.cos();
            let ty = r * theta.sin();
            let tz = (1.0 - x1).sqrt();

            let tangent: Vec3A;
            if tmp.hit.normal.x.abs() > 0.1 {
                tangent = Vec3A::new(0.0, 1.0, 0.0);
            } else {
                tangent = Vec3A::new(1.0, 0.0, 0.0);
            }

            let u = tangent.cross(tmp.hit.normal).normalize();
            let v = tmp.hit.normal.cross(u);

            let rotation_matrix = Mat3A::from_cols(u, v, tmp.hit.normal);

            let new_dir = rotation_matrix * Vec3A::new(tx, ty, tz);

            tmp.direction = new_dir * roughness + specular_dir * (1.0 - roughness);
            tmp.direction = tmp.direction.normalize();
            tmp.origin = tmp.at(Ray::OFFSET);
        } else {
            tmp.set_direction(
                self.direction - 2.0 * self.direction.dot(self.hit.normal) * self.hit.normal,
            );

            tmp.origin = tmp.at(Ray::OFFSET);
        }

        if tmp.hit.normal.dot(tmp.direction).signum() == tmp.hit.normal.dot(self.direction).signum()
        {
            let factor = tmp.hit.normal.dot(self.direction) * -Ray::EPSILON
                - tmp.direction.dot(tmp.hit.normal);
            tmp.direction += factor * tmp.hit.normal;
            tmp.direction = tmp.direction.normalize();
        }

        tmp
    }

    pub fn scatter_normal(&mut self, rng: &mut StdRng) {
        let x1 = random_float(rng);
        let x2 = random_float(rng);

        let r = x1.sqrt();
        let theta = 2.0 * PI * x2;

        let tangent = if self.hit.normal.x.abs() > 0.1 {
            Vec3A::new(0.0, 1.0, 0.0)
        } else {
            Vec3A::new(1.0, 0.0, 0.0)
        };

        let u = tangent.cross(self.hit.normal).normalize();
        let v = self.hit.normal.cross(u);

        let rotation_matrix = Mat3A::from_cols(u, v, self.hit.normal);

        let new_dir =
            rotation_matrix * Vec3A::new(r * theta.cos(), r * theta.sin(), (1.0 - x1).sqrt());

        self.set_direction(new_dir);
        self.origin = self.at(Ray::OFFSET);
    }

    pub fn diffuse_reflection(&mut self, ray: &mut Ray, rng: &mut StdRng, scene: &Scene) {
        *self = ray.clone();

        let normal = self.hit.normal;
        if !normal.is_finite() {
            dbg!(normal);
        }
        let mut x1 = random_float(rng);
        let mut x2 = random_float(rng);

        let mut r = x1.sqrt();
        let mut theta = 2.0 * PI * x2;

        let mut tx = r * theta.cos();
        let mut ty = r * theta.sin();
        let tz: f32;

        if scene.sun_sampling_strategy.importance_sampling {
            let sun_az = scene.sun.azimuth;
            let sun_alt_fake = scene.sun.altitude;
            let sun_alt = if sun_alt_fake.abs() > PI / 2.0 {
                sun_alt_fake.signum() * PI - sun_alt_fake
            } else {
                sun_alt_fake
            };
            let sun_dx = sun_az.cos() * sun_alt.cos();
            let sun_dz = sun_az.sin() * sun_alt.cos();
            let sun_dy = sun_alt.sin();

            let (mut sun_tx, mut sun_ty, sqrt): (f32, f32, f32);
            let sun_tz = sun_dx * normal.x + sun_dy * normal.y + sun_dz * normal.z;
            if normal.x.abs() > 0.1 {
                sun_tx = sun_dx * normal.z - sun_dz * normal.x;
                sun_ty = sun_dx * normal.x * normal.y
                    - sun_dy * (normal.x * normal.x + normal.z * normal.z)
                    + sun_dz * normal.y * normal.z;
                sqrt = normal.x.hypot(normal.z);
            } else {
                sun_tx = sun_dz * normal.y - sun_dy * normal.z;
                sun_ty = sun_dy * normal.x * normal.y
                    - sun_dx * (normal.y * normal.y + normal.z * normal.z)
                    + sun_dz * normal.x * normal.z;
                sqrt = normal.z.hypot(normal.y);
            }

            sun_tx /= sqrt;
            sun_ty /= sqrt;

            let circle_radius = scene.sun.radius * scene.sun.importance_sample_radius;
            let mut sample_chance = scene.sun.importance_sample_chance;

            let sun_alt_relative = sun_tz.asin();
            // check if there is any chance of the sun being visible
            if sun_alt_relative + circle_radius > Ray::EPSILON {
                // if the sun is not at too shallow of an angle, then sample a circular region
                if sun_tx.hypot(sun_ty) + circle_radius + Ray::EPSILON < 1.0 {
                    if random_float(rng) < sample_chance {
                        tx = sun_tx + tx * circle_radius;
                        ty = sun_ty + ty * circle_radius;
                        // diminish the contribution of the ray based on the circle area and the sample chance
                        ray.hit.color *= circle_radius * circle_radius / sample_chance;
                        // non-sun sampling
                        // now, rather than guaranteeing that the ray is cast within a circle, instead guarantee that it does not
                    } else {
                        while (tx - sun_tx).hypot(ty - sun_ty) < circle_radius {
                            tx -= sun_tx;
                            ty -= sun_ty;
                            if tx == 0.0 && ty == 0.0 {
                                break;
                            }
                            tx /= circle_radius;
                            ty /= circle_radius;
                        }

                        ray.hit.color *=
                            (1.0 - circle_radius * circle_radius) / (1.0 - sample_chance);
                    }
                } else {
                    // the sun is at a shallow angle, so instead we're using a "rectangular-ish segment"
                    // it is important that we sample from a shape which we can easily calculate the area of
                    let min_r = (sun_alt_relative + circle_radius).cos();
                    let max_r = ((sun_alt_relative - circle_radius).max(0.0)).cos();

                    let sun_theta = sun_ty.atan2(sun_tx);
                    let segment_area_proportion =
                        ((max_r * max_r - min_r * min_r) * circle_radius) / PI;
                    sample_chance *= segment_area_proportion / (circle_radius * circle_radius);
                    sample_chance = sample_chance.min(Sun::MAX_IMPORTANCE_SAMPLE_CHANCE);
                    if random_float(rng) < sample_chance {
                        r = (min_r * min_r * x1 + max_r * max_r * (1.0 - x1)).sqrt();
                        theta = sun_theta + (2.0 * x2 - 1.0) * circle_radius;
                        tx = r * theta.cos();
                        ty = r * theta.sin();

                        ray.hit.color *= segment_area_proportion / sample_chance;
                    } else {
                        while r > min_r
                            && r < max_r
                            && angle_distance(theta, sun_theta) < circle_radius
                        {
                            x1 = random_float(rng);
                            x2 = random_float(rng);
                            r = x1.sqrt();
                            theta = 2.0 * PI * x2;
                        }
                        tx = r * theta.cos();
                        ty = r * theta.sin();
                        ray.hit.color *= (1.0 - segment_area_proportion) / (1.0 - sample_chance);
                    }
                }
            }
        }

        tz = (1.0 - tx * tx - ty * ty).sqrt();

        let (xx, xy, xz): (f32, f32, f32);
        let (mut ux, mut uy, mut uz): (f32, f32, f32);
        let (vx, vy, vz): (f32, f32, f32);

        if normal.x.abs() > 0.1 {
            xx = 0.0;
            xy = 1.0;
            xz = 0.0
        } else {
            xx = 1.0;
            xy = 0.0;
            xz = 0.0;
        }

        ux = xy * normal.z - xz * normal.y;
        uy = xz * normal.x - xx * normal.z;
        uz = xx * normal.y - xy * normal.x;

        r = 1.0 / (ux * ux + uy * uy + uz * uz).sqrt();

        ux *= r;
        uy *= r;
        uz *= r;

        vx = uy * normal.z - uz * normal.y;
        vy = uz * normal.x - ux * normal.z;
        vz = ux * normal.y - uy * normal.x;

        let mut direction = Vec3A::default();
        direction.x = ux * tx + vx * ty + normal.x * tz;
        direction.y = uy * tx + vy * ty + normal.y * tz;
        direction.z = uz * tx + vz * ty + normal.z * tz;

        self.set_direction(direction);

        self.origin = self.at(Ray::OFFSET);
        //dbg!("new_dir: {:?}", ray.direction);

        self.hit.current_material = self.hit.previous_material.clone();
        self.hit.specular = false;

        if (normal.dot(self.direction)).signum() == (normal.dot(ray.direction)).signum() {
            let factor = normal.dot(ray.direction).signum() * -Ray::EPSILON
                - self.direction.dot(self.hit.normal);
            self.direction += normal * factor;
            self.direction = self.direction.normalize();
        }
    }
}
