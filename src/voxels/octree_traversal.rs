use glam::{UVec3, Vec2, Vec3A};

use crate::{
    ray_tracing::{cuboid::Face, ray::Ray},
    util,
};

use super::octree::{OctantId, Octree, Position};
pub const OCTREE_MAX_STEPS: usize = 1000;
pub const OCTREE_MAX_SCALE: usize = 23;
pub const OCTREE_EPSILON: f32 = 1.1920929e-7;
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

pub struct OctreeIntersectResult<'a, T> {
    pub ty: &'a T,
    pub voxel_position: Vec3A,
    pub hit_position: Vec3A,
    pub uv: Vec2,
    pub face: Face,
}

impl<T: PartialEq + Default + Clone> Octree<T> {
    pub fn intersect_octree(&self, ray: &Ray, max_dst: f32) -> Option<OctreeIntersectResult<T>> {
        let tree_root = if self.root.is_some() {
            self.root.unwrap()
        } else {
            println!("no root");
            return None;
        };
        let octree_scale = f32::exp2(-(self.depth() as f32));
        let mut stack: [(OctantId, f32); OCTREE_MAX_SCALE as usize + 1] =
            [Default::default(); OCTREE_MAX_SCALE as usize + 1];
        let mut ro = ray.origin * octree_scale;

        let mut rd: Vec3A = ray.direction;

        let max_dst = max_dst * octree_scale;

        ro += 1.0; // shift the coordinates to [1-2)

        let mut parent_octant_idx = tree_root;

        let mut scale: i32 = (OCTREE_MAX_SCALE - 1) as i32;
        let mut scale_exp2: f32 = 0.5f32; //exp2(scale-MAX_SCALE)

        let sign_mask: u32 = 1 << 31;
        let epsilon_bits_without_sign: u32 = OCTREE_EPSILON.to_bits() & !sign_mask;

        if rd.x.abs() < OCTREE_EPSILON {
            rd.x = f32::from_bits(epsilon_bits_without_sign | rd.x.to_bits() & sign_mask);
        }
        if rd.y.abs() < OCTREE_EPSILON {
            rd.y = f32::from_bits(epsilon_bits_without_sign | rd.y.to_bits() & sign_mask);
        }
        if rd.z.abs() < OCTREE_EPSILON {
            rd.z = f32::from_bits(epsilon_bits_without_sign | rd.z.to_bits() & sign_mask);
        }

        let t_coef = 1.0 / -rd.abs();
        let mut t_bias = t_coef * ro;

        let mut mirror_mask: u32 = 0;

        if rd.x > 0.0 {
            mirror_mask ^= 1;
            t_bias.x = 3.0 * t_coef.x - t_bias.x;
        }

        if rd.y > 0.0 {
            mirror_mask ^= 2;
            t_bias.y = 3.0 * t_coef.y - t_bias.y;
        }

        if rd.z > 0.0 {
            mirror_mask ^= 4;
            t_bias.z = 3.0 * t_coef.z - t_bias.z;
        }

        let mut t_min = (2.0 * t_coef - t_bias).max_element().max(0.0);

        let mut t_max = (t_coef - t_bias).min_element();

        let mut h: f32 = t_max;

        let mut idx: u32 = 0;

        let mut pos: Vec3A = Vec3A::splat(1.0);

        if t_min < 1.5 * t_coef.x - t_bias.x {
            idx ^= 1;
            pos.x = 1.5
        }
        if t_min < 1.5 * t_coef.y - t_bias.y {
            idx ^= 2;
            pos.y = 1.5
        }
        if t_min < 1.5 * t_coef.z - t_bias.z {
            idx ^= 4;
            pos.z = 1.5
        }

        for _ in 0..OCTREE_MAX_STEPS {
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
                    println!("ray origin: {}", ray.origin);
                    println!("ray dir: {}", ray.direction);
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

                    let mut unmirrored_pos = pos;

                    if (mirror_mask & 1) != 0 {
                        unmirrored_pos.x = 3.0 - scale_exp2 - unmirrored_pos.x;
                    }
                    if (mirror_mask & 2) != 0 {
                        unmirrored_pos.y = 3.0 - scale_exp2 - unmirrored_pos.y;
                    }
                    if (mirror_mask & 4) != 0 {
                        unmirrored_pos.z = 3.0 - scale_exp2 - unmirrored_pos.z;
                    }

                    let face_id;
                    let mut uv;
                    if tc_min == t_corner.x {
                        face_id = (rd.x.to_bits() >> 31) & 1;
                        uv = Vec2::new(
                            (ro.z + rd.z * t_corner.x) - unmirrored_pos.z,
                            (ro.y + rd.y * t_corner.x) - unmirrored_pos.y,
                        ) / scale_exp2;
                        if rd.x > 0.0 {
                            uv.x = 1.0 - uv.x;
                        }
                    } else if tc_min == t_corner.y {
                        face_id = 2 | ((rd.y.to_bits() >> 31) & 1);
                        uv = Vec2::new(
                            (ro.x + rd.x * t_corner.y) - unmirrored_pos.x,
                            (ro.z + rd.z * t_corner.y) - unmirrored_pos.z,
                        ) / scale_exp2;
                        if rd.y > 0.0 {
                            uv.y = 1.0 - uv.y;
                        }
                    } else {
                        face_id = 4 | ((rd.z.to_bits() >> 31) & 1);
                        uv = Vec2::new(
                            (ro.x + rd.x * t_corner.z) - unmirrored_pos.x,
                            (ro.y + rd.y * t_corner.z) - unmirrored_pos.y,
                        ) / scale_exp2;
                        if rd.z < 0.0 {
                            uv.x = 1.0 - uv.x;
                        }
                    }

                    let min = unmirrored_pos + OCTREE_EPSILON;
                    let max = unmirrored_pos + scale_exp2 - OCTREE_EPSILON;
                    let mut hit_position = (ro + t_min * rd)
                        .max(pos + OCTREE_EPSILON)
                        .min(pos + scale_exp2 - OCTREE_EPSILON)
                        .clamp(min, max);
                    hit_position -= 1.0;
                    hit_position /= octree_scale;
                    unmirrored_pos -= 1.0;
                    unmirrored_pos /= octree_scale;

                    return Some(OctreeIntersectResult {
                        ty: leaf_value,
                        voxel_position: unmirrored_pos,
                        hit_position,
                        uv,
                        face: face_id.try_into().unwrap(),
                    });
                } else {
                    let half_scale = scale_exp2 * 0.5;

                    let t_center = half_scale * t_coef + t_corner;

                    let tv_max = t_max.min(tc_max);

                    if t_min <= tv_max && child.is_octant() {
                        if tc_max < h {
                            stack[scale as usize] = (parent_octant_idx, t_max);
                        }
                        h = tc_max;

                        parent_octant_idx = child.get_octant_value().unwrap();
                        scale -= 1;
                        scale_exp2 = half_scale;

                        idx = 0;

                        if t_min < t_center.x {
                            idx ^= 1;
                            pos.x += scale_exp2;
                        }
                        if t_min < t_center.y {
                            idx ^= 2;
                            pos.y += scale_exp2;
                        }
                        if t_min < t_center.z {
                            idx ^= 4;
                            pos.z += scale_exp2;
                        }

                        t_max = tv_max;
                        continue;
                    }
                }
            }
            //advance
            //println!("advance!");

            let mut step_mask = 0;

            if tc_max >= t_corner.x {
                step_mask ^= 1;
                pos.x -= scale_exp2;
            }
            if tc_max >= t_corner.y {
                step_mask ^= 2;
                pos.y -= scale_exp2;
            }
            if tc_max >= t_corner.z {
                step_mask ^= 4;
                pos.z -= scale_exp2;
            }

            t_min = tc_max;
            idx ^= step_mask;

            if (idx & step_mask) != 0 {
                //println!("pop!");
                let mut differing_bits: u32 = 0;

                if (step_mask & 1) != 0 {
                    differing_bits |= pos.x.to_bits() ^ (pos.x + scale_exp2).to_bits();
                }

                if (step_mask & 2) != 0 {
                    differing_bits |= pos.y.to_bits() ^ (pos.y + scale_exp2).to_bits();
                }

                if (step_mask & 4) != 0 {
                    differing_bits |= pos.z.to_bits() ^ (pos.z + scale_exp2).to_bits();
                }

                scale = util::find_msb(differing_bits as i32);
                scale_exp2 = f32::exp2((scale - OCTREE_MAX_SCALE as i32) as f32);

                if scale >= OCTREE_MAX_SCALE as i32 {
                    return None;
                }
                (parent_octant_idx, t_max) = *stack.get(scale as usize).expect("had invalid scale");

                let (shx, shy, shz): (u32, u32, u32);

                shx = pos.x.to_bits() >> scale;
                pos.x = f32::from_bits(shx << scale);

                shy = pos.y.to_bits() >> scale;
                pos.y = f32::from_bits(shy << scale);

                shz = pos.z.to_bits() >> scale;
                pos.z = f32::from_bits(shz << scale);

                idx = (shx & 1) | ((shy & 1) << 1) | ((shz & 1) << 2);
                h = 0.0;
            }
        }
        return None;
    }
}
