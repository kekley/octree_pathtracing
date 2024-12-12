use std::f32::INFINITY;

use crate::{
    aabb::AABB,
    hittable::HitRecord,
    interval::Interval,
    material::{self, Material},
    ray::Ray,
    vec3::{Axis, Vec3},
    TextureManager,
};

pub enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Front,
    Back,
}
#[derive(Debug, Clone)]
pub struct Cuboid {
    pub bbox: AABB,
    material_idx: u16,
}
pub const EPSILON: f32 = 0.00000000001;

impl Cuboid {
    pub fn get_bbox(&self) -> AABB {
        self.bbox.clone()
    }
    pub fn new(bbox: AABB, material_idx: u16) -> Self {
        Self { bbox, material_idx }
    }
    pub fn new_multi_texture(bbox: AABB, materials_idx: u16) -> Self {
        Self {
            bbox,
            material_idx: materials_idx,
        }
    }

    pub fn hit(&self, ray: &mut Ray, ray_t: Interval) {
        let mut interval = ray_t.clone();

        for axis in Axis::iter() {
            let box_axis_min = self.bbox.get_interval(*axis).min;
            let box_axis_max = self.bbox.get_interval(*axis).max;
            let ray_axis_origin = ray.origin.get_axis(*axis);
            let ray_axis_dir_inverse = ray.inv_dir.get_axis(*axis);

            let t0 = (box_axis_min - ray_axis_origin) * ray_axis_dir_inverse;
            let t1 = (box_axis_max - ray_axis_origin) * ray_axis_dir_inverse;

            if t0 < t1 {
                interval.min = t0.max(interval.min);
                interval.max = t1.min(interval.max);
            } else {
                interval.min = t1.max(interval.min);
                interval.max = t0.min(interval.max);
            }
            if interval.max <= interval.min {
                return;
            }
        }

        let point = ray.at(interval.min);
        let mut u = 0.0;
        let mut v = 0.0;
        let mut normal = Vec3::UP;
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
                        normal = Vec3::LEFT;
                        u = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                    (true, Axis::Y) => {
                        mat_index = 1;
                        normal = Vec3::DOWN;
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                    }
                    (true, Axis::Z) => {
                        mat_index = 5;
                        normal = Vec3::BACK;
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                    (false, Axis::X) => {
                        mat_index = 3;
                        normal = Vec3::RIGHT;
                        u = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                    (false, Axis::Y) => {
                        mat_index = 0;
                        normal = Vec3::UP;
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.z - self.bbox.get_interval(Axis::Z).min)
                            / self.bbox.get_interval(Axis::Z).size();
                    }
                    (false, Axis::Z) => {
                        mat_index = 4;
                        normal = Vec3::FORWARD;
                        u = (point.x - self.bbox.get_interval(Axis::X).min)
                            / self.bbox.get_interval(Axis::X).size();
                        v = (point.y - self.bbox.get_interval(Axis::Y).min)
                            / self.bbox.get_interval(Axis::Y).size();
                    }
                };
            });

        ray.hit.u = u;
        ray.hit.v = v;
        ray.hit.t = interval.min;
        ray.hit.outward_normal = normal;
        ray.hit.mat_idx = self.material_idx;
    }
}
