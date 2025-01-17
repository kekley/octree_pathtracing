use fastrand::Rng;
use glam::Vec3A;
use rayon::prelude::*;
use std::{cmp::max, io::Write, sync::atomic::AtomicU32};

use crate::{
    bvh::BVHTree,
    defocus_disk_sample,
    hittable::{HitList, Hittable},
    interval::Interval,
    ray::Ray,
    sample_square,
    util::{self, degrees_to_rads, random_float, random_in_unit_disk},
};

#[derive(Debug, Clone)]
pub struct Camera {
    pub aspect_ratio: f32,
    pub image_width: u32,
    pub v_fov: f32,
    pub look_from: Vec3A,
    pub look_at: Vec3A,
    pub v_up: Vec3A,
    pub defocus_angle: f32,
    pub focus_dist: f32,
    image_height: u32,
    center: Vec3A,
    pixel00_loc: Vec3A,
    pixel_delta_u: Vec3A,
    pixel_delta_v: Vec3A,
    u: Vec3A,
    v: Vec3A,
    w: Vec3A,
    defocus_disk_u: Vec3A,
    defocus_disk_v: Vec3A,
}

impl Camera {
    pub fn new(
        look_from: Vec3A,
        look_at: Vec3A,
        image_width: u32,
        fov: f32,
        aspect_ratio: f32,
    ) -> Self {
        let mut a = Self {
            aspect_ratio: aspect_ratio,
            image_width: image_width,
            image_height: 0,
            center: Vec3A::ZERO,
            pixel00_loc: Vec3A::ZERO,
            pixel_delta_u: Vec3A::ZERO,
            pixel_delta_v: Vec3A::ZERO,
            v_fov: fov,
            look_at: look_at,
            look_from: look_from,
            v_up: Vec3A::new(0.0, 1.0, 0.0),
            u: Vec3A::ZERO,
            v: Vec3A::ZERO,
            w: Vec3A::ZERO,
            defocus_angle: 0.0,
            focus_dist: 10.0,
            defocus_disk_u: Vec3A::ZERO,
            defocus_disk_v: Vec3A::ZERO,
        };
        a.initialize();
        a
    }

    fn initialize(&mut self) {
        self.image_height = max((self.image_width as f32 / self.aspect_ratio) as u32, 1);

        self.center = self.look_from;

        //viewport
        let theta = degrees_to_rads(self.v_fov);
        let h = f32::tan(theta / 2.0);
        let viewport_height = 2.0 * h * self.focus_dist;
        let viewport_width = viewport_height * (self.image_width as f32 / self.image_height as f32);

        self.w = (self.look_from - self.look_at).normalize();
        self.u = self.v_up.cross(self.w).normalize();
        self.v = self.w.cross(self.u);
        // uv vectors
        let viewport_u = viewport_width * self.u;
        let viewport_v = viewport_height * -self.v;

        self.pixel_delta_u = viewport_u / self.image_width as f32;
        self.pixel_delta_v = viewport_v / self.image_height as f32;

        let viewport_upper_left =
            self.center - (self.focus_dist * self.w) - viewport_u / 2.0 - viewport_v / 2.0;

        self.pixel00_loc = viewport_upper_left + 0.5 * (self.pixel_delta_u + self.pixel_delta_v);

        let defocus_radius = self.focus_dist * f32::tan(degrees_to_rads(self.defocus_angle / 2.0));
        self.defocus_disk_u = self.u * defocus_radius;
        self.defocus_disk_v = self.v * defocus_radius;
    }

    pub fn get_ray(&self, rng: &mut Rng, x: u32, y: u32) -> Ray {
        let offset = Vec3A::ZERO;

        let pixel_sample = self.pixel00_loc
            + ((x as f32 + offset.x * 0.1) * self.pixel_delta_u)
            + ((y as f32 + offset.y * 0.1) * self.pixel_delta_v);

        let ray_origin = match self.defocus_angle <= 0.0 {
            true => self.center,
            false => {
                defocus_disk_sample(rng, self.center, self.defocus_disk_u, self.defocus_disk_v)
            }
        };
        let ray_direction = (pixel_sample - ray_origin).normalize();

        Ray::new(ray_origin, ray_direction)
    }
}
