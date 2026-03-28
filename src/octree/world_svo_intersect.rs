use crate::{
    geometry::axis::Axis,
    octree::world::{OctantId, WorldOctree},
    path_tracing::{ray::Ray, vector_extensions::VectorExtensions as _},
};

const MAX_TRAVERSAL_STEPS: usize = 1000;
const MAX_OCTREE_SCALE: usize = 23;
const TRANVERSAL_STACK_SIZE: usize = MAX_OCTREE_SCALE + 1;
const TRAVERSAL_STARTING_SCALE: u32 = MAX_OCTREE_SCALE as u32 - 1;
const SPECIAL_TRAVERSAL_EPSILON: f32 = 1.1920929e-7;
const MAX_DISTANCE: f32 = 1000.0;
const SIGN_MASK: u32 = 1 << 31;

pub fn intersect_world_svo(octree: &WorldOctree, ray: &Ray) -> Option<u32> {
    let tree_root = octree.root().expect("Octree should have a root");

    let inverse_scale = octree.inverse_scale();

    let mut traversal_stack: [(OctantId, f32); TRANVERSAL_STACK_SIZE] =
        [Default::default(); TRANVERSAL_STACK_SIZE];

    // This traversal algorithm operates with coordinates in the range [1-2), so we scale and
    // transform our ray's origin by the tree's scale factor into the range [0,1) and add 1.0

    let ray_origin = (ray.origin * inverse_scale) + 1.0;

    let mut ray_direction = *ray.get_direction();

    let max_distance = MAX_DISTANCE * inverse_scale;

    let mut parent_octant_index = tree_root;

    let mut scale = TRAVERSAL_STARTING_SCALE;
    // 2^(scale-MAX_OCTREE_SCALE) or 2^-1
    let mut scale_exp2 = 0.5f32;

    let ray_direction_absolute_value = ray_direction.abs();

    for axis in Axis::iter() {
        let axis_index: usize = axis.into();
        if ray_direction_absolute_value[axis_index] < SPECIAL_TRAVERSAL_EPSILON {
            let signed_epsilon = SPECIAL_TRAVERSAL_EPSILON.to_bits()
                | (ray_direction[axis_index].to_bits() | SIGN_MASK);
            ray_direction[axis_index] = f32::from_bits(signed_epsilon);
        }
    }

    let t_coefficient = 1.0 / -ray_direction_absolute_value;

    let mut t_bias = t_coefficient * ray_origin;

    todo!();
}

#[test]
fn intersect_world() {
    println!("{SIGN_MASK:#032b}");
    println!("{:#032b}", SPECIAL_TRAVERSAL_EPSILON.to_bits());
    println!("{:#032b}", SPECIAL_TRAVERSAL_EPSILON.to_bits() & !(1 << 31));
}
