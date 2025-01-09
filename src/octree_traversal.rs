use std::f32::INFINITY;

use glam::{UVec3, Vec3, Vec3A, Vec4};

use crate::{
    axis::{Axis, AxisOps},
    Cuboid, HitRecord, Material, OctantId, Octree, Ray, Texture, AABB,
};

impl Octree<u32> {
    const MAX_STEPS: usize = 1000;
    const MAX_SCALE: u32 = 23;
    const EPSILON: f32 = 0.00000011920929;

    pub fn intersect_octree(
        &self,
        ray: &mut Ray,
        max_dst: f32,
        do_translucency: bool,
        palette: &Vec<Cuboid>,
        materials: &Vec<Material>,
    ) -> bool {
        let mut stack: Vec<(OctantId, f32)> = Vec::new();
        let (mut t_min, mut t_max) = (0.0, INFINITY);
        let mut pos = Vec3A::ZERO;
        let mut size = 2f32.powi(self.depth() as i32) as u32;
        let (t0, t1) = Self::project_cube(&pos, size, &ray);
        t_min = t0.max_element();
        t_max = t0.min_element();
        let mut h = t_max;
        let mut parent = self.root.unwrap();

        let mut child_idx = Self::select_child(&mut pos, &mut size, ray, t_min);

        for _ in 0..1000 {
            let (t0, t1) = Self::project_cube(&pos, size, ray);
            let t_corner = t1;
            let t_corner_max = t_corner.min_element();

            let child = &self.octants[parent as usize].children[child_idx as usize];

            let is_leaf = child.is_leaf();
            let is_octant = !child.is_none();

            if is_octant && t_max <= t_max {
                if is_leaf && t_min == 0.0 {
                    //FIXME: inside voxel
                }
                if is_leaf && t_min > 0.0 {
                    println!("hit!");

                    let val = child.get_leaf_value().unwrap();

                    println!("value: {}", val);
                } else {
                    //descend
                    let tv_max = t_corner_max.min(t_max);

                    if t_min <= tv_max {
                        //push

                        if t_corner_max < h {
                            stack.push((parent, t_min));
                        }
                        h = t_corner_max;

                        parent = child.get_octant_value().unwrap();

                        child_idx = Self::select_child(&mut pos, &mut size, ray, t_min);

                        t_max = tv_max;

                        continue;
                    }
                }
            } else {
                //adjacent leaf count
            }

            //advance
            let mut old_pos = pos;
            let mut size_copy = size;

            let possible_child = Self::select_child(&mut pos, &mut size_copy, ray, t_min);

            t_min = t_corner_max;

            child_idx ^= possible_child;

            if (child_idx & possible_child) != 0 {
                //pop
            }
        }
        false
    }
    pub fn select_child(origin: &mut Vec3A, size: &mut u32, ray: &Ray, t_min: f32) -> (u8) {
        *size /= 2;
        let (mins, maxs) = Self::project_cube(&origin, *size, ray);
        let t_cmax = maxs.min_element();
        let mut idx: u8 = 0;
        for &axis in Axis::iter() {
            let a = maxs.get_axis(axis);
            idx |= 1 << axis as usize;
            if a == t_cmax {
                println!("advance! axis{:?}", axis);
                println!("idx: {}", idx);
                origin[axis as usize] = origin.get_axis(axis) + *size as f32;
            };
        }
        idx
    }
    pub fn project_cube(origin: &Vec3A, size: u32, ray: &Ray) -> (Vec3A, Vec3A) {
        let box_min = origin;
        let box_max = *origin + size as f32;
        let ray_origin = ray.origin;
        let ray_inv_dir = ray.inv_dir;
        let t_bot = (box_min - ray_origin) * ray_inv_dir;
        let t_top = (box_max - ray_origin) * ray_inv_dir;

        let mins = t_bot.min(t_top);
        let maxs = t_bot.max(t_top);

        let t0 = mins.max_element();
        let t1 = maxs.min_element();

        //println!("t0: {}, t1:{}", t0, t1);
        (mins, maxs)
    }
    pub fn oct_test() {
        let mut octree: Octree<u32> = Octree::new();
        octree.set_leaf(UVec3::new(4, 0, 12), 0);
        let start = Vec3A::ZERO + 1.0;
        let end = Vec3A::new(4.1, 0.1, 12.1);

        let dir = (end - start).normalize();

        let mut ray = Ray {
            origin: start,
            direction: dir,
            inv_dir: 1.0 / dir,
            distance_travelled: 0.0,
            hit: HitRecord::default(),
        };

        let materials = vec![Material::default()];
        let palette = vec![Cuboid {
            bbox: AABB::new(start, start + 1.0),
            textures: [0u16; 6],
        }];
        let res = octree.intersect_octree(&mut ray, 100.0, false, &palette, &materials);
        println!("{}", res);
    }
}
