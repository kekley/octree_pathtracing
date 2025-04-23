use std::default::Default;

use crate::HittableIdx;

pub type OctantId = u32;

#[derive(Debug)]
pub struct Octant {}

#[derive(Debug)]
pub struct Octree {
    root: Option<OctantId>,
    octants: Vec<Octant>,
}

#[derive(Debug, Default)]
pub enum Child {
    #[default]
    None,
    Octant(OctantId),
    Leaf(HittableIdx),
}
