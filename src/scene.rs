use crate::{
    axis::Axis,
    find_msb,
    octree_traversal::{OCTREE_EPSILON, OCTREE_MAX_SCALE, OCTREE_MAX_STEPS},
    path_trace, BVHTree, Camera, Cuboid, Face, Material, OctantId, Octree, Ray, Sphere,
};

use glam::{UVec3, Vec2, Vec3A, Vec4, Vec4Swizzles};
use rand::rngs::StdRng;
pub struct Scene {
    spheres: Vec<Sphere>,
    cubes: Vec<Cuboid>,
    bvhs: Vec<BVHTree>,
    pub octree: Octree<u32>,
    pub octree_palette: Vec<Cuboid>,
    pub materials: Vec<Material>,
    pub spp: u32,
    pub branch_count: u32,
    pub camera: Camera,
}

pub struct SceneBuilder {
    pub spp: Option<u32>,
    pub branch_count: Option<u32>,
    pub camera: Option<Camera>,
}

impl SceneBuilder {
    pub fn build(self) -> Scene {
        Scene {
            spheres: Vec::new(),
            cubes: Vec::new(),
            bvhs: Vec::new(),
            materials: Vec::new(),
            spp: self.spp.unwrap_or(1),
            branch_count: self.branch_count.unwrap_or(1),
            camera: self.camera.unwrap_or(Camera::default()),
            octree: Octree::new(),
            octree_palette: Vec::new(),
        }
    }

    pub fn spp(self, spp: u32) -> Self {
        Self {
            spp: Some(spp),
            ..self
        }
    }

    pub fn branch_count(self, branch_count: u32) -> Self {
        Self {
            branch_count: Some(branch_count),
            ..self
        }
    }

    pub fn camera(self, camera: Camera) -> Self {
        Self {
            camera: Some(camera),
            ..self
        }
    }
}

impl Scene {
    pub fn new() -> SceneBuilder {
        SceneBuilder {
            spp: None,
            branch_count: None,
            camera: None,
        }
    }
    pub const SKY_COLOR: Vec4 = Vec4::new(0.5, 0.7, 1.0, 1.0);

    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.spheres.push(sphere);
    }

    pub fn add_cube(&mut self, cube: Cuboid) {
        self.cubes.push(cube);
    }

    pub fn add_bvh(&mut self, bvh: BVHTree) {
        self.bvhs.push(bvh);
    }

    pub fn hit(&self, ray: &mut Ray) -> bool {
        let hit = false;
        self.octree
            .intersect_octree(ray, 1000.0, false, &self.cubes, &self.materials)
    }

    pub fn get_current_branch_count(&self) -> u32 {
        if self.spp < self.branch_count {
            if self.spp as f32 <= (self.branch_count as f32).sqrt() {
                return 1;
            } else {
                return self.branch_count - self.spp;
            }
        } else {
            return self.branch_count;
        }
    }

    pub fn get_pixel_color(&self, x: f32, y: f32, rng: &mut StdRng) -> glam::Vec3 {
        let mut ray = self.camera.get_ray(rng, x, y);
        path_trace(&self, &mut ray, true);
        ray.hit.color.xyz()
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
                    return true;
                }

                if is_leaf && t_min > 0.0 {
                    //println!("hit");
                    //println!("pos: {:?},t_min:{}, current_parent: {}, unmirrored_idx: {}, scale: {}, is_child:{}, is_leaf: {}",ray.origin+ray.direction*(t_min/octree_scale),t_min/octree_scale,parent_octant_idx,unmirrored_idx,scale,is_child,is_leaf);
                    let leaf_value = self.octants[parent_octant_idx as usize].children
                        [unmirrored_idx as usize]
                        .get_leaf_value()
                        .unwrap();
                    return leaf_value.intersect_octree(
                        ray,
                        max_dst,
                        do_translucency,
                        palette,
                        materials,
                    );
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
                scale = find_msb(differing_bits as i32);
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
