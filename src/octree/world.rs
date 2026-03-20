#![feature(int_lowest_highest_one)]
use crate::octree::region::build_region_octree;
use hashbrown::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Mutex;
use std::{fmt::Debug, sync::Arc, time::Instant};

use mc_utils::{coords::block::BlockCoords, owned::nbt_string::NBTString, region::borrow::Region};

#[derive(Default)]
//max depth of 21
pub struct Octree {
    root: Option<OctantId>,
    octants: Vec<Octant>,
    depth: u8,
}

impl Octree {
    fn new_octant(&mut self, parent: Option<OctantId>) -> OctantId {
        let new_octant_id = self.octants.len();
        self.octants.push(Default::default());
        new_octant_id as OctantId
    }

    pub fn root(&self) -> Option<OctantId> {
        self.root
    }

    pub fn octants_slice(&self) -> &[Octant] {
        &self.octants
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }

    pub fn scale(&self) -> f32 {
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
            let new_root_id = self.new_octant(None);

            if let Some(root_id) = self.root {
                self.octants[new_root_id as usize].set_child(ChildType::Octant, root_id, 0);
            }
            self.root = Some(new_root_id)
        }
        self.depth += depth
    }
}

pub type OctantId = u32;

#[derive(Default, Debug, Clone)]
pub struct Octant {
    pub(crate) child_mask: u16,
    pub(crate) children: [u32; 8],
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
                self.child_mask |= 1 << (index + 8);

                self.child_mask &= !(1 << index);
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

    pub fn init_children_with<F: FnMut(u8) -> (ChildType, u32)>(&mut self, mut f: F) {
        (0..8u8).for_each(|child_idx| {
            let (child_type, result_data) = f(child_idx);
            self.overwrite_child(child_type, result_data, child_idx);
        });
    }

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

    #[inline]
    pub fn overwrite_child(&mut self, child_type: ChildType, data: u32, index: u8) {
        self.set_mask_for(index, child_type);
        self.children[index as usize] = data;
    }

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

    #[inline]
    pub fn set_child(&mut self, child_type: ChildType, data: u32, index: u8) -> (ChildType, u32) {
        let ret_val = (self.get_type_of(index), self.children[index as usize]);
        self.set_mask_for(index, child_type);
        self.children[index as usize] = data;

        ret_val
    }

    #[inline]
    pub fn get_child(&self, index: u8) -> (ChildType, u32) {
        (self.get_type_of(index), self.children[index as usize])
    }
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

pub fn construct_all() {
    let path = PathBuf::from("./assets/worlds/test_world/r.1.0.mca");

    let bytes = std::fs::read(&path).unwrap();

    let region = Region::from_bytes(
        &bytes,
        mc_utils::coords::region::RegionCoords { x: 1, z: 0 },
    );

    let blockstate_map = Arc::new(Mutex::new(HashMap::new()));

    let air = NBTString::new_from_str("minecraft:air#normal");
    blockstate_map.lock().unwrap().insert(air, 0);

    let start = Instant::now();
    let _two = build_region_octree(region, blockstate_map);
    let end = Instant::now();

    println!("total time: {:?}", end.duration_since(start));
}
