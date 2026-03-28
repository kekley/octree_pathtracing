#![feature(int_lowest_highest_one)]
use crate::octree::builders::region::{build_region_octree, encode_morton};
use hashbrown::HashMap;
use mc_utils::coords::region::RegionCoords;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Mutex;
use std::{fmt::Debug, sync::Arc, time::Instant};

use mc_utils::{owned::nbt_string::NBTString, region::borrow::Region};

pub trait OctreePosition {
    fn x(&self) -> u32;
    fn y(&self) -> u32;
    fn z(&self) -> u32;
    fn required_depth(&self) -> u8 {
        let max = self.x().max(self.y()).max(self.z());
        if max == 0 {
            1
        } else {
            (u32::BITS - max.leading_zeros()) as u8
        }
    }

    fn to_morton(&self) -> u64 {
        encode_morton(self.x().into(), self.y().into(), self.z().into())
    }
    fn from_xyz(x: u32, y: u32, z: u32) -> Self;
}

impl OctreePosition for (u32, u32, u32) {
    fn x(&self) -> u32 {
        self.0
    }

    fn y(&self) -> u32 {
        self.1
    }

    fn z(&self) -> u32 {
        self.2
    }

    fn from_xyz(x: u32, y: u32, z: u32) -> Self {
        (x, y, z)
    }
}

#[derive(Default, Debug)]
//max depth of 21
pub struct WorldOctree {
    root: Option<OctantId>,
    octants: Vec<Octant>,
    depth: u8,
}

impl WorldOctree {
    //Pushes an octant to the array and returns its index. Inserting into the octree is up to the
    //caller
    fn new_octant(&mut self) -> OctantId {
        let new_octant_id = self.octants.len();
        self.octants.push(Default::default());
        new_octant_id as OctantId
    }
    pub fn from_octants_root_depth(octants: Vec<Octant>, root: OctantId, depth: u8) -> Self {
        Self {
            root: Some(root),
            octants,
            depth,
        }
    }

    pub fn root(&self) -> Option<OctantId> {
        self.root
    }

    pub fn octants_slice(&self) -> &[Octant] {
        &self.octants
    }

    ///The depth of the octree
    pub fn depth(&self) -> u8 {
        self.depth
    }
    ///The scale of the octree, (2^-depth)
    pub fn inverse_scale(&self) -> f32 {
        f32::exp2(-(self.depth as f32))
    }

    fn expand_to(&mut self, depth: u8) {
        if self.depth > depth {
            return;
        }
        let diff = depth - self.depth;

        if diff > 0 {
            self.expand_by(diff);
        }
    }

    fn expand_by(&mut self, depth: u8) {
        for _ in 0..depth {
            let new_root_id = self.new_octant();

            if let Some(root_id) = self.root {
                self.octants[new_root_id as usize].set_child(ChildType::Octant, root_id, 0);
            }
            self.root = Some(new_root_id)
        }
        self.depth += depth
    }

    pub fn set_leaf(&mut self, position: impl OctreePosition, data: u32) -> (ChildType, u32) {
        self.expand_to(position.required_depth());
        let mut depth = self.depth;

        let morton = position.to_morton();

        let mut current_octant = self
            .root()
            .expect("Root should be initialized by self.expand_to()");
        while depth > 0 {
            let child_index: u8 = extract_index_for_depth_from_morton(morton, depth);

            if depth == 1 {
                let retval = self.octants[current_octant as usize].set_child(
                    ChildType::Leaf,
                    data,
                    child_index,
                );
                return retval;
            }

            current_octant = self.step_into_or_create_octant_at(current_octant, child_index);

            depth -= 1;
        }
        unreachable!()
    }

    pub fn get_leaf(&self, position: impl OctreePosition) -> Option<u32> {
        let mut current_octant = self.root().expect("No root");
        let mut depth = self.depth;

        let morton = position.to_morton();
        while depth > 0 {
            let child_index = extract_index_for_depth_from_morton(morton, depth);

            let (child_type, data) = self.octants[current_octant as usize].get_child(child_index);

            match child_type {
                ChildType::Empty => {
                    return None;
                }
                ChildType::Leaf => {
                    return Some(data);
                }
                ChildType::Octant => {
                    current_octant = data;
                }
            }
            depth -= 1;
        }

        unreachable!()
    }
    pub fn step_into_or_create_octant_at(&mut self, octant: OctantId, child_index: u8) -> OctantId {
        let (child_type, data) = self.octants[octant as usize].get_child(child_index);
        match child_type {
            ChildType::Empty => {
                let new_octant = self.new_octant();
                self.octants[octant as usize].overwrite_child(
                    ChildType::Octant,
                    new_octant,
                    child_index,
                );
                new_octant
            }
            ChildType::Leaf => {
                unreachable!("Unexpected Leaf");
            }
            ChildType::Octant => data,
        }
    }
}

pub type OctantId = u32;

#[derive(Default, Clone)]
pub struct Octant {
    pub(crate) child_mask: u16,
    pub(crate) children: [u32; 8],
}

impl Debug for Octant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Octant")
            .field("child_mask", &format!("{:#018b}", self.child_mask))
            .field("children", &self.children)
            .finish()
    }
}

pub struct OctantChildIterator<'a> {
    child_mask: u16,
    index: usize,
    children: &'a [u32; 8],
}

impl<'a> Iterator for OctantChildIterator<'a> {
    type Item = (ChildType, &'a u32);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index > 7 {
            return None;
        }
        let i = self.index;
        self.index += 1;

        if ((1 << i) & self.child_mask) != 0 {
            if ((1 << (i + 8)) & self.child_mask) != 0 {
                Some((ChildType::Leaf, &self.children[i]))
            } else {
                Some((ChildType::Octant, &self.children[i]))
            }
        } else {
            Some((ChildType::Empty, &self.children[i]))
        }
    }
}

pub struct OctantChildIteratorMut<'a> {
    child_mask: u16,
    index: usize,
    children: *mut u32,
    phantom_data: PhantomData<&'a mut u32>,
}

impl<'a> Iterator for OctantChildIteratorMut<'a> {
    type Item = (ChildType, &'a mut u32);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index > 7 {
            return None;
        }
        let i = self.index;
        self.index += 1;

        if ((1 << i) & self.child_mask) != 0 {
            if ((1 << (i + 8)) & self.child_mask) != 0 {
                return Some((ChildType::Leaf, unsafe {
                    self.children.add(i).as_mut().unwrap()
                }));
            } else {
                return Some((ChildType::Octant, unsafe {
                    self.children.add(i).as_mut().unwrap()
                }));
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildType {
    Empty,
    Leaf,
    Octant,
}

impl Octant {
    #[inline]
    pub fn is_child(&self, index: u8) -> bool {
        ((1 << index as usize) & self.child_mask) != 0
    }

    #[inline]
    pub fn is_octant(&self, index: u8) -> bool {
        self.is_child(index) && !self.is_leaf(index)
    }

    #[inline]
    pub fn is_leaf(&self, index: u8) -> bool {
        ((1 << (index + 8) as usize) & self.child_mask) != 0
    }

    #[inline]
    fn set_mask_for(&mut self, index: u8, child_type: ChildType) {
        match child_type {
            ChildType::Empty => {
                self.child_mask &= !(1 << index);
                self.child_mask &= !(1 << (index + 8));
            }
            ChildType::Leaf => {
                self.child_mask |= 1 << index;
                self.child_mask |= 1 << (index + 8);
            }
            ChildType::Octant => {
                self.child_mask |= 1 << index;
                self.child_mask &= !(1 << (index + 8));
            }
        }
    }

    pub fn child_count(&self) -> u8 {
        (self.child_mask & 0xFF).count_ones() as u8
    }

    pub fn iter_children(&self) -> impl Iterator<Item = (ChildType, &u32)> {
        OctantChildIterator {
            child_mask: self.child_mask,
            index: 0,
            children: &self.children,
        }
    }

    pub fn iter_children_mut(&mut self) -> impl Iterator<Item = (ChildType, &mut u32)> {
        OctantChildIteratorMut {
            child_mask: self.child_mask,
            index: 0,
            children: self.children.as_mut_ptr(),
            phantom_data: PhantomData,
        }
    }

    //set the values of this octant's children with a closure
    pub fn init_children_with<F: FnMut(u8) -> (ChildType, u32)>(&mut self, mut f: F) {
        (0..8u8).for_each(|child_idx| {
            let (child_type, result_data) = f(child_idx);
            self.overwrite_child(child_type, result_data, child_idx);
        });
    }

    ///Extracts the type of an octant from its bit fields
    #[inline]
    pub fn get_type_of(&self, index: u8) -> ChildType {
        if self.is_child(index) {
            if self.is_leaf(index) {
                ChildType::Leaf
            } else {
                ChildType::Octant
            }
        } else {
            ChildType::Empty
        }
    }
    ///Sets the type and data of the child at `index`, discarding the previous value
    #[inline]
    pub fn overwrite_child(&mut self, child_type: ChildType, data: u32, index: u8) {
        self.set_mask_for(index, child_type);
        self.children[index as usize] = data;
    }

    ///Returns true if the octant is composed only of leaves of the same type
    #[inline]
    pub fn is_compactable(&self) -> bool {
        let first = &self.children[1];
        //All leaf bits must be set
        (((self.child_mask >> 8) as u8) == u8::MAX && self.children.iter().all(|val| val == first))
            || self.child_mask == 0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.child_mask == 0
    }
    ///Sets the type and data of the child at `index`, returns the old data
    #[inline]
    pub fn set_child(&mut self, child_type: ChildType, data: u32, index: u8) -> (ChildType, u32) {
        let ret_val = (self.get_type_of(index), self.children[index as usize]);
        self.set_mask_for(index, child_type);
        self.children[index as usize] = data;

        ret_val
    }

    ///Gets the type and data of the child at `index`
    #[inline]
    pub fn get_child(&self, index: u8) -> (ChildType, u32) {
        (self.get_type_of(index), self.children[index as usize])
    }
    ///Resets the octant
    #[inline]
    pub(crate) fn clear(&mut self) {
        *self = Octant::default()
    }

    ///Gets the index of the first empty child slot
    #[inline]
    pub(crate) fn free_slot(&self) -> Option<u8> {
        if let Some(lowest_zero) = (!self.child_mask).lowest_one() {
            if lowest_zero < 8 {
                Some(lowest_zero as u8)
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct LeafId {
    parent: OctantId,
    idx: u8,
}
///Given a morton code and an octree depth, this function extracts the three bits that correspond
///to the index for an octant at that depth
fn extract_index_for_depth_from_morton(morton: u64, depth: u8) -> u8 {
    //        println!("Required Depth: {depth}");
    //    println!("morton: {morton:#066b}");
    let shift_amt = (depth - 1) * 3;
    let mask: u64 = 0b111 << shift_amt;
    //            println!("mask: {mask:#064b}");
    let child_index: u8 = ((morton & mask) >> shift_amt)
        .try_into()
        .expect("Child index should be between 0-7");

    //    println!("idx: {child_index:#010b}");
    child_index
}

#[test]
fn set_leaf() {
    let mut tree = WorldOctree::default();
    let pos_1 = (1, 1, 1);
    tree.set_leaf(pos_1, 69);
    println!("Set leaf 1");
    let pos_2 = (9, 1, 300);

    tree.set_leaf(pos_2, 420);
    println!("Set leaf 2");

    assert!(tree.get_leaf(pos_1).unwrap() == 69);
    println!("Get leaf 1");

    assert!(tree.get_leaf(pos_2).unwrap() == 420);
    println!("Get leaf 2");
}
