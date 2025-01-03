use std::f32::INFINITY;

use crate::{
    aabb::AABB,
    axis::{Axis, AxisOps},
    ray::Ray,
    scene, Scene, Texture,
};
use glam::Vec3A as Vec3;

pub enum Face {
    Top,
    Bottom,
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone)]
pub struct Cuboid {
    pub bbox: AABB,
    pub textures: [u16; 6],
}
pub const EPSILON: f32 = 0.00000000001;

impl Cuboid {
    pub fn get_bbox(&self) -> AABB {
        self.bbox.clone()
    }
    pub fn new(bbox: AABB, material_idx: u32) -> Self {
        Self {
            bbox,
            textures: [material_idx as u16; 6],
        }
    }

    pub fn hit(&self, ray: &mut Ray) -> bool {
        ray.hit.t = INFINITY;
        let t = self.bbox.intersects(ray);
        if t == INFINITY {
            return false;
        } else {
            true
        }
    }
    pub fn intt(&self, ray: &mut Ray) -> bool {
        let mut t_min = -INFINITY;
        let mut t_max = INFINITY;
        for &axis in Axis::iter() {
            let box_axis_min = self.bbox.get_interval(axis).min;
            let box_axis_max = self.bbox.get_interval(axis).max;
            let ray_axis_origin = ray.origin.get_axis(axis);
            let ray_axis_dir_inverse = (1.0 / ray.direction).get_axis(axis);

            let t0 = (box_axis_min - ray_axis_origin) * ray_axis_dir_inverse;
            let t1 = (box_axis_max - ray_axis_origin) * ray_axis_dir_inverse;

            if t0 < t1 {
                t_max = t_max.min(t1);
                t_min = t_min.max(t0);
            } else {
                t_max = t_max.min(t0);
                t_min = t_min.max(t1);
            }
            if t_max <= t_min {
                return false;
            }
        }

        let point = ray.at(t_min);
        let mut u = 0.0;
        let mut v = 0.0;
        let mut normal = Vec3::ZERO;
        let mut mat_index: usize = 0;
        Axis::iter()
            .find(|&&axis| {
                let distance_to_min =
                    (point.get_axis(axis) - self.bbox.get_interval(axis).min).abs();
                let distance_to_max =
                    (point.get_axis(axis) - self.bbox.get_interval(axis).max).abs();
                distance_to_min < EPSILON || distance_to_max < EPSILON
            })
            .map(|&axis| {
                let distance_to_min =
                    (point.get_axis(axis) - self.bbox.get_interval(axis).min).abs();
                let is_min_face = distance_to_min < EPSILON;

                match (is_min_face, axis) {
                    (true, Axis::X) => {
                        mat_index = 2;
                        normal = Vec3::new(-1.0, 0.0, 0.0);
                        u = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                    (true, Axis::Y) => {
                        mat_index = 1;
                        normal = Vec3::new(0.0, -1.0, 0.0);
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                    }
                    (true, Axis::Z) => {
                        mat_index = 5;
                        normal = Vec3::new(0.0, 0.0, -1.0);
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                    (false, Axis::X) => {
                        mat_index = 3;
                        normal = Vec3::new(1.0, 0.0, 0.0);
                        u = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                    (false, Axis::Y) => {
                        mat_index = 0;
                        normal = Vec3::new(0.0, 1.0, 0.0);
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                    }
                    (false, Axis::Z) => {
                        mat_index = 4;
                        normal = Vec3::new(0.0, 0.0, 1.0);
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                };
            });

        ray.hit.u = u;
        ray.hit.v = v;
        ray.hit.t_next = t_min;
        ray.hit.outward_normal = normal;
        ray.hit.current_material = 0;
        return true;
    }

    pub fn intersect(&self, ray: &mut Ray, scene: &Scene) -> bool {
        let mut hit = false;
        ray.hit.t = INFINITY;
        if self.intt(ray) {
            hit = Self::intersect_texture(ray, scene, self.textures[0]);
        }
        hit
    }

    pub fn intersect_texture(ray: &mut Ray, scene: &Scene, material_idx: u16) -> bool {
        let color = scene.materials[material_idx as usize].albedo.value(
            ray.hit.u,
            ray.hit.v,
            &ray.at(ray.hit.t_next),
        );
        if color.w > Ray::EPSILON {
            assert!(color.w == 1.0);
            ray.hit.color = color;
            true
        } else {
            println!("something went wrong");
            false
        }
    }
}
