use std::cmp::min;

use crate::{hittable::HitList, interval::Interval, ray::Ray, util::INFINITY, vec3::Vec3};

#[derive(Debug)]
pub struct Camera {
    pub aspect_ratio: f64,
    pub image_width: u64,
    image_height: u64,
    center: Vec3,
    pixel00_loc: Vec3,
    pixel_delta_u: Vec3,
    pixel_delta_v: Vec3,
}

impl Camera {
    const SKY_COLOR: Vec3 = Vec3::new(0.5, 0.7, 1.0);

    pub fn new() -> Self {
        Self {
            aspect_ratio: 1.0,
            image_width: 100,
            image_height: 0,
            center: Vec3::ZERO,
            pixel00_loc: Vec3::ZERO,
            pixel_delta_u: Vec3::ZERO,
            pixel_delta_v: Vec3::ZERO,
        }
    }
    pub fn render(&mut self, world: &HitList) -> Vec<u8> {
        self.initialize();
        let mut buf = vec![0u8; (self.image_height * self.image_width * 11) as usize];
        for y in 0..self.image_height {
            for x in 0..self.image_width {
                let pixel_center = self.pixel00_loc
                    + (x as f64 * self.pixel_delta_u)
                    + (y as f64 * self.pixel_delta_v);
                let ray_direction = pixel_center - self.center;

                let ray = Ray::new(self.center, ray_direction);

                let color = self.ray_color(&ray, &world);
                Vec3::write_rgb8_color_as_text_to_stream(&color, &mut buf);
            }
        }
        buf
    }

    fn initialize(&mut self) {
        self.image_height = min((self.image_width as f64 / self.aspect_ratio) as u64, 1);

        self.center = Vec3::ZERO;

        //viewport
        let focal_length = 1.0;
        let viewport_height = 2.0;
        let viewport_width = viewport_height * (self.image_width as f64 / self.image_height as f64);

        // uv vectors
        let viewport_u = Vec3::new(viewport_width, 0f64, 0f64);
        let viewport_v = Vec3::new(0f64, viewport_height, 0f64);

        self.pixel_delta_u = viewport_u / self.image_width as f64;
        self.pixel_delta_v = viewport_v / self.image_height as f64;

        let viewport_upper_left = self.center
            - Vec3::new(0f64, 0f64, focal_length)
            - viewport_u / 2f64
            - viewport_v / 2f64;

        self.pixel00_loc = viewport_upper_left + 0.5f64 * (self.pixel_delta_u + self.pixel_delta_v);
    }

    fn ray_color(&self, ray: &Ray, world: &HitList) -> Vec3 {
        match world.hit(&ray, Interval::new(0f64, INFINITY)) {
            Some(hit_record) => 0.5 * (hit_record.normal + Vec3::ONE),
            None => {
                let normalized_dir = ray.direction.normalize();

                let a = 0.5f64 * (normalized_dir.y + 1f64);

                (1.0 - a) * Vec3::ONE + a * Camera::SKY_COLOR
            }
        }
    }
}
