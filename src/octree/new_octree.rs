use std::{fmt::Debug, num::NonZeroUsize, sync::Arc};

use hashbrown::HashMap;
use lasso::Spur;
use nonany::NonAnyU32;
use spider_eye::{
    blockstate::borrow::BlockState,
    borrow::{nbt_compound::RootNBTCompound, nbt_string::NBTStr},
    chunk::borrow::Chunk,
    owned::nbt_string::NBTString,
    region::borrow::Region,
    section::borrow::Section,
};

pub struct Octree<T> {
    scale: f32,
    root: Option<OctantId>,
    octants: Vec<Octant<T>>,
    depth: u8,
}

impl<T> Octree<T> {}

pub type OctantId = u32;

impl<T> Copy for Octant<T> where T: Copy {}

impl<T> Clone for Octant<T>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            child_count: self.child_count.clone(),
            parent: self.parent.clone(),
            children: self.children.clone(),
        }
    }
}

pub struct Octant<T> {
    pub parent: Option<OctantId>,
    pub child_count: u8,
    pub children: [Child<T>; 8],
}

impl<T> Octant<T> {
    pub fn set_child(&mut self, child: Child<T>, index: u8) {}
}

impl<T: Default> Default for Octant<T> {
    fn default() -> Self {
        Self {
            parent: Default::default(),
            children: Default::default(),
            child_count: 0,
        }
    }
}

impl<T: Debug> Debug for Octant<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Octant")
            .field("parent", &self.parent)
            .field("children", &self.children)
            .finish()
    }
}

#[derive(Debug, Default)]
pub enum Child<T> {
    #[default]
    None,
    Octant(OctantId),
    Leaf(T),
}

impl<T> Copy for Child<T> where T: Copy {}

impl<T> Clone for Child<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Octant(arg0) => Self::Octant(arg0.clone()),
            Self::Leaf(arg0) => Self::Leaf(arg0.clone()),
        }
    }
}

pub struct LeafId {
    parent: OctantId,
    idx: u8,
}
//Assume we're starting from 0,0 towards positive x and z
pub fn construct(target_depth: u8) {
    let mut octants: Vec<Octant<usize>> = Vec::new();
    let mut map: HashMap<Arc<[u8]>, usize> = HashMap::new();

    let region = Region::load_from_file("./world/r.0.0.mca").expect("Could not load region");
    let chunk = region.load_chunk_data(0, 0).unwrap();

    let compound = RootNBTCompound::from_bytes(&chunk).unwrap();

    let chunk = Chunk::from_compound(compound).unwrap();

    let sections = chunk.get_sections().unwrap();

    let mut octants = Vec::new();
    let mut blockstate_map: HashMap<NBTString, usize> = HashMap::new();

    let air = NBTString::new_from_str("minecraft:air#normal");

    blockstate_map.insert(air, 0);

    sections.iter_sections().for_each(|section| {
        section_to_octant(&section, &mut octants, &mut blockstate_map);
    });
}

pub fn section_to_octant(
    section: &Section<'_, '_>,
    octants: &mut Vec<Octant<usize>>,
    blockstate_map: &mut HashMap<NBTString, usize>,
) {
    pub const DEPTH: usize = 4;
    //TODO: load sections into an SVO, insert them into an existing tree with a separate worker
    let mut blockstate_count = blockstate_map.len();
    let palette = section.get_palette();

    let owned_palette: Vec<_> = palette
        .iter()
        .map(|blockstate| blockstate.to_mapped_state())
        .collect::<Vec<_>>();

    for nbt_string in owned_palette.as_slice() {
        println!("mapped state: {}", nbt_string.as_str().to_str());
    }

    let mut morton_order_data: [Option<NonZeroUsize>; 4096] = [Option::None; 4096];

    for (i, palette_index) in section.iter_block_indices().enumerate() {
        let (x, y, z) = index_to_coordinates(i as u64);
        let morton_code = calculate_morton_code(x, y, z);

        let blockstate = owned_palette
            .get(palette_index as usize)
            .expect("index should be in range of palette");

        let value = blockstate_map.entry(blockstate.clone()).or_insert_with(|| {
            let old = blockstate_count;
            blockstate_count += 1;
            old
        });

        morton_order_data[morton_code as usize] = NonZeroUsize::new(*value);
    }

    let mut octant_buffer: [Octant<usize>; DEPTH] = [Default::default(); DEPTH];

    //The lowest level of the tree
    let octant = octant_buffer
        .get_mut(DEPTH - 1)
        .expect("octant buffer should have DEPTH elements");

    let mut data_index = 0_usize;
    for child in &mut octant.children {
        let data = morton_order_data[data_index];
        *child = if let Some(data) = data {
            Child::Leaf(data.get())
        } else {
            Child::None
        };
        data_index += 1;
    }
}

fn calculate_morton_code(x: u64, y: u64, z: u64) -> u64 {
    (part_by_2(z) << 2) + (part_by_2(y) << 1) + part_by_2(x)
}

fn part_by_2(a: u64) -> u64 {
    let mut x = a & 0x1fffff; // we only look at the first 21 bits
    x = (x | x << 32) & 0x1f00000000ffff; // shift left 32 bits, OR with self, and 00011111000000000000000000000000000000001111111111111111
    x = (x | x << 16) & 0x1f0000ff0000ff; // shift left 32 bits, OR with self, and 00011111000000000000000011111111000000000000000011111111
    x = (x | x << 8) & 0x100f00f00f00f00f; // shift left 32 bits, OR with self, and 0001000000001111000000001111000000001111000000001111000000000000
    x = (x | x << 4) & 0x10c30c30c30c30c3; // shift left 32 bits, OR with self, and 0001000011000011000011000011000011000011000011000011000100000000
    x = (x | x << 2) & 0x1249249249249249;
    return x;
}

fn index_to_coordinates(index: u64) -> (u64, u64, u64) {
    const X_BITS: u64 = 0xF;
    const Y_BITS: u64 = 0xF00;
    const Z_BITS: u64 = 0x0F0;
    const BITS_PER_COORD: u64 = 4;
    let (x, y, z) = (
        index & X_BITS,
        (index & Y_BITS) >> (BITS_PER_COORD * 2),
        (index & Z_BITS) >> BITS_PER_COORD,
    );
    (x, y, z)
}

#[test]
pub fn section_test() {
    construct(4);
}
