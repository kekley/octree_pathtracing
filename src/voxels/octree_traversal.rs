use glam::{UVec3, Vec2, Vec3A};

use crate::{
    ray_tracing::{
        cuboid::{Cuboid, Face},
        material::Material,
        quad::Quad,
        ray::Ray,
        resource_manager::ResourceModel,
    },
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

impl Octree<ResourceModel> {
    pub fn intersect_octree_path_tracer(
        &self,
        ray: &mut Ray,
        max_dst: f32,
        materials: &[Material],
        quads: &[Quad],
    ) -> bool {
        let tree_root = match self.root {
            Some(root) => root,
            None => {
                println!("no root");
                return false;
            }
        };
        let octree_scale = self.octree_scale;
        let mut stack: [(OctantId, f32); OCTREE_MAX_SCALE as usize + 1] =
            [Default::default(); OCTREE_MAX_SCALE as usize + 1];
        let mut ro = ray.origin * octree_scale;

        let mut rd: Vec3A = *ray.get_direction();

        let max_dst = max_dst * octree_scale;

        ro += 1.0; // shift the coordinates to [1-2)

        let mut octant_index = tree_root;

        let mut scale: u32 = (OCTREE_MAX_SCALE - 1) as u32;
        let mut scale_exp2: f32 = 0.5f32; //exp2(scale-MAX_SCALE)

        let sign_mask: u32 = 1 << 31;
        let epsilon_bits_without_sign: u32 = OCTREE_EPSILON.to_bits() & !sign_mask;
        let rd_abs = rd.abs();
        let b_vec = rd_abs.cmplt(Vec3A::splat(OCTREE_EPSILON));

        (0..3).for_each(|i| {
            if b_vec.test(i) {
                rd[i] = f32::from_bits(epsilon_bits_without_sign | rd[i].to_bits() & sign_mask)
            }
        });

        let t_coef = 1.0 / -rd_abs;

        let mut t_bias = t_coef * ro;

        let b_vec = rd.cmpgt(Vec3A::ZERO);
        let mirror_mask = b_vec.bitmask();

        (0..3).for_each(|i| {
            if b_vec.test(i) {
                t_bias[i] = 3.0 * t_coef[i] - t_bias[i];
            }
        });
        let mut t_min = (2.0 * t_coef - t_bias).max_element().max(0.0);

        let mut t_max = (t_coef - t_bias).min_element();

        let mut h: f32 = t_max;

        let mut index: u32 = 0;

        let mut pos: Vec3A = Vec3A::splat(1.0);
        let upper = 1.5 * t_coef - t_bias;
        let b_vec = upper.cmpgt(Vec3A::splat(t_min));
        let bitmask = b_vec.bitmask();
        index ^= bitmask;

        (0..3).for_each(|i: usize| {
            if b_vec.test(i) {
                pos[i] = 1.5;
            }
        });

        for _ in 0..OCTREE_MAX_STEPS {
            if max_dst >= 0.0 && t_min > max_dst {
                return false;
            }

            let t_corner = pos * t_coef - t_bias;

            let tc_max = t_corner.min_element();

            let unmirrored_child_index = index ^ mirror_mask;

            let child = &self.octants[octant_index as usize].children
                [unmirrored_child_index as usize]
                .clone();

            if !child.is_none() && t_min <= t_max {
                if child.is_leaf() && t_min >= 0.0 {
                    //println!("hit");
                    //println!("pos: {:?},t_min:{}, current_parent: {}, unmirrored_idx: {}, scale: {}, is_child:{}, is_leaf: {}",ray.origin+ray.direction*(t_min/octree_scale),t_min/octree_scale,parent_octant_idx,unmirrored_idx,scale,is_child,is_leaf);
                    let leaf_value = child.get_leaf_value().unwrap();
                    let model = *leaf_value;

                    let mut unmirrored_pos = pos;
                    (0..3).for_each(|i: usize| {
                        if mirror_mask & 1 << i != 0 {
                            unmirrored_pos[i] = 3.0 - scale_exp2 - unmirrored_pos[i]
                        }
                    });

                    let t_corner = (pos + scale_exp2) * t_coef - t_bias;
                    let tc_min = t_corner.max_element();

                    let face_id;
                    let mut uv;
                    let b_vec = t_corner.cmpeq(Vec3A::splat(tc_min));
                    let b_vec_rd = rd.cmplt(Vec3A::ZERO);
                    if b_vec.test(0) {
                        face_id = 1 << 0 | (rd[0].to_bits() >> 31) & 1;
                        uv = Vec2::new(
                            (ro[2] + rd[2] * t_corner[0]) - unmirrored_pos[2],
                            (ro[1] + rd[1] * t_corner[0]) - unmirrored_pos[1],
                        ) / scale_exp2;
                        if b_vec_rd.test(0) {
                            uv[0] = 1.0 - uv[0];
                        }
                    } else if b_vec.test(1) {
                        face_id = 1 << 1 | ((rd[1].to_bits() >> 31) & 1);
                        uv = Vec2::new(
                            (ro[0] + rd[0] * t_corner[1]) - unmirrored_pos[0],
                            (ro[2] + rd[2] * t_corner[1]) - unmirrored_pos[2],
                        ) / scale_exp2;
                        if b_vec_rd.test(1) {
                            uv[1] = 1.0 - uv[1];
                        }
                    } else {
                        face_id = 1 << 2 | ((rd[2].to_bits() >> 31) & 1);
                        uv = Vec2::new(
                            (ro[0] + rd[0] * t_corner[2]) - unmirrored_pos[0],
                            (ro[1] + rd[1] * t_corner[2]) - unmirrored_pos[1],
                        ) / scale_exp2;
                        if b_vec_rd.test(2) {
                            uv[0] = 1.0 - uv[0];
                        }
                    }

                    match model {
                        ResourceModel::SingleBlock(single_block_model) => {
                            if !(t_min == 0.0) {
                                if single_block_model.intersect(
                                    ray,
                                    t_min / octree_scale,
                                    face_id.try_into().unwrap(),
                                    &uv,
                                    quads,
                                    materials,
                                ) {
                                    return true;
                                }
                            }
                        }
                        ResourceModel::Quad(quad_model) => {
                            unmirrored_pos -= 1.0;
                            unmirrored_pos /= octree_scale;
                            if quad_model.intersect(ray, &unmirrored_pos, t_min, quads, materials) {
                                return true;
                            } else {
                            }
                        }
                    }
                } else {
                    let half_scale = scale_exp2 * 0.5;

                    let t_center = half_scale * t_coef + t_corner;

                    let tv_max = t_max.min(tc_max);

                    if t_min <= tv_max && child.is_octant() {
                        if tc_max < h {
                            stack[scale as usize] = (octant_index, t_max);
                        }
                        h = tc_max;

                        octant_index = child.get_octant_value().unwrap();
                        scale -= 1;
                        scale_exp2 = half_scale;

                        index = 0;
                        let b_vec = t_center.cmpgt(Vec3A::splat(t_min));
                        index ^= b_vec.bitmask();
                        (0..3).for_each(|i: usize| {
                            if b_vec.test(i) {
                                pos[i] += scale_exp2;
                            }
                        });
                        t_max = tv_max;
                        continue;
                    }
                }
            }
            //advance
            //println!("advance!");

            let mut step_mask = 0;

            let b_vec = t_corner.cmple(Vec3A::splat(tc_max));
            step_mask ^= b_vec.bitmask();
            (0..3).for_each(|i: usize| {
                if b_vec.test(i) {
                    pos[i] -= scale_exp2;
                }
            });

            t_min = tc_max;
            index ^= step_mask;

            if (index & step_mask) != 0 {
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

                scale = util::find_msb_u32(differing_bits);
                scale_exp2 = f32::exp2((scale as i32 - OCTREE_MAX_SCALE as i32) as f32);

                if scale >= OCTREE_MAX_SCALE as u32 {
                    return false;
                }
                (octant_index, t_max) = *stack.get(scale as usize).expect("had invalid scale");

                let (shx, shy, shz): (u32, u32, u32);

                shx = pos.x.to_bits() >> scale;
                pos.x = f32::from_bits(shx << scale);

                shy = pos.y.to_bits() >> scale;
                pos.y = f32::from_bits(shy << scale);

                shz = pos.z.to_bits() >> scale;
                pos.z = f32::from_bits(shz << scale);

                index = (shx & 1) | ((shy & 1) << 1) | ((shz & 1) << 2);
                h = 0.0;
            }
        }
        return false;
    }
}

impl Octree<ResourceModel> {
    pub fn intersect_octree_preview(
        &self,
        ray: &mut Ray,
        max_dst: f32,
        materials: &[Material],
        quads: &[Quad],
    ) -> bool {
        let tree_root = match self.root {
            Some(root) => root,
            None => {
                println!("no root");
                return false;
            }
        };
        let octree_scale = self.octree_scale;

        let mut stack: [(OctantId, f32); OCTREE_MAX_SCALE as usize + 1] =
            [Default::default(); OCTREE_MAX_SCALE as usize + 1];
        let mut ro = ray.origin * octree_scale;

        let mut rd: Vec3A = *ray.get_direction();

        let max_dst = max_dst * octree_scale;

        ro += 1.0; // shift the coordinates to [1-2)

        let mut parent_octant_idx = tree_root;

        let mut scale: u32 = (OCTREE_MAX_SCALE - 1) as u32;
        let mut scale_exp2: f32 = 0.5f32; //exp2(scale-MAX_SCALE)

        let sign_mask: u32 = 1 << 31;
        let epsilon_bits_without_sign: u32 = OCTREE_EPSILON.to_bits() & !sign_mask;
        let rd_abs = rd.abs();
        let b_vec = rd_abs.cmplt(Vec3A::splat(OCTREE_EPSILON));

        (0..3).for_each(|i| {
            if b_vec.test(i) {
                rd[i] = f32::from_bits(epsilon_bits_without_sign | rd[i].to_bits() & sign_mask)
            }
        });

        let t_coef = 1.0 / -rd_abs;
        let mut t_bias = t_coef * ro;

        let b_vec = rd.cmpgt(Vec3A::ZERO);
        let mirror_mask = b_vec.bitmask();

        (0..3).for_each(|i| {
            if b_vec.test(i) {
                t_bias[i] = 3.0 * t_coef[i] - t_bias[i];
            }
        });

        let mut t_min = (2.0 * t_coef - t_bias).max_element().max(0.0);

        let mut t_max = (t_coef - t_bias).min_element();

        let mut h: f32 = t_max;

        let mut idx: u32 = 0;

        let mut pos: Vec3A = Vec3A::splat(1.0);
        let value = 1.5 * t_coef - t_bias;
        let b_vec = value.cmpgt(Vec3A::splat(t_min));
        let bitmask = b_vec.bitmask();
        idx ^= bitmask;
        (0..3).for_each(|i: usize| {
            if b_vec.test(i) {
                pos[i] = 1.5;
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

            if !child.is_none() && t_min <= t_max {
                if child.is_leaf() && t_min > 0.0 {
                    //println!("hit");
                    //println!("pos: {:?},t_min:{}, current_parent: {}, unmirrored_idx: {}, scale: {}, is_child:{}, is_leaf: {}",ray.origin+ray.direction*(t_min/octree_scale),t_min/octree_scale,parent_octant_idx,unmirrored_idx,scale,is_child,is_leaf);
                    let leaf_value = child.get_leaf_value().unwrap();
                    let model = *leaf_value;

                    let mut unmirrored_pos = pos;
                    (0..3).for_each(|i: usize| {
                        if mirror_mask & 1 << i != 0 {
                            unmirrored_pos[i] = 3.0 - scale_exp2 - unmirrored_pos[i]
                        }
                    });
                    let t_corner = (pos + scale_exp2) * t_coef - t_bias;
                    let tc_min = t_corner.max_element();

                    let face_id;
                    let mut uv;
                    let b_vec = t_corner.cmpeq(Vec3A::splat(tc_min));
                    let b_vec_rd = rd.cmplt(Vec3A::ZERO);
                    if b_vec.test(0) {
                        face_id = 1 << 0 | (rd[0].to_bits() >> 31) & 1;
                        uv = Vec2::new(
                            (ro[2] + rd[2] * t_corner[0]) - unmirrored_pos[2],
                            (ro[1] + rd[1] * t_corner[0]) - unmirrored_pos[1],
                        ) / scale_exp2;
                        if b_vec_rd.test(0) {
                            uv[0] = 1.0 - uv[0];
                        }
                    } else if b_vec.test(1) {
                        face_id = 1 << 1 | ((rd[1].to_bits() >> 31) & 1);
                        uv = Vec2::new(
                            (ro[0] + rd[0] * t_corner[1]) - unmirrored_pos[0],
                            (ro[2] + rd[2] * t_corner[1]) - unmirrored_pos[2],
                        ) / scale_exp2;
                        if b_vec_rd.test(1) {
                            uv[1] = 1.0 - uv[1];
                        }
                    } else {
                        face_id = 1 << 2 | ((rd[2].to_bits() >> 31) & 1);
                        uv = Vec2::new(
                            (ro[0] + rd[0] * t_corner[2]) - unmirrored_pos[0],
                            (ro[1] + rd[1] * t_corner[2]) - unmirrored_pos[1],
                        ) / scale_exp2;
                        if b_vec_rd.test(2) {
                            uv[0] = 1.0 - uv[0];
                        }
                    }
                    ray.hit.u = uv.x;
                    ray.hit.v = uv.y;
                    let index = model.get_first_index();
                    let texture = &materials[quads[index as usize].material_id as usize].texture;
                    ray.hit.current_material = quads[index as usize].material_id;
                    return Cuboid::intersect_texture_not_transparent(ray, texture);
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
                        let b_vec = t_center.cmpgt(Vec3A::splat(t_min));
                        idx ^= b_vec.bitmask();
                        (0..3).for_each(|i: usize| {
                            if b_vec.test(i) {
                                pos[i] += scale_exp2;
                            }
                        });

                        t_max = tv_max;
                        continue;
                    }
                }
            }
            //advance
            //println!("advance!");

            let mut step_mask = 0;

            let b_vec = t_corner.cmple(Vec3A::splat(tc_max));
            step_mask ^= b_vec.bitmask();
            (0..3).for_each(|i: usize| {
                if b_vec.test(i) {
                    pos[i] -= scale_exp2;
                }
            });

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

                scale = util::find_msb_u32(differing_bits);
                scale_exp2 = f32::exp2((scale as i32 - OCTREE_MAX_SCALE as i32) as f32);

                if scale >= OCTREE_MAX_SCALE as u32 {
                    return false;
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
        return false;
    }
}
