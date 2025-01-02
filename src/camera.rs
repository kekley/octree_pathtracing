use glam::Vec3A as Vec3;

use rand::{rngs::StdRng, Rng};
use rand_distr::UnitDisc;
use std::f32::{consts::FRAC_PI_6, INFINITY};

use crate::{axis::UP, ray::Ray, HitRecord};

#[derive(Debug, Clone)]
pub struct Camera {
    pub fov: f32,
    pub up: Vec3,
    pub aperture: f32,
    pub focus_dist: f32,
    direction: Vec3,
    position: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: FRAC_PI_6,
            up: UP,
            aperture: 0.0,
            focus_dist: 0.0,
            direction: Vec3::NEG_Z,
            position: Vec3::ZERO,
        }
    }
}

impl Camera {
    pub fn look_at(look_from: Vec3, look_at: Vec3, up: Vec3, fov: f32) -> Self {
        let direction = (look_at - look_from).normalize();
        let up = (up - up.dot(direction) * direction).normalize();

        Self {
            fov: fov,
            up: up,
            aperture: 0.0,
            focus_dist: 0.0,
            direction: direction,
            position: look_from,
        }
    }

    pub fn focus(mut self, focal_point: Vec3, aperture: f32) -> Self {
        self.focus_dist = (focal_point - self.position).dot(self.direction);
        self.aperture = aperture;
        self
    }
    /*     pub fn render(&mut self, world: &Hittable, materials: Arc<TextureManager>) -> Vec<u8> {
           self.initialize();
           let mut buf = Vec::with_capacity((self.image_height * self.image_height * 11) as usize);
           buf.write(format!("P3\n{}\n{}\n255\n", self.image_width, self.image_height,).as_bytes())
               .unwrap();
           let mut rng = Rng::new();
           for y in 0..self.image_height {
               for x in 0..self.image_width {
                   let mut pixel_color = Vec3::ZERO;
                   for _ in 0..self.samples_per_pixel {
                       let mut ray = self.get_ray(&mut rng, x, y);
                       pixel_color +=
                           Self::ray_color(&mut rng, &mut ray, self.max_depth, &world, &materials);
                   }
                   pixel_color = pixel_color * self.pixel_sample_scale;
                   write_rgb8_color_as_text_to_stream(&pixel_color, &mut buf);
               }
           }
           buf
       }
       pub fn multi_threaded_render(
           mut self,
           world: &Hittable,
           materials: &TextureManager,
       ) -> Vec<u8> {
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
                               let mut ray = Camera::thread_safe_get_ray(
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
                               pixel_color += Camera::ray_color(
                                   &mut rng,
                                   &mut ray,
                                   self.max_depth,
                                   &world,
                                   &materials,
                               );
                           }
                           pixel_color * self.pixel_sample_scale
                       })
                       .collect();
                   let prev = rows_done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                   println!(
                       "{}% done.",
                       ((prev + 1) as f32 / self.image_height as f32) * 100.0
                   );
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

       pub fn multi_threaded_render_tiled(
           mut self,
           world: &Hittable,
           materials: &TextureManager,
       ) -> Vec<u8> {
           self.initialize();
           let mut buf = Vec::with_capacity((self.image_height * self.image_width * 11) as usize);
           buf.write(format!("P3\n{}\n{}\n255\n", self.image_width, self.image_height).as_bytes())
               .unwrap();

           // Tile dimensions
           let tile_size = 32;
           let tiles_x = (self.image_width + tile_size - 1) / tile_size;
           let tiles_y = (self.image_height + tile_size - 1) / tile_size;

           let tiles_done = AtomicU32::new(0);

           // Collect pixel data for each tile
           let tiles: Vec<Vec<Vec<Vec3>>> = (0..tiles_y)
               .into_par_iter()
               .map(|tile_y| {
                   (0..tiles_x)
                       .into_par_iter()
                       .map(|tile_x| {
                           let res = (0..tile_size)
                               .flat_map(|dy| {
                                   (0..tile_size).map({
                                       let mut rng = Rng::new();
                                       let mat = materials.clone();

                                       {
                                           move |dx| {
                                               let x = tile_x * tile_size + dx;
                                               let y = tile_y * tile_size + dy;
                                               let mut pixel_color = Vec3::ZERO;
                                               for _ in 0..self.samples_per_pixel {
                                                   let mut ray = Camera::thread_safe_get_ray(
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
                                                   pixel_color += Camera::ray_color(
                                                       &mut rng,
                                                       &mut ray,
                                                       self.max_depth,
                                                       &world,
                                                       &mat,
                                                   );
                                               }
                                               pixel_color * self.pixel_sample_scale
                                           }
                                       }
                                   })
                               })
                               .collect::<Vec<_>>();
                           let prev = tiles_done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                           println!(
                               "{}% done.",
                               ((prev + 1) as f32 / (tiles_x * tiles_y) as f32) * 100.0
                           );
                           res
                       })
                       .collect::<Vec<_>>()
               })
               .collect();

           for tile in &tiles {
               for row in tile {
                   for color in row {
                       write_rgb8_color_as_text_to_stream(color, &mut buf);
                   }
               }
           }

           buf
       }

       fn initialize(&mut self) {
           self.image_height = max((self.image_width as f32 / self.aspect_ratio) as u32, 1);

           self.pixel_sample_scale = 1f32 / self.samples_per_pixel as f32;

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
    */
    /*     fn ray_color(
        rng: &mut Rng,
        ray: &mut Ray,
        depth: i64,
        world: &Hittable,
        materials: &TextureManager,
    ) -> Vec3 {
        if depth <= 0 {
            return Vec3::splat(0f32);
        }

        world.hit(ray, Interval::ZEROISH_TO_INFINITY);

        if ray.hit.t == INFINITY {
            let normalized_dir = ray.direction.normalize();
            let a = 0.5f32 * (normalized_dir.y + 1f32);
            return (1.0 - a) * Vec3::ONE + a * Camera::SKY_COLOR;
        }

        if let Some(color) = materials.get_material(ray.hit.mat_idx).scatter(rng, ray) {
            return color * Self::ray_color(rng, ray, depth - 1, world, materials);
        }

        Vec3::ZERO
    } */

    pub fn get_ray(&self, rng: &mut StdRng, x: u32, y: u32) -> Ray {
        let distance_to_image_plane = (self.fov / 2.0).tan().recip();

        let right = self.direction.cross(self.up).normalize();

        let mut origin = self.position;
        let mut new_dir =
            distance_to_image_plane * self.direction + x as f32 * right + y as f32 * self.up;

        if self.aperture > 0.0 {
            let focal_point = origin + new_dir.normalize() * self.focus_dist;
            let [x, y]: [f32; 2] = rng.sample(UnitDisc);
            origin += (x * right + y * self.up) * self.aperture;
            new_dir = focal_point - origin;
        }
        Ray {
            origin: origin,
            direction: new_dir,
            hit: HitRecord::default(),
            distance_travelled: 0.0,
        }
    }
}
