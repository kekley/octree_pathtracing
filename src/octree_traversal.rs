#![allow(clippy::style)]

use std::f32::{EPSILON, INFINITY, NEG_INFINITY};

use glam::{UVec3, Vec2, Vec3A};
pub const OCTREE_MAX_STEPS: u32 = 1000;
pub const OCTREE_MAX_SCALE: i32 = 23;
pub const OCTREE_EPSILON: f32 = 1.1920929e-7;
use crate::{
    axis::{self, Axis, AxisOps},
    util, Cuboid, Face, HitRecord, Material, Octant, OctantId, Octree, Position, Ray, AABB,
};
impl Position for Vec3A {
    fn construct(x: u32, y: u32, z: u32) -> Self {
        Self::new(x as f32, y as f32, z as f32)
    }

    fn idx(&self) -> u8 {
        let u_vec: UVec3 = UVec3::new(self.x as u32, self.y as u32, self.z as u32);
        let val: u8 = (u_vec.x + u_vec.y * 2 + u_vec.z * 4) as u8;
        val
    }

    fn required_depth(&self) -> u8 {
        let depth = self.max_element();
        depth.log2().floor() as u8 + 1
    }

    fn x(&self) -> u32 {
        self.x as u32
    }

    fn y(&self) -> u32 {
        self.y as u32
    }

    fn z(&self) -> u32 {
        self.z as u32
    }

    fn div(&self, rhs: u32) -> Self {
        *self / rhs as f32
    }

    fn rem_assign(&mut self, rhs: u32) {
        *self %= rhs as f32;
    }
}

impl<T: PartialEq> Octree<T> {
    pub fn intersect_octree(&self, ray: &Ray, max_dst: f32) -> Option<(&T, Vec3A)> {
        let tree_root = if self.root.is_some() {
            self.root.unwrap()
        } else {
            println!("no root");
            return None;
        };
        let octree_scale = f32::exp2(-(self.depth() as f32)); //scale factor for putting the size of the octree in the range[ 0-1]
        let mut stack: [Option<(OctantId, f32)>; OCTREE_MAX_SCALE as usize + 1] =
            [None; OCTREE_MAX_SCALE as usize + 1];
        let mut ro = Vec3A::new(
            ray.origin.x as f32,
            ray.origin.y as f32,
            ray.origin.z as f32,
        ) * octree_scale;

        let mut rd: Vec3A = Vec3A::new(
            ray.direction.x as f32,
            ray.direction.y as f32,
            ray.direction.z as f32,
        );

        let max_dst = max_dst * octree_scale;

        ro += 1.0; // shift the coordinates to [1-2)

        let mut parent_octant_idx = tree_root;

        let mut scale: i32 = OCTREE_MAX_SCALE - 1;
        let mut scale_exp2: f32 = 0.5f32; //exp2(scale-MAX_SCALE)

        let mut last_leaf_value: Option<&T> = None;
        let mut adjacent_leaf_count: u32 = 0;

        let sign_mask: u32 = 1 << 31;
        let epsilon_bits_without_sign: u32 = OCTREE_EPSILON.to_bits() & !sign_mask;

        Axis::iter().for_each(|&axis| {
            if rd[axis as usize].abs() < OCTREE_EPSILON {
                rd[axis as usize] = f32::from_bits(
                    epsilon_bits_without_sign | rd[axis as usize].to_bits() & sign_mask,
                );
            }
        });

        let t_coef = 1.0 / -rd.abs();
        let mut t_bias = t_coef * ro;

        let mut mirror_mask: u32 = 0;

        Axis::iter().for_each(|&axis| {
            if rd[axis as usize] > 0.0 {
                mirror_mask ^= 1 << axis as usize;
                t_bias[axis as usize] = 3.0 * t_coef[axis as usize] - t_bias[axis as usize];
            }
        });

        let mut t_min = (2.0 * t_coef - t_bias).max_element().max(0.0);

        let mut t_max = (t_coef - t_bias).min_element().min(0.0);

        let mut h: f32 = t_max;

        let mut idx: u32 = 0;

        let mut pos: Vec3A = Vec3A::splat(1.0);

        Axis::iter().for_each(|&axis| {
            if t_min < 1.5 * t_coef[axis as usize] - t_bias[axis as usize] {
                idx ^= 1 << axis as usize;
                pos[axis as usize] = 1.5;
            }
        });

        while scale < OCTREE_MAX_SCALE {
            if max_dst >= 0.0 && t_min > max_dst {
                return None;
            }

            let t_corner = pos * t_coef - t_bias;

            let tc_max = t_corner.min_element();

            let unmirrored_idx = idx ^ mirror_mask;

            let child =
                &self.octants[(parent_octant_idx) as usize].children[unmirrored_idx as usize];

            if !child.is_none() && t_min <= t_max {
                if child.is_leaf() && t_min == 0.0 {
                    println!("inside block");
                    //println!("ray origin: {}", ray.origin);
                    //println!("ray dir: {}", ray.direction);
                    return None;
                }

                if child.is_leaf() && t_min > 0.0 {
                    //println!("hit");
                    //println!("pos: {:?},t_min:{}, current_parent: {}, unmirrored_idx: {}, scale: {}, is_child:{}, is_leaf: {}",ray.origin+ray.direction*(t_min/octree_scale),t_min/octree_scale,parent_octant_idx,unmirrored_idx,scale,is_child,is_leaf);
                    let leaf_value = self.octants[parent_octant_idx as usize].children
                        [unmirrored_idx as usize]
                        .get_leaf_value()
                        .unwrap();

                    let t_corner = (pos + scale_exp2) * t_coef - t_bias;
                    let tc_min = t_corner.max_element();

                    let mut voxel_pos = pos;
                    Axis::iter().for_each(|&axis| {
                        if mirror_mask & (1 << axis as usize) != 0 {
                            voxel_pos[axis as usize] = 3.0 - scale_exp2 - voxel_pos[axis as usize];
                        }
                    });

                    return Some((leaf_value, (voxel_pos - 1.0) / octree_scale));
                } else {
                    let half_scale = scale_exp2 * 0.5;

                    let t_center = half_scale * t_coef + t_corner;

                    let tv_max = t_max.min(tc_max);

                    if t_min <= tv_max {
                        //push
                        //println!("push!");
                        if !child.is_octant() {
                            break;
                        }
                        if tc_max < h {
                            stack[scale as usize] = Some((parent_octant_idx, t_max));
                        }
                        h = tc_max;

                        parent_octant_idx = child.get_octant_value().unwrap();
                        scale -= 1;
                        scale_exp2 = half_scale;

                        idx = 0;
                        Axis::iter().for_each(|&axis| {
                            if t_min < t_center[axis as usize] {
                                idx ^= 1 << axis as usize;
                                pos[axis as usize] += scale_exp2;
                            }
                        });

                        t_max = tv_max;
                        continue;
                    }
                }
            } else {
            }
            //advance
            //println!("advance!");

            let mut step_mask = 0;
            Axis::iter().for_each(|&axis| {
                if tc_max >= t_corner[axis as usize] {
                    step_mask ^= 1 << axis as usize;
                    pos[axis as usize] -= scale_exp2;
                }
            });

            t_min = tc_max;
            idx ^= step_mask;

            if (idx & step_mask) != 0 {
                //println!("pop!");
                let mut differing_bits: u32 = 0;

                Axis::iter().for_each(|&axis| {
                    if (step_mask & (1 << axis as usize)) != 0 {
                        differing_bits |= pos[axis as usize].to_bits()
                            ^ (pos[axis as usize] + scale_exp2).to_bits()
                    }
                });

                //find msb
                scale = util::find_msb(differing_bits as i32);
                //println!("{:b}", differing_bits);
                scale_exp2 = f32::exp2(((scale - OCTREE_MAX_SCALE + 127) << 23) as f32);

                if scale >= OCTREE_MAX_SCALE {
                    return None;
                }
                (parent_octant_idx, t_max) =
                    stack[scale as usize].expect("popped empty traversal stack");

                //let (mut shx, mut shy, mut shz): (u32, u32, u32) = (0, 0, 0);

                let mut sh = UVec3::splat(0);

                Axis::iter().for_each(|&axis| {
                    sh[axis as usize] = pos[axis as usize].to_bits() >> scale;
                    pos[axis as usize] = f32::from_bits(sh[axis as usize] << scale);
                });

                /*                 shx = pos.x.to_bits() >> scale;
                shy = pos.y.to_bits() >> scale;
                shz = pos.z.to_bits() >> scale;

                pos.x = f32::from_bits(shx << scale);
                pos.y = f32::from_bits(shy << scale);
                pos.z = f32::from_bits(shz << scale); */

                idx = (sh.x & 1) | ((sh.y & 1) << 1) | ((sh.z & 1) << 2);
                h = 0.0;
            }
        }
        return None;
    }
}
