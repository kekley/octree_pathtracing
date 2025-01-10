use std::{fmt::Debug, hash::Hash, mem};

use glam::UVec3;
use rand_distr::num_traits::{Pow};


pub type OctantId = u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct LeafId {
    pub parent: OctantId,
    pub idx: u8,
}

pub trait Position: Copy + Clone + Debug + Sized {
    fn construct(x: u32, y: u32, z: u32) -> Self;
    fn idx(&self) -> u8;
    fn required_depth(&self) -> u8;
    fn x(&self) -> u32;
    fn y(&self) -> u32;
    fn z(&self) -> u32;
    fn div(&self, rhs: u32) -> Self;
    fn rem_assign(&mut self, rhs: u32);
}

impl Position for UVec3 {
    fn idx(&self) -> u8 {
        let val = (self.x + self.y * 2 + self.z * 4) as u8;
        val
    }
    fn required_depth(&self) -> u8 {
        let depth = self.max_element();
        (depth as f32).log2().floor() as u8 + 1
    }

    fn construct(x: u32, y: u32, z: u32) -> Self {
        Self::new(x, y, z)
    }

    fn x(&self) -> u32 {
        self.x
    }

    fn y(&self) -> u32 {
        self.y
    }

    fn z(&self) -> u32 {
        self.z
    }

    fn div(&self, rhs: u32) -> Self {
        *self / rhs
    }

    fn rem_assign(&mut self, rhs: u32) {
        *self %= rhs;
    }
}

#[derive(Debug, Default)]
pub enum Child<T> {
    #[default]
    None,
    Octant(OctantId),
    Leaf(T),
}

impl<T> Child<T> {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_octant(&self) -> bool {
        matches!(self, Self::Octant(_))
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    pub fn get_leaf_value(&self) -> Option<&T> {
        match self {
            Self::Leaf(val) => Some(val),
            _ => None,
        }
    }
    pub fn get_octant_value(&self) -> Option<OctantId> {
        match self {
            Self::Octant(id) => Some(*id),
            _ => None,
        }
    }

    pub fn get_leaf_value_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Leaf(val) => Some(val),
            _ => None,
        }
    }

    pub fn into_leaf_value(self) -> Option<T> {
        match self {
            Self::Leaf(val) => Some(val),
            _ => None,
        }
    }
}

impl<T: PartialEq> PartialEq for Child<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Child::None, Child::None) => true,
            (Child::Octant(l), Child::Octant(r)) => l == r,
            (Child::Leaf(l), Child::Leaf(r)) => l == r,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Octant<T> {
    pub parent: Option<OctantId>,
    pub child_count: u8,
    pub children: [Child<T>; 8],
}

impl<T> Octant<T> {
    fn set_child(&mut self, idx: u8, child: Child<T>) -> Child<T> {
        let idx = idx as usize;
        if self.children[idx].is_none() && !child.is_none() {
            self.child_count += 1;
        }
        if !self.children[idx].is_none() && child.is_none() {
            self.child_count -= 1;
        }
        let mut child = child;

        mem::swap(&mut child, &mut self.children[idx]);
        child
    }
}

#[derive(Debug)]
pub struct Octree<T> {
    pub(super) root: Option<OctantId>,
    pub octants: Vec<Octant<T>>,
    free_list: Vec<OctantId>,
    depth: u8,
}

impl<T> Octree<T> {
    pub fn new() -> Self {
        Self::new_in()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity)
    }
}

impl<T: PartialEq> PartialEq for Octree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root.eq(&other.root)
            && self.octants.eq(&other.octants)
            && self.free_list.eq(&other.free_list)
            && self.depth.eq(&other.depth)
    }
}

impl<T> Octree<T> {
    fn new_in() -> Self {
        Self::with_capacity_in(0)
    }

    fn with_capacity_in(capacity: usize) -> Self {
        Self {
            root: None,
            octants: Vec::with_capacity(capacity),
            free_list: Vec::new(),
            depth: 0,
        }
    }

    pub fn set_leaf(&mut self, pos: impl Position, leaf: T) -> (LeafId, Option<T>) {
        self.expand(pos.required_depth());

        let mut it = self.root.unwrap();
        let mut pos = pos;
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;

            let idx = pos.div(size).idx();
            pos.rem_assign(size);

            if size == 1 {
                let prev = self.octants[it as usize].set_child(idx, Child::Leaf(leaf));
                return (LeafId { parent: it, idx }, prev.into_leaf_value());
            }
            it = self.step_into_or_create_octant_at(it, idx);
        }
        unreachable!()
    }

    pub fn get_leaf(&self, pos: impl Position) -> Option<&T> {
        let mut it = self.root.unwrap();
        let mut pos = pos;
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size > 0 {
            size /= 2;
            let idx = pos.div(size).idx();
            //println!("it: {}, pos: {:?} idx: {},size: {}", it, pos, idx, size);
            pos.rem_assign(size);

            let child = &self.octants[it as usize].children[idx as usize];
            if child.is_none() {
                //println!("is none! size:{}", size);
                break;
            }

            match &self.octants[it as usize].children[idx as usize] {
                Child::None => break,
                Child::Octant(id) => it = *id,
                Child::Leaf(val) => return Some(val),
            }
        }

        None
    }

    pub fn construct_octants_with<P: Position, F: Fn(P) -> Option<T>>(&mut self, depth: u8, f: F) {
        self.reset();

        let size = 2f32.pow(depth as i32) as u32;

        if let Some(result) = self.construct_octants_with_impl(size, P::construct(0, 0, 0), &f) {
            self.root = Some(result);
            self.depth = depth;
        }
    }
    fn construct_octants_with_impl<P: Position, F: Fn(P) -> Option<T>>(
        &mut self,
        size: u32,
        pos: P,
        f: &F,
    ) -> Option<OctantId> {
        let size = size / 2;

        let mut new_parent = None;

        for i in 0u8..8 {
            let child_pos = P::construct(
                pos.x() + size * ((i as u32) & 1),
                pos.y() + size * ((i as u32 >> 1) & 1),
                pos.z() + size * ((i as u32 >> 2) & 1),
            );

            if size > 1 {
                let child_id = self.construct_octants_with_impl(size, child_pos, f);
                let Some(child_id) = child_id else {
                    continue;
                };

                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Octant(child_id));

                let child = &mut self.octants[child_id as usize];
                child.parent = Some(*parent_id);

                continue;
            }

            if let Some(value) = f(child_pos) {
                let parent_id = new_parent.get_or_insert_with(|| self.new_octant(None));
                self.octants[*parent_id as usize].set_child(i, Child::Leaf(value));
            }
        }

        new_parent
    }

    pub fn move_leaf(&mut self, leaf_id: LeafId, to_pos: impl Position) -> (LeafId, Option<T>) {
        self.expand_to(to_pos.required_depth());

        let mut it = self.root.unwrap();
        let mut pos = to_pos;
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;
            let idx = pos.div(size).idx();
            pos.rem_assign(size);

            if size == 1 {
                //leaf replaced with itself
                if it == leaf_id.parent && idx == leaf_id.idx {
                    return (leaf_id, None);
                }

                let old_leaf = self.octants[it as usize].set_child(leaf_id.idx, Child::None);

                let new_leaf =
                    self.octants[leaf_id.parent as usize].set_child(leaf_id.idx, Child::None);

                if new_leaf.get_leaf_value().is_some() {
                    self.octants[it as usize].set_child(idx, new_leaf);
                }

                let new_leaf_id = LeafId { parent: it, idx };

                match old_leaf {
                    Child::None => return (new_leaf_id, None),
                    Child::Octant(_) => unreachable!("found unexpected octant"),
                    Child::Leaf(val) => return (new_leaf_id, Some(val)),
                }
            }
            it = self.step_into_or_create_octant_at(it, idx);
        }
        unreachable!("could not reach end of tree")
    }

    pub fn remove_leaf(&mut self, pos: impl Position) -> (Option<T>, Option<LeafId>) {
        if pos.required_depth() > self.depth {
            return (None, None);
        }

        let mut it = self.root.unwrap();
        let mut pos = pos;
        let mut size = 2f32.pow(self.depth as i32) as u32;

        while size >= 1 {
            size /= 2;
            let idx = pos.div(size).idx();
            pos.rem_assign(size);

            match &self.octants[it as usize].children[idx as usize] {
                Child::None => break,
                Child::Octant(id) => it = *id,
                Child::Leaf(_) => match self.octants[it as usize].set_child(idx, Child::None) {
                    Child::None => return (None, None),
                    Child::Octant(_) => unreachable!(),
                    Child::Leaf(val) => {
                        return (
                            Some(val),
                            Some(LeafId {
                                parent: it,
                                idx: idx,
                            }),
                        )
                    }
                },
            }
        }
        (None, None)
    }

    pub fn remove_leaf_by_id(&mut self, leaf_id: LeafId) -> Option<T> {
        match self.octants[leaf_id.parent as usize].children[leaf_id.idx as usize] {
            Child::None | Child::Octant(_) => None,
            Child::Leaf(_) => {
                match self.octants[leaf_id.parent as usize].set_child(leaf_id.idx, Child::None) {
                    Child::None => None,
                    Child::Octant(_) => unreachable!("found unexpected octant"),
                    Child::Leaf(val) => Some(val),
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.root = None;
        self.octants.clear();
        self.free_list.clear();
        self.depth = 0;
    }

    pub fn expand_to(&mut self, to: u8) {
        if self.depth > to {
            return;
        }
        let diff = to - self.depth;
        if diff > 0 {
            todo!()
        }
    }

    pub fn expand(&mut self, by: u8) {
        for _ in 0..by {
            let new_root_id = self.new_octant(None);

            if let Some(root_id) = self.root {
                self.octants[root_id as usize].parent = Some(new_root_id);
                self.octants[new_root_id as usize].set_child(0, Child::Octant(root_id));
            }
            self.root = Some(new_root_id)
        }
        self.depth += by
    }

    fn new_octant(&mut self, parent: Option<OctantId>) -> OctantId {
        if let Some(free_id) = self.free_list.pop() {
            self.octants[free_id as usize].parent = parent;
            return free_id;
        }
        let id = self.octants.len() as OctantId;
        self.octants.push(Octant {
            parent,
            child_count: 0,
            children: Default::default(),
        });
        id
    }

    pub fn compact(&mut self) {
        if self.root.is_none() {
            return;
        }
        self.compact_octant(self.root.unwrap());

        if self.octants[self.root.unwrap() as usize].child_count != 0 {
            return;
        }

        self.reset();
    }

    fn compact_octant(&mut self, octant_id: OctantId) {
        let children = self.octants[octant_id as usize].children.len();

        for i in 0..children {
            let id = {
                let octant = &self.octants[octant_id as usize];
                octant.children[i].get_octant_value()
            };
            if id.is_none() {
                continue;
            }
            let id = id.unwrap();

            self.compact_octant(id);

            let octant = &self.octants[id as usize];
            if octant.child_count == 0 {
                self.delete_octant(id);
                self.octants[octant_id as usize].set_child(i as u8, Child::None);
            }
        }
    }

    fn delete_octant(&mut self, id: OctantId) {
        if let Some(parent) = self.octants[id as usize].parent {
            let children = &self.octants[parent as usize].children;
            let idx = children
                .iter()
                .position(|x| x.get_octant_value() == Some(id));

            if let Some(idx) = idx {
                self.octants[parent as usize].set_child(idx as u8, Child::None);
            }
        }

        let octant = &mut self.octants[id as usize];
        octant.parent = None;
        octant.child_count = 0;

        for child in &mut octant.children {
            *child = Child::None;
        }

        self.free_list.push(id);
    }

    fn step_into_or_create_octant_at(&mut self, it: OctantId, idx: u8) -> OctantId {
        match &self.octants[it as usize].children[idx as usize] {
            Child::None => {
                let prev_id = it;
                let next_id = self.new_octant(Some(prev_id));
                let prev_octant: &mut Octant<T> = &mut self.octants[prev_id as usize];
                prev_octant.set_child(idx, Child::Octant(next_id));

                next_id
            }
            Child::Octant(id) => *id,
            Child::Leaf(_) => unreachable!("found unexpected leaf"),
        }
    }

    pub fn depth(&self) -> u8 {
        self.depth
    }
}
