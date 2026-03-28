use glam::Vec3A;

use crate::{
    find_msb_u32,
    geometry::axis::Axis,
    octree::world::{ChildType, OctantId, WorldOctree},
    path_tracing::ray::Ray,
};

const MAX_TRAVERSAL_STEPS: usize = 1000;
const MAX_SCALE_INDEX: usize = 23;
const TRAVERSAL_STACK_SIZE: usize = MAX_SCALE_INDEX + 1;
const TRAVERSAL_START_SCALE_INDEX: u32 = MAX_SCALE_INDEX as u32 - 1;
const SPECIAL_TRAVERSAL_EPSILON: f32 = 1.1920929e-7;
const MAX_DISTANCE: f32 = 1000.0;
const SIGN_MASK: u32 = 1 << 31;

pub struct TraversalContext {
    current_octant_index: OctantId,
    scale_index: u32,
    index_stack: [u32; TRAVERSAL_STACK_SIZE],
    time_stack: [f32; TRAVERSAL_STACK_SIZE],
}

pub struct OctreeIntersection {
    pub data: u32,
    pub position: Vec3A,
}

impl std::fmt::Debug for OctreeIntersection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OctreeIntersection")
            .field("data", &self.data)
            .field(
                "position",
                &format!(
                    "x: {}, y: {}, z: {}",
                    self.position.x, self.position.y, self.position.z
                ),
            )
            .finish()
    }
}

pub fn intersect_world_svo_simple(octree: &WorldOctree, ray: &Ray) -> Option<OctreeIntersection> {
    let tree_root = octree.root().expect("Octree should have a root");

    let inverse_octree_scale = octree.inverse_scale();

    let mut traversal_stack: [(OctantId, f32); TRAVERSAL_STACK_SIZE] =
        [Default::default(); TRAVERSAL_STACK_SIZE];

    // This traversal algorithm operates with coordinates in the range [1-2), so we scale and
    // transform our ray's origin by the tree's scale factor into the range [0,1) and add 1.0

    let ray_origin = (ray.origin * inverse_octree_scale) + 1.0;
    //    println!("Ray origin:{ray_origin}");

    let mut ray_direction = *ray.get_direction();
    //    println!("Ray dir:{ray_direction}");

    let max_distance = MAX_DISTANCE * inverse_octree_scale;

    let mut current_octant_index = tree_root;

    let mut scale_index = TRAVERSAL_START_SCALE_INDEX;
    // 2^(scale-MAX_OCTREE_SCALE) or 2^-1
    let mut scale_exp2 = 0.5f32;

    let ray_direction_absolute_value = ray_direction.abs();

    for axis in Axis::iter() {
        let axis: usize = axis.into();
        if ray_direction_absolute_value[axis] < SPECIAL_TRAVERSAL_EPSILON {
            let signed_epsilon =
                SPECIAL_TRAVERSAL_EPSILON.to_bits() | ray_direction[axis].to_bits() & SIGN_MASK;

            ray_direction[axis] = f32::from_bits(signed_epsilon);
        }
    }
    //    println!("Ray dir after mutation:{ray_direction}");

    let t_coefficient = 1.0 / -ray_direction_absolute_value;

    let mut t_bias = t_coefficient * ray_origin;

    let mut ray_direction_mirror_mask = 0;

    // Mirror ray directions to all be negative, keep track of which directions we mirrored
    for axis in Axis::iter() {
        let axis: usize = axis.into();
        if ray_direction[axis] > 0.0 {
            t_bias[axis] = 3.0 * t_coefficient[axis] - t_bias[axis];
            ray_direction_mirror_mask |= 1 << axis;
        }
    }

    //    println!("t_bias: {t_bias}");
    //    println!("mirror_mask: {ray_direction_mirror_mask:#010b}");

    // calculate t_min and only allow >=0.0
    let mut t_min = (2.0 * t_coefficient - t_bias).max_element().max(0.0);

    let mut t_max = (t_coefficient - t_bias).min_element();

    let mut h = t_max;

    let mut current_child_index = 0;

    let mut current_pos = Vec3A::splat(1.0);

    //calculate where the ray "exits" the current octant (the middle of the world)
    let upper = 1.5 * t_coefficient - t_bias;

    for axis in Axis::iter() {
        let axis: usize = axis.into();
        if upper[axis] > t_min {
            current_pos[axis] = 1.5;
            current_child_index |= 1 << axis;
        }
    }

    //    println!("current_pos: {current_pos}");
    //    println!("current_child_index: {current_child_index}");

    for _ in 0..MAX_TRAVERSAL_STEPS {
        if max_distance >= 0.0 && t_min > max_distance {
            return None;
        }

        let t_corner = current_pos * t_coefficient - t_bias;

        let t_corner_max = t_corner.min_element();

        let unmirrored_child_index = current_child_index ^ ray_direction_mirror_mask;

        let (child_type, data) =
            octree.octants_slice()[current_octant_index as usize].get_child(unmirrored_child_index);
        //        println!("Child Type: {child_type:?}, data: {data}");
        //        println!("current_octant_index: {current_octant_index}");
        //        println!("unmirrored_child_index: {unmirrored_child_index}");
        match child_type {
            ChildType::Empty => {}
            ChildType::Leaf => {
                if t_min >= 0.0 {
                    let mut unmirrored_pos = current_pos;
                    for axis in Axis::iter() {
                        let axis: usize = axis.into();
                        if ray_direction_mirror_mask & (1 << axis) != 0 {
                            unmirrored_pos[axis] = 3.0 - scale_exp2 - unmirrored_pos[axis];
                        }
                    }
                    return Some(OctreeIntersection {
                        data,
                        position: (unmirrored_pos - 1.0) / inverse_octree_scale,
                    });
                }
            }
            ChildType::Octant => {
                //Descend
                //                println!("Descend");
                let half_scale = scale_exp2 * 0.5;

                let t_center = half_scale * t_coefficient + t_corner;

                let tv_max = t_max.min(t_corner_max);

                if t_min <= tv_max {
                    let octant_index = data;
                    if t_corner_max < h {
                        traversal_stack[scale_index as usize] = (current_octant_index, t_max);
                    }

                    h = t_corner_max;
                    current_octant_index = octant_index;
                    scale_index -= 1;
                    scale_exp2 = half_scale;
                    current_child_index = 0;

                    for axis in Axis::iter() {
                        let axis: usize = axis.into();
                        if t_center[axis] > t_min {
                            current_pos[axis] += scale_exp2;
                            current_child_index |= 1 << axis;
                        }
                    }
                    t_max = tv_max;
                    continue;
                }
            }
        }
        //Advance Ray
        let mut step_mask = 0;

        for axis in Axis::iter() {
            let axis: usize = axis.into();
            if t_corner[axis] <= t_corner_max {
                current_pos[axis] -= scale_exp2;
                step_mask |= 1 << axis;
            }
        }

        t_min = t_corner_max;
        current_child_index ^= step_mask;

        if (current_child_index & step_mask) != 0 {
            //            println!("Pop!");
            // We need to step back up the tree
            let mut differing_bits = 0;
            for axis in Axis::iter() {
                let axis: usize = axis.into();
                if step_mask & (1 << axis) != 0 {
                    differing_bits |=
                        current_pos[axis].to_bits() ^ (current_pos[axis] + scale_exp2).to_bits();
                }
            }

            scale_index = find_msb_u32(differing_bits);
            scale_exp2 = f32::exp2((scale_index as i32 - MAX_SCALE_INDEX as i32) as f32);

            if scale_index >= MAX_SCALE_INDEX as u32 {
                return None;
            }

            (current_octant_index, t_max) = traversal_stack[scale_index as usize];

            current_child_index = 0;
            for axis in Axis::iter() {
                let axis: usize = axis.into();
                let sh = current_pos[axis].to_bits() >> scale_index;
                current_pos[axis] = f32::from_bits(sh << scale_index);
                current_child_index |= (sh as u8 & 1) << axis;
            }

            h = 0.0;
        }
    }
    None
}

#[cfg(test)]
pub mod tests {
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use hashbrown::HashMap;
    use mc_utils::{
        coords::region::RegionCoords, owned::nbt_string::NBTString, region::borrow::Region,
    };

    use crate::{octree::builders::region::build_region_octree, renderer::camera::Camera};

    use super::*;
    #[test]
    fn intersect_world() {
        let path = PathBuf::from("./assets/test_worlds/region/r.1.0.mca");

        let bytes = std::fs::read(&path).unwrap();

        let region = Region::from_bytes(&bytes, RegionCoords { x: 1, z: 0 });

        let blockstate_map = Arc::new(Mutex::new(HashMap::new()));

        let air = NBTString::new_from_str("minecraft:air#normal");
        blockstate_map.lock().unwrap().insert(air, 0);

        let (tree, map) = build_region_octree(region, blockstate_map).unwrap();
        let camera = Camera::look_at(
            Vec3A::new(0.0, 135.0, 16.0),
            Vec3A::new(12.0, 130.0, 12.0),
            Vec3A::Y,
            70.0f32.to_radians(),
        );
        let ray = camera.get_ray(0.0, 0.0);

        let intersect_result = intersect_world_svo_simple(&tree, &ray);

        dbg!(intersect_result);
    }
}
