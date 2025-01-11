#![allow(clippy::style)]

use std::f32::EPSILON;

use glam::{UVec3, Vec2, Vec3, Vec3A, Vec4};
pub const OCTREE_MAX_STEPS: u32 = 1000;
pub const OCTREE_MAX_SCALE: i32 = 23;
pub const OCTREE_EPSILON: f32 = 0.00000011920929;
use crate::{
    axis::{self, Axis},
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
        let mut stack: [(OctantId, f32); OCTREE_MAX_SCALE as usize + 1] =
            [(self.root.unwrap(), 2.0 - OCTREE_EPSILON); OCTREE_MAX_SCALE as usize + 1];
        let mut ro = ray.origin * octree_scale;
        let mut rd = ray.direction;
        let max_dst = max_dst * octree_scale;

        ro += 1.0; // shift the coordinates to [1-2)

        let mut parent_octant_idx = self.root.unwrap();

        let mut scale: i32 = OCTREE_MAX_SCALE - 1;
        let mut scale_exp2: f32 = 0.5f32; //exp2(scale-MAX_SCALE)

        let _last_leaf_value = u32::max_value();
        let adjacent_leaf_count = 0;

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

        /* let mut t_min: f32 = (2.0 * t_coef.x - t_bias.x)
        .max(2.0 * t_coef.y - t_bias.y)
        .max(2.0 * t_coef.z - t_bias.z)
        .max(0.0); */

        let mut t_max = (t_coef - t_bias).min_element();
        /*  let mut t_max: f32 = (t_coef.x - t_bias.x)
                   .min(t_coef.y - t_bias.y)
                   .min(t_coef.z - t_bias.z);
        */

        let mut h: f32 = t_max;

        let mut idx: u32 = 0;

        let mut pos: Vec3A = Vec3A::splat(1.0);

        Axis::iter().for_each(|&axis| {
            if t_min < 1.5 * t_coef[axis as usize] - t_bias[axis as usize] {
                idx ^= 1 << axis as usize;
                pos[axis as usize] = 1.5;
            }
        });

        for i in 0..OCTREE_MAX_STEPS {
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

            if is_child && t_min <= t_max {
                if is_leaf && t_min == 0.0 {
                    println!("inside block");
                    ray.hit.color = Vec4::splat(1.0);
                    return true;
                }

                if is_leaf && t_min > 0.0 {
                    //println!("hit");
                    //println!("pos: {:?},t_min:{}, current_parent: {}, unmirrored_idx: {}, scale: {}, is_child:{}, is_leaf: {}",ray.origin+ray.direction*(t_min/octree_scale),t_min/octree_scale,parent_octant_idx,unmirrored_idx,scale,is_child,is_leaf);
                    let leaf_value = self.octants[parent_octant_idx as usize].children
                        [unmirrored_idx as usize]
                        .get_leaf_value()
                        .unwrap();

                    let t_corner = (pos + scale_exp2) * t_coef - t_bias;

                    let t_corner_min = t_corner.max_element();

                    let mut pos = pos;
                    Axis::iter().for_each(|&axis| {
                        if (mirror_mask & (1 << axis as usize)) != 0 {
                            pos[axis as usize] = 3.0 - scale_exp2 - pos[axis as usize];
                        }
                    });

                    let mut face_id: Face = Face::East;
                    let mut uv = Vec2::splat(0.0);

                    if t_corner_min == t_corner.x {
                        uv = Vec2::new(
                            (ro.z + rd.z * t_corner.x) - pos.z,
                            (ro.y + rd.y * t_corner.x) - pos.y,
                        ) / scale_exp2;
                        face_id = if rd.x > 0.0 {
                            uv.x = 1.0 - uv.x;
                            Face::East
                        } else {
                            Face::West
                        };
                    }
                    if t_corner_min == t_corner.y {
                        uv = Vec2::new(
                            (ro.x + rd.x * t_corner.y) - pos.x,
                            (ro.z + rd.z * t_corner.y) - pos.z,
                        ) / scale_exp2;
                        face_id = if rd.y > 0.0 {
                            uv.y = 1.0 - uv.y;
                            Face::Top
                        } else {
                            Face::Bottom
                        };
                    }
                    if t_corner_min == t_corner.z {
                        uv = Vec2::new(
                            (ro.x + rd.x * t_corner.z) - pos.x,
                            (ro.y + rd.y * t_corner.z) - pos.y,
                        ) / scale_exp2;
                        face_id = if rd.z < 0.0 {
                            uv.x = 1.0 - uv.x;
                            Face::South
                        } else {
                            Face::North
                        };
                    }

                    ray.hit.u = uv.x;
                    ray.hit.v = uv.y;
                    ray.hit.previous_material = ray.hit.previous_material;
                    ray.hit.t_next = t_min / octree_scale;
                    let mat = &materials[0 as usize];
                    Cuboid::intersect_texture(ray, mat);
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

                /* if (step_mask & 1) != 0 {
                    differing_bits |= pos.x.to_bits() ^ (pos.x + scale_exp2).to_bits();
                }
                if (step_mask & 2) != 0 {
                    differing_bits |= pos.y.to_bits() ^ (pos.y + scale_exp2).to_bits();
                }
                if (step_mask & 4) != 0 {
                    differing_bits |= pos.z.to_bits() ^ (pos.z + scale_exp2).to_bits();
                } */

                Axis::iter().for_each(|&axis| {
                    if (step_mask & (1 << axis as usize)) != 0 {
                        differing_bits |= pos[axis as usize].to_bits()
                            ^ (pos[axis as usize] + scale_exp2).to_bits()
                    }
                });

                //find msb
                scale = util::find_msb(differing_bits as i32);
                //println!("{:b}", differing_bits);
                scale_exp2 = f32::exp2((scale - OCTREE_MAX_SCALE) as f32);

                if scale >= OCTREE_MAX_SCALE {
                    return false;
                }
                (parent_octant_idx, t_max) = stack[scale as usize];

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
        return false;
    }
}

impl Octree<Octree<u32>> {
    pub fn intersect_octree(
        &self,
        ray: &mut Ray,
        max_dst: f32,
        do_translucency: bool,
        palette: &Vec<Cuboid>,
        materials: &Vec<Material>,
    ) -> bool {
        let octree_scale = f32::exp2(-(self.depth() as f32)); //scale factor for putting the size of the octree in the range[ 0-1]
        let mut stack: [(OctantId, f32); OCTREE_MAX_SCALE as usize + 1] =
            [(self.root.unwrap(), 2.0 - OCTREE_EPSILON); OCTREE_MAX_SCALE as usize + 1];
        let mut ro = ray.origin * octree_scale;
        let mut rd = ray.direction;
        let max_dst = max_dst * octree_scale;

        ro += 1.0; // shift the coordinates to [1-2)

        let mut parent_octant_idx = self.root.unwrap();

        let mut scale: i32 = OCTREE_MAX_SCALE - 1;
        let mut scale_exp2: f32 = 0.5f32; //exp2(scale-MAX_SCALE)

        let mut _last_leaf_value: Option<&Octree<u32>> = None;
        let mut adjacent_leaf_count = 0;

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

        /* let mut t_min: f32 = (2.0 * t_coef.x - t_bias.x)
        .max(2.0 * t_coef.y - t_bias.y)
        .max(2.0 * t_coef.z - t_bias.z)
        .max(0.0); */

        let mut t_max = (t_coef - t_bias).min_element();
        /*  let mut t_max: f32 = (t_coef.x - t_bias.x)
                   .min(t_coef.y - t_bias.y)
                   .min(t_coef.z - t_bias.z);
        */

        let mut h: f32 = t_max;

        let mut idx: u32 = 0;

        let mut pos: Vec3A = Vec3A::splat(1.0);

        Axis::iter().for_each(|&axis| {
            if t_min < 1.5 * t_coef[axis as usize] - t_bias[axis as usize] {
                idx ^= 1 << axis as usize;
                pos[axis as usize] = 1.5;
            }
        });

        for i in 0..10 {
            if max_dst >= 0.0 && t_min > max_dst {
                return false;
            }

            let t_corner = pos * t_coef - t_bias;

            let tc_max = t_corner.min_element();

            let unmirrored_idx = idx ^ mirror_mask;

            let child =
                &self.octants[(parent_octant_idx) as usize].children[unmirrored_idx as usize];

            let is_child = child.is_octant();
            let is_leaf = child.is_leaf();

            if is_child || is_leaf && t_min <= t_max {
                if is_leaf && t_min == 0.0 {
                    let leaf_value = child.get_leaf_value().unwrap();
                    ray.origin *= 32.0;
                    let hit = leaf_value.intersect_octree(
                        ray,
                        (max_dst / octree_scale) * 32.0,
                        do_translucency,
                        palette,
                        materials,
                    );
                    if hit {
                        return true;
                    }
                    ray.origin /= 32.0;
                }

                if is_leaf && t_min > 0.0 {
                    //println!("hit");
                    //println!("pos: {:?},t_min:{}, current_parent: {}, unmirrored_idx: {}, scale: {}, is_child:{}, is_leaf: {}",ray.origin+ray.direction*(t_min/octree_scale),t_min/octree_scale,parent_octant_idx,unmirrored_idx,scale,is_child,is_leaf);
                    let leaf_value = self.octants[parent_octant_idx as usize].children
                        [unmirrored_idx as usize]
                        .get_leaf_value()
                        .unwrap();

                    let t_corner = (pos + scale_exp2) * t_coef - t_bias;

                    let t_corner_min = t_corner.max_element();

                    let mut pos = pos;
                    Axis::iter().for_each(|&axis| {
                        if (mirror_mask & (1 << axis as usize)) != 0 {
                            pos[axis as usize] = 3.0 - scale_exp2 - pos[axis as usize];
                        }
                    });

                    let mut face_id: Face = Face::East;
                    let mut uv = Vec2::splat(0.0);

                    if t_corner_min == t_corner.x {
                        uv = Vec2::new(
                            (ro.z + rd.z * t_corner.x) - pos.z,
                            (ro.y + rd.y * t_corner.x) - pos.y,
                        ) / scale_exp2;
                        face_id = if rd.x > 0.0 {
                            uv.x = 1.0 - uv.x;
                            Face::East
                        } else {
                            Face::West
                        };
                    }
                    if t_corner_min == t_corner.y {
                        uv = Vec2::new(
                            (ro.x + rd.x * t_corner.y) - pos.x,
                            (ro.z + rd.z * t_corner.y) - pos.z,
                        ) / scale_exp2;
                        face_id = if rd.y > 0.0 {
                            uv.y = 1.0 - uv.y;
                            Face::Top
                        } else {
                            Face::Bottom
                        };
                    }
                    if t_corner_min == t_corner.z {
                        uv = Vec2::new(
                            (ro.x + rd.x * t_corner.z) - pos.x,
                            (ro.y + rd.y * t_corner.z) - pos.y,
                        ) / scale_exp2;
                        face_id = if rd.z < 0.0 {
                            uv.x = 1.0 - uv.x;
                            Face::South
                        } else {
                            Face::North
                        };
                    }

                    ray.hit.u = uv.x;
                    ray.hit.v = uv.y;
                    ray.hit.previous_material = ray.hit.previous_material;
                    ray.hit.t_next = t_min / octree_scale;
                    let mat = &materials[0 as usize];
                    let old_origin = ray.origin;

                    ray.origin *= 32.0;
                    ray.origin = ray.at((t_min / octree_scale) * 32.0);
                    let hit = leaf_value.intersect_octree(
                        ray,
                        (max_dst / octree_scale) * 32.0,
                        do_translucency,
                        palette,
                        materials,
                    );

                    let first_of_kind =
                        adjacent_leaf_count == 0 || Some(leaf_value) != _last_leaf_value;
                    if hit {
                        return true;
                    }
                    ray.origin = old_origin;

                    adjacent_leaf_count += 1;
                    _last_leaf_value = Some(leaf_value);
                } else {
                    let half_scale = scale_exp2 * 0.5;

                    let t_center = half_scale * t_coef + t_corner;

                    let tv_max = t_max.min(tc_max);

                    if t_min <= tv_max && child.get_octant_value().is_some() {
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
                adjacent_leaf_count = 0;
                _last_leaf_value = None;
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

                /* if (step_mask & 1) != 0 {
                    differing_bits |= pos.x.to_bits() ^ (pos.x + scale_exp2).to_bits();
                }
                if (step_mask & 2) != 0 {
                    differing_bits |= pos.y.to_bits() ^ (pos.y + scale_exp2).to_bits();
                }
                if (step_mask & 4) != 0 {
                    differing_bits |= pos.z.to_bits() ^ (pos.z + scale_exp2).to_bits();
                } */

                Axis::iter().for_each(|&axis| {
                    if (step_mask & (1 << axis as usize)) != 0 {
                        differing_bits |= pos[axis as usize].to_bits()
                            ^ (pos[axis as usize] + scale_exp2).to_bits()
                    }
                });

                //find msb
                scale = util::find_msb(differing_bits as i32);
                //println!("{:b}", differing_bits);
                scale_exp2 = f32::exp2((scale - OCTREE_MAX_SCALE) as f32);

                if scale >= OCTREE_MAX_SCALE {
                    return false;
                }
                (parent_octant_idx, t_max) = stack[scale as usize];

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
        return false;
    }
}
