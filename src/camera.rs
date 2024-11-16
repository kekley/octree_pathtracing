use fastrand::Rng;
use rayon::prelude::*;
use std::{cmp::max, io::Write, sync::atomic::AtomicU32};

use crate::{
    hittable::{HitList, Hittable},
    interval::Interval,
    ray::Ray,
    util::{
        degrees_to_rads, random_float, random_in_unit_disk, write_rgb8_color_as_text_to_stream,
    },
    vec3::Vec3,
};

#[derive(Debug, Clone)]
pub struct Camera {
    pub aspect_ratio: f64,
    pub image_width: u64,
    pub samples_per_pixel: u64,
    pub max_depth: i64,
    pub v_fov: f64,
    pub look_from: Vec3,
    pub look_at: Vec3,
    pub v_up: Vec3,
    pub defocus_angle: f64,
    pub focus_dist: f64,
    image_height: u64,
    center: Vec3,
    pixel00_loc: Vec3,
    pixel_delta_u: Vec3,
    pixel_delta_v: Vec3,
    pixel_sample_scale: f64,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    defocus_disk_u: Vec3,
    defocus_disk_v: Vec3,
}

impl Camera {
    const SKY_COLOR: Vec3 = Vec3::new(0.5, 0.7, 1.0);

    pub fn new() -> Self {
        Self {
            aspect_ratio: 1.0,
            image_width: 100,
            image_height: 0,
            samples_per_pixel: 10,
            center: Vec3::ZERO,
            pixel00_loc: Vec3::ZERO,
            pixel_delta_u: Vec3::ZERO,
            pixel_delta_v: Vec3::ZERO,
            pixel_sample_scale: 0.0,
            max_depth: 0,
            v_fov: 90.0,
            look_at: Vec3::new(0.0, 0.0, -1.0),
            look_from: Vec3::ZERO,
            v_up: Vec3::new(0.0, 1.0, 0.0),
            u: Vec3::ZERO,
            v: Vec3::ZERO,
            w: Vec3::ZERO,
            defocus_angle: 0.0,
            focus_dist: 10.0,
            defocus_disk_u: Vec3::ZERO,
            defocus_disk_v: Vec3::ZERO,
        }
    }
    pub fn render(&mut self, world: Box<Hittable>) -> Vec<u8> {
        self.initialize();
        let mut buf = Vec::with_capacity((self.image_height * self.image_height * 11) as usize);
        buf.write(format!("P3\n{}\n{}\n255\n", self.image_width, self.image_height,).as_bytes())
            .unwrap();
        let mut rng = Rng::new();
        for y in 0..self.image_height {
            for x in 0..self.image_width {
                let mut pixel_color = Vec3::ZERO;
                for _ in 0..self.samples_per_pixel {
                    let ray = self.get_ray(&mut rng, x, y);
                    pixel_color += Self::ray_color(&mut rng, &ray, self.max_depth, &world);
                }
                pixel_color = pixel_color * self.pixel_sample_scale;
                write_rgb8_color_as_text_to_stream(&pixel_color, &mut buf);
            }
        }
        buf
    }
    pub fn multi_threaded_render(mut self, world: Box<Hittable>) -> Vec<u8> {
        self.initialize();
        let mut buf = Vec::with_capacity((self.image_height * self.image_width * 11) as usize);
        buf.write(format!("P3\n{}\n{}\n255\n", self.image_width, self.image_height).as_bytes())
            .unwrap();
        let rows_done = AtomicU32::new(0);
        // Collect pixel data in a nested Vec for each row
        let rows: Vec<Vec<Vec3>> = (0..self.image_height)
            .into_par_iter()
            .map(|y| {
                let mut rng = Rng::new();
                let res = (0..self.image_width)
                    .into_iter()
                    .map(|x| {
                        let mut pixel_color = Vec3::ZERO;
                        for _ in 0..self.samples_per_pixel {
                            let ray = Camera::thread_safe_get_ray(
                                self.center,
                                self.pixel_delta_u,
                                self.pixel_delta_v,
                                self.pixel00_loc,
                                self.defocus_angle,
                                self.defocus_disk_u,
                                self.defocus_disk_v,
                                &mut rng,
                                x,
                                y,
                            );
                            pixel_color +=
                                Camera::ray_color(&mut rng, &ray, self.max_depth, &world);
                        }
                        pixel_color * self.pixel_sample_scale
                    })
                    .collect();
                rows_done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                res
            })
            .collect();

        for row in &rows {
            for color in row {
                write_rgb8_color_as_text_to_stream(&color, &mut buf);
            }
        }

        buf
    }

    fn initialize(&mut self) {
        self.image_height = max((self.image_width as f64 / self.aspect_ratio) as u64, 1);

        self.pixel_sample_scale = 1f64 / self.samples_per_pixel as f64;

        self.center = self.look_from;

        //viewport
        let theta = degrees_to_rads(self.v_fov);
        let h = f64::tan(theta / 2.0);
        let viewport_height = 2.0 * h * self.focus_dist;
        let viewport_width = viewport_height * (self.image_width as f64 / self.image_height as f64);

        self.w = (self.look_from - self.look_at).normalize();
        self.u = self.v_up.cross(self.w).normalize();
        self.v = self.w.cross(self.u);
        // uv vectors
        let viewport_u = viewport_width * self.u;
        let viewport_v = viewport_height * -self.v;

        self.pixel_delta_u = viewport_u / self.image_width as f64;
        self.pixel_delta_v = viewport_v / self.image_height as f64;

        let viewport_upper_left =
            self.center - (self.focus_dist * self.w) - viewport_u / 2.0 - viewport_v / 2.0;

        self.pixel00_loc = viewport_upper_left + 0.5 * (self.pixel_delta_u + self.pixel_delta_v);

        let defocus_radius = self.focus_dist * f64::tan(degrees_to_rads(self.defocus_angle / 2.0));
        self.defocus_disk_u = self.u * defocus_radius;
        self.defocus_disk_v = self.v * defocus_radius;
    }

    fn ray_color(rng: &mut Rng, ray: &Ray, depth: i64, world: &Hittable) -> Vec3 {
        if depth <= 0 {
            return Vec3::splat(0f64);
        }

        let color = match world.hit(&ray, Interval::ZEROISH_TO_INFINITY) {
            Some(hit_record) => match hit_record.material.scatter(rng, ray, &hit_record) {
                Some(scatter) => {
                    scatter.color * Self::ray_color(rng, &scatter.ray, depth - 1, world)
                }
                None => Vec3::ZERO,
            },
            None => {
                let normalized_dir = ray.direction.normalize();

                let a = 0.5f64 * (normalized_dir.y + 1f64);

                (1.0 - a) * Vec3::ONE + a * Camera::SKY_COLOR
            }
        };
        color
    }

    fn get_ray(&self, rng: &mut Rng, x: u64, y: u64) -> Ray {
        // Construct a camera ray originating from the defocus disk and directed at a randomly
        // sampled point around the pixel location i, j.
        let offset = Self::sample_square(rng);

        let pixel_sample = self.pixel00_loc
            + ((x as f64 + offset.x) * self.pixel_delta_u)
            + ((y as f64 + offset.y) * self.pixel_delta_v);

        let ray_origin = match self.defocus_angle <= 0.0 {
            true => self.center,
            false => Self::defocus_disk_sample(
                rng,
                self.center,
                self.defocus_disk_u,
                self.defocus_disk_v,
            ),
        };
        let ray_direction = pixel_sample - ray_origin;

        Ray::new(ray_origin, ray_direction)
    }

    fn defocus_disk_sample(rng: &mut Rng, center: Vec3, disc_u: Vec3, disc_v: Vec3) -> Vec3 {
        let p = random_in_unit_disk(rng);
        center + (p.x * disc_u) + (p.y * disc_v)
    }

    fn thread_safe_get_ray(
        center: Vec3,
        pixel_delta_u: Vec3,
        pixel_delta_v: Vec3,
        pixel00_loc: Vec3,
        defocus_angle: f64,
        disc_u: Vec3,
        disc_v: Vec3,
        rng: &mut Rng,
        x: u64,
        y: u64,
    ) -> Ray {
        let offset = Self::sample_square(rng);

        let pixel_sample = pixel00_loc
            + ((x as f64 + offset.x) * pixel_delta_u)
            + ((y as f64 + offset.y) * pixel_delta_v);

        let ray_origin = match defocus_angle <= 0.0 {
            true => center,
            false => Self::defocus_disk_sample(rng, center, disc_u, disc_v),
        };
        let ray_direction = pixel_sample - ray_origin;
        let ray_time = random_float(rng);

        Ray::create_at(ray_origin, ray_direction, ray_time)
    }
    #[inline]

    fn sample_square(rng: &mut Rng) -> Vec3 {
        Vec3::new(random_float(rng) - 0.5f64, random_float(rng) - 0.5f64, 0.0)
    }
}
