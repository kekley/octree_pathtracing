#![allow(clippy::style)]

use std::f32::INFINITY;

use glam::{UVec3, Vec3A, Vec4};
use num_traits::PrimInt;

use crate::{
    axis::{self, Axis, AxisOps},
    interval::Interval,
    Cuboid, HitRecord, Material, OctantId, Octree, Position, Ray, Texture, AABB,
};
impl Position for Vec3A {
    fn construct(pos: [u32; 3]) -> Self {
        Self::new(pos[0] as f32, pos[1] as f32, pos[2] as f32)
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

const MAX_STEPS: u32 = 1000;
const MAX_SCALE: i32 = 23;
const THIS_EPSILON: f32 = 0.00000011920929;
impl Octree<u32> {
    pub fn intersect_octree(
        &self,
        ray: &mut Ray,
        max_dst: f32,
        do_translucency: bool,
        palette: &Vec<Cuboid>,
        materials: &Vec<Material>,
    ) -> bool {
        let octree_scale = f32::exp2(-(self.depth() as f32)); //scale factor for putting the size of the octree in the range[ 0-1]
        let mut stack: [(OctantId, f32); MAX_SCALE as usize + 1] =
            [Default::default(); MAX_SCALE as usize + 1];
        let mut ro = ray.origin * octree_scale;
        let mut rd = ray.direction;
        let max_dst = max_dst * octree_scale;

        ro += 1.0; // shift the coordinates to [1-2)

        let mut parent_octant_idx = self.root.unwrap();

        let mut scale: i32 = MAX_SCALE - 1;
        let mut scale_exp2: f32 = 0.5f32; //exp2(scale-MAX_SCALE)

        let _last_leaf_value = u32::max_value();
        let adjacent_leaf_count = 0;

        let sign_mask: u32 = 1 << 31;
        let epsilon_bits_without_sign: u32 = THIS_EPSILON.to_bits() & !sign_mask;

        Axis::iter().for_each(|&axis| {
            if rd[axis as usize] < THIS_EPSILON {
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

        let mut t_min: f32 = (2.0 * t_coef.x - t_bias.x)
            .max(2.0 * t_coef.y - t_bias.y)
            .max(2.0 * t_coef.z - t_bias.z)
            .max(0.0);

        let mut t_max: f32 = (t_coef.x - t_bias.x)
            .min(t_coef.y - t_bias.y)
            .min(t_coef.z - t_bias.z);

        let mut h: f32 = t_max;

        let mut idx: u32 = 0;

        let mut pos: Vec3A = Vec3A::splat(1.0);

        Axis::iter().for_each(|&axis| {
            if t_min < 1.5 * t_coef[axis as usize] - t_bias[axis as usize] {
                idx ^= 1 << axis as usize;
                pos[axis as usize] = 1.5;
            }
        });

        for i in (0..MAX_STEPS) {
            if max_dst >= 0.0 && t_min > max_dst {
                return false;
            }

            let t_corner = pos * t_coef - t_bias;

            let tc_max = t_corner.min_element();

            let unmirrored_idx = idx ^ mirror_mask;

            let child =
                &self.octants[(parent_octant_idx) as usize].children[unmirrored_idx as usize];

            let is_child = !child.is_none();
            let is_leaf = child.is_leaf();

            //println!("pos: {:?},t_min:{}, current_parent: {}, unmirrored_idx: {}, scale: {}, is_child:{}, is_leaf: {}",ray.origin+ray.direction*(t_min/octree_scale),t_min/octree_scale,parent_octant_idx,unmirrored_idx,scale,is_child,is_leaf);

            if is_child && t_min <= t_max {
                if is_leaf && t_min == 0.0 {
                    println!("inside block");
                    ray.hit.color = Vec4::ONE;
                    return true;
                }

                if is_leaf && t_min > 0.0 {
                    //println!("hit");
                    ray.hit.color = Vec4::new(1.0, 0.0, 1.0, 1.0);
                    return true;
                } else {
                    let half_scale = scale_exp2 * 0.5;

                    let t_center = half_scale * t_coef + t_corner;

                    let tv_max = t_max.min(tc_max);

                    if t_min <= tv_max {
                        //push
                        //println!("push!");
                        if tc_max < h {
                            stack[scale as usize] = (parent_octant_idx, t_max);
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
                //adjacent leaf stuff
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

                if (step_mask & 1) != 0 {
                    differing_bits |= (pos.x.to_bits() ^ (pos.x + scale_exp2).to_bits());
                }
                if (step_mask & 2) != 0 {
                    differing_bits |= (pos.y.to_bits() ^ (pos.y + scale_exp2).to_bits());
                }
                if (step_mask & 4) != 0 {
                    differing_bits |= (pos.z.to_bits() ^ (pos.z + scale_exp2).to_bits());
                }

                //find msb
                scale = find_msb_old(differing_bits).unwrap_or(-1);
                //println!("{:b}", differing_bits);
                scale_exp2 = f32::exp2((scale - MAX_SCALE) as f32);

                if scale >= MAX_SCALE {
                    return false;
                }
                (parent_octant_idx, t_max) = stack[scale as usize];

                let (mut shx, mut shy, mut shz): (u32, u32, u32) = (0, 0, 0);

                shx = pos.x.to_bits() >> scale;
                shy = pos.y.to_bits() >> scale;
                shz = pos.z.to_bits() >> scale;

                pos.x = f32::from_bits(shx << scale);
                pos.y = f32::from_bits(shy << scale);
                pos.z = f32::from_bits(shz << scale);

                idx = (shx & 1) | ((shy & 1) << 1) | ((shz & 1) << 2);
                h = 0.0;
            }
        }
        return false;
    }
    pub fn oct_test() {
        let mut octree: Octree<u32> = Octree::new();
        octree.set_leaf(UVec3::new(4, 1, 12), 0);
        let start = Vec3A::ZERO + 1.0;
        let end = Vec3A::new(4.0, 1.0, 12.0);

        let dir1 = ((end - 0.1) - start).normalize();
        let dir2 = ((end + 0.1) - start).normalize();
        println!("dir1: {}, dir2: {}", dir1, dir2);

        let mut ray = Ray {
            origin: start,
            direction: dir1,
            inv_dir: 1.0 / dir1,
            distance_travelled: 0.0,
            hit: HitRecord::default(),
        };

        let mut ray2 = Ray {
            origin: start,
            direction: dir2,
            inv_dir: 1.0 / dir2,
            distance_travelled: 0.0,
            hit: HitRecord::default(),
        };

        let materials = vec![Material::default()];
        let palette = vec![Cuboid {
            bbox: AABB::new(start, start + 1.0),
            textures: [0u16; 6],
        }];
        let a = octree.intersect_octree(&mut ray, 1000.0, false, &palette, &materials);
        let b = octree.intersect_octree(&mut ray2, 1000.0, false, &palette, &materials);
        //let res = octree.new_intersect(&mut ray, 100.0, false, &palette, &materials);
        //println!("{}", res);
        println!("-.5:{} , +0.5:{}", a, b);
    }
}

fn find_msb_old(value: u32) -> Option<i32> {
    if value == 0 {
        return None;
    }
    let mut msb: u32 = 31; // u32 has 32 bits
    while (value & (1 << msb)) == 0 {
        msb -= 1;
    }
    let a: u32 = 0b110000000000000000000;

    Some(msb as i32)
}
fn find_msb(mut x: i32) -> i32 {
    let mut res = -1;
    if x < 0 {
        x = !x;
    }
    for i in 0..32 {
        let mask = 0x80000000u32 as i32 >> i;
        if x & mask != 0 {
            res = 31 - i;
            break;
        }
    }
    res
}
