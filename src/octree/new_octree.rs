use hashbrown::HashMap;
use std::cell::RefCell;
use std::hint::black_box;
use std::num::NonZeroU32;
use std::ops::RangeBounds;
use std::path::PathBuf;
use std::slice::SliceIndex;
use std::sync::Mutex;
use std::{fmt::Debug, num::NonZeroUsize, sync::Arc, time::Instant};

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelBridge,
    ParallelIterator,
};
use spider_eye::{
    borrow::nbt_compound::RootNBTCompound, chunk::borrow::Chunk, coords::block::BlockCoords,
    owned::nbt_string::NBTString, region::borrow::Region, section::borrow::Section,
};

#[derive(Default)]
//max depth of 21
pub struct Octree<T> {
    root: Option<OctantId>,
    octants: Vec<Octant<T>>,
    depth: u8,
}

impl<T: Default> Octree<T> {
    pub fn new_octant(&mut self) -> OctantId {
        let new_octant_id = self.octants.len();
        self.octants.push(Default::default());
        new_octant_id as OctantId
    }

    fn step_into_or_create_octant_at_morton(&mut self, morton_code: u64) -> OctantId {
        let mut current_octant = if let Some(root) = self.root {
            root
        } else {
            let new_root = self.new_octant();
            self.root = Some(new_root);
            new_root
        };

        //TODO this doesn't work

        //+1 because (21 bits per axis *3 axes) = 64
        let mut shift_amt = 1 + (63 - (3 * self.depth));
        loop {
            if shift_amt > 58 {
                break;
            }
            let child_idx = (morton_code << shift_amt) >> 61;
            println!("child idx: {child_idx}");
            match self.octants[current_octant as usize].children[child_idx as usize] {
                Child::None => {
                    let new_octant_id = self.new_octant();
                    self.octants[current_octant as usize].children[child_idx as usize] =
                        Child::Octant(new_octant_id);
                    current_octant = new_octant_id;
                }
                Child::Octant(id) => {
                    current_octant = id;
                }
                _ => {
                    panic!(
                        "Tried to place octant at {position:?} but it was not empty",
                        position = decode_morton(morton_code)
                    );
                }
            }
            shift_amt += 3;
        }
        current_octant
    }

    pub fn expand_to(&mut self, depth: u8) {
        if self.depth > depth {
            return;
        }
        let diff = depth - self.depth;

        if diff > 0 {
            self.expand_by(diff);
        }
    }

    pub fn expand_by(&mut self, depth: u8) {
        for _ in 0..depth {
            let new_root_id = self.new_octant();

            if let Some(root_id) = self.root {
                self.octants[new_root_id as usize].set_child(Child::Octant(root_id), 0);
            }
            self.root = Some(new_root_id)
        }
        self.depth += depth
    }
}

pub type OctantId = u32;

impl<T> Copy for Octant<T> where T: Copy {}

impl<T> Clone for Octant<T>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            child_count: self.child_count,
            children: self.children.clone(),
        }
    }
}

pub struct Octant<T> {
    child_count: u8,
    children: [Child<T>; 8],
}

impl<T> Octant<T> {
    #[inline]
    pub fn set_child(&mut self, mut new_child: Child<T>, index: u8) -> Child<T> {
        assert!(index < 8);
        if let Some(old_child) = self.children.get_mut(index as usize) {
            if old_child.is_none() && !new_child.is_none() {
                self.child_count += 1;
            } else if !old_child.is_none() && new_child.is_none() {
                self.child_count -= 1;
            }
            std::mem::swap(&mut new_child, old_child);
            //contains the old child now
            return new_child;
        }
        unreachable!()
    }
    pub fn child_count(&mut self) -> u8 {
        self.child_count
    }
}

impl<T: Default> Default for Octant<T> {
    fn default() -> Self {
        Self {
            children: Default::default(),
            child_count: 0,
        }
    }
}

impl<T: Debug> Debug for Octant<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Octant")
            .field("child_count", &self.child_count)
            .field("children", &self.children)
            .finish()
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum Child<T> {
    #[default]
    None,
    Lod(T),
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
            Self::Octant(arg0) => Self::Octant(*arg0),
            Self::Leaf(arg0) => Self::Leaf(arg0.clone()),
            Self::Lod(arg0) => Self::Lod(arg0.clone()),
        }
    }
}

impl<T> Child<T> {
    pub fn is_leaf(&self) -> bool {
        matches!(self, Child::Leaf(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Child::None)
    }
    pub fn is_octant(&self) -> bool {
        matches!(self, Child::Octant(_))
    }
}

pub struct LeafId {
    parent: OctantId,
    idx: u8,
}

fn calculate_loading_range(position: &BlockCoords, octree_depth: u8) {
    let world_size = 2_u32.pow(octree_depth as u32);

    let half_world_size = world_size / 2;

    let start_x = position.x - half_world_size as i64;
    let start_z = position.z - half_world_size as i64;
}

pub fn construct_all() {
    let octree_depth = 9;
    let path = PathBuf::from("./assets/worlds/test_world/r.1.0.mca");

    let region = Region::load_from_file(&path).expect("Could not load region");

    let blockstate_map = Arc::new(Mutex::new(HashMap::new()));

    let air = NBTString::new_from_str("minecraft:air#normal");
    blockstate_map.lock().unwrap().insert(air, 0);

    let start = Instant::now();
    let two = build_region_octree(region, blockstate_map);
    let end = Instant::now();

    println!("total time: {:?}", end.duration_since(start));
}

const LOWEST_SECTION_INDEX: i8 = -4;

const HIGHEST_SECTION_INDEX: i8 = 19;

pub fn build_region_octree(
    region: Region,
    blockstate_map: Arc<Mutex<HashMap<NBTString, u32>>>,
) -> Octree<usize> {
    //TODO maybe redo blockstate hash function
    let start = Instant::now();
    let region_chunk_data = region.load_all_chunk_data();
    let end = Instant::now();
    println!("time loading chunks: {:?}", end.duration_since(start));

    let start = Instant::now();
    let nbts: [Option<RootNBTCompound<'_>>; 1024] = region_chunk_data
        .iter()
        .map(|chunk_data| {
            let chunk_data = chunk_data.as_ref()?;
            RootNBTCompound::from_bytes(chunk_data.as_slice())
                .map_err(|err| println!("{err:?}"))
                .ok()
        })
        .collect::<Vec<Option<RootNBTCompound>>>()
        .try_into()
        .unwrap();

    let end = Instant::now();

    println!("time parsing nbt: {:?}", end.duration_since(start));

    let start = Instant::now();

    let chunks: [Option<Chunk<'_>>; 1024] = nbts
        .into_iter()
        .map(|nbt| Chunk::from_compound(nbt?))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let end = Instant::now();

    println!("time parsing chunks: {:?}", end.duration_since(start));

    let coords_and_sections = chunks
        .iter()
        .enumerate()
        .filter_map(|(i, chunk)| {
            let (chunk_local_x, chunk_local_z) = chunk_index_to_coordinates(i);
            //println!("x: {local_x} z: {local_z}");
            let chunk = chunk.as_ref()?;

            let sections = chunk.get_section_tower()?;

            Some(sections.iter_sections().filter_map(move |section| {
                let y_index = section.get_y_index();

                if !(LOWEST_SECTION_INDEX..HIGHEST_SECTION_INDEX + 1).contains(&y_index) {
                    //TODO allow non vanilla world heights
                    return None;
                }
                let y_pos = y_index + (-LOWEST_SECTION_INDEX);

                Some((
                    (chunk_local_x as u64, y_pos as u64, chunk_local_z as u64),
                    section,
                ))
            }))
        })
        .flatten()
        .collect::<Vec<_>>();

    let mut blockstate_map = blockstate_map.lock().unwrap();
    let start = Instant::now();
    let coords_and_sections = coords_and_sections
        .into_iter()
        .map(|((x, y, z), section)| {
            let palette = section.get_palette();
            let mapped_palette: Vec<u32> = palette
                .iter()
                .map(|blockstate| {
                    let mapped_state = blockstate.to_mapped_state();
                    let current_len = blockstate_map.len() as u32;
                    let value = blockstate_map
                        .entry(mapped_state)
                        .or_insert_with(|| current_len);
                    *value
                })
                .collect::<Vec<_>>();
            ((x, y, z, section), mapped_palette)
        })
        .collect::<Vec<_>>();

    drop(blockstate_map);

    let end = Instant::now();

    println!(
        "time remapping section palettes: {:?}",
        end.duration_since(start)
    );
    let start = Instant::now();

    let mut sections = coords_and_sections
        .into_iter()
        .map(|((x, y, z, section), palette)| {
            let morton_code = encode_morton(x, y, z);
            (morton_code, section_to_compacted_octree(&section, &palette))
        })
        .collect::<Vec<_>>();

    let end = Instant::now();
    println!("time to build octrees: {:?}", end.duration_since(start));
    sections.sort_unstable_by_key(|octree| octree.0);

    println!("number of sections: {count}", count = sections.len());

    let mut builder = RegionOctreeBuilder::new();
    let start = Instant::now();
    builder.build(sections);

    let end = Instant::now();

    println!("time to build region tree:{:?}", end.duration_since(start));

    Octree::default()
}

fn chunk_index_to_coordinates(i: usize) -> (u8, u8) {
    let i = i as u16;
    const BOTTOM_5_BITS: u16 = 0b11111;
    let chunk_local_x = i & BOTTOM_5_BITS;
    let chunk_local_z = i >> 5;
    (chunk_local_x as u8, chunk_local_z as u8)
}

pub const REGION_OCTREE_DEPTH: usize = 9;

#[derive(Debug, Default)]
struct RegionOctreeBuilder {
    octants: Vec<Octant<u32>>,
}

enum RegionSubtreeResult {
    Empty,
    Lod(u32),
    Octant(OctantId),
}
impl RegionOctreeBuilder {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn build(&mut self, mut morton_codes_and_sections: Vec<(u64, SectionOctantResult)>) {
        let tree_depth = REGION_OCTREE_DEPTH - SECTION_OCTREE_DEPTH; //we are using local
                                                                     //coordinates and a region
                                                                     //is 32x32 on the x and z
                                                                     //axes, so depth is 5

        let result =
            self.recursive_build(tree_depth as u8, morton_codes_and_sections.as_mut_slice());

        println!("octants final len: {}", self.octants.len());

        println!(
            "memory footprint: {}kb",
            (self.octants.len() * size_of::<Octant<usize>>()) / 1000
        );
    }

    fn recursive_build(
        &mut self,
        target_depth: u8,
        data: &mut [(u64, SectionOctantResult)],
    ) -> RegionSubtreeResult {
        let mut data_opt = Some(data);
        let new_depth = target_depth - 1;
        const BITS_PER_DEPTH: usize = 3;

        let prefix_shift_amount = new_depth * BITS_PER_DEPTH as u8;
        let prefix_base = (1 << prefix_shift_amount) - 1; //fills all the bits to the right of
                                                          //prefix_shift_amount with 1
        let mut child_count = 0;
        let children: [Child<u32>; 8] = (0..8)
            .map(|child_index: u64| {
                let data = data_opt.take().unwrap();

                let prefix = (child_index << prefix_shift_amount) | prefix_base;

                if new_depth > 0 {
                    let slice_end_index = data.partition_point(|(value, _)| *value <= prefix);

                    let (subtree_slice, new_data) = data.split_at_mut(slice_end_index);
                    data_opt = Some(new_data);
                    if subtree_slice.is_empty() {
                        return Child::None;
                    }

                    let child = self.recursive_build(new_depth, subtree_slice);

                    match child {
                        RegionSubtreeResult::Empty => Child::None,
                        RegionSubtreeResult::Lod(data) => {
                            child_count += 1;
                            Child::Lod(data)
                        }
                        RegionSubtreeResult::Octant(octant) => {
                            child_count += 1;
                            Child::Octant(octant)
                        }
                    }
                } else {
                    assert!(data.len() <= 8);
                    let child = if let Some((_, section)) = data.get_mut(child_index as usize) {
                        match section {
                            SectionOctantResult::Subtree {
                                section_octants,
                                root,
                            } => {
                                child_count += 1;
                                let current_octants_len = self.octants.len() as u32;

                                let new_root = *root + current_octants_len;

                                section_octants.iter_mut().for_each(|octant| {
                                    octant.children.iter_mut().for_each(|child| {
                                        if let Child::Octant(val) = child {
                                            *val += current_octants_len;
                                        }
                                    });
                                });
                                self.octants.extend(section_octants.as_slice());
                                Child::Octant(new_root)
                            }
                            SectionOctantResult::Empty => Child::None,
                            SectionOctantResult::Lod(data) => {
                                child_count += 1;
                                Child::Lod(*data)
                            }
                        }
                    } else {
                        Child::None
                    };
                    data_opt = Some(data);
                    child
                }
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let first = &children[0];
        let result = if children.iter().all(|child| child == first) {
            match first {
                Child::None => RegionSubtreeResult::Empty,
                Child::Lod(data) => RegionSubtreeResult::Lod(*data),
                _ => unreachable!(),
            }
        } else {
            let octant_id = self.octants.len();
            self.octants.push(Octant {
                child_count,
                children,
            });
            RegionSubtreeResult::Octant(octant_id as u32)
        };

        result
    }
}

pub const SECTION_OCTREE_DEPTH: usize = 4;
pub const CHILD_COUNT: usize = 8;

#[derive(Debug, Default)]
struct ChildBuffer {
    initialized_count: u8,
    child_count: u8,
    uncompactable: bool,
    buffer: [Child<u32>; CHILD_COUNT],
}

impl ChildBuffer {
    pub fn clear(&mut self) {
        self.initialized_count = 0;
        self.child_count = 0;
        self.uncompactable = false;
    }
    pub fn insert_child(&mut self, child: &Child<u32>) -> bool {
        if self.initialized_count > 0 {
            if child != &self.buffer[0] {
                self.uncompactable = true;
            }
        }
        if self.initialized_count < 8 {
            let free_slot_index = self.initialized_count as usize;
            if !child.is_none() {
                self.child_count += 1;
            }
            self.buffer[free_slot_index] = *child;
            self.initialized_count += 1;
            true
        } else {
            false
        }
    }
    pub fn is_compactable(&self) -> bool {
        !self.uncompactable
    }
    pub fn child_count(&self) -> u8 {
        self.child_count
    }
    pub fn buffer(&self) -> &[Child<u32>; 8] {
        &self.buffer
    }
}

#[derive(Default, Debug)]
struct SectionOctantBuilder {
    octants: Vec<Octant<u32>>,
    child_buffers: [ChildBuffer; SECTION_OCTREE_DEPTH - 1],
}

#[derive(Debug, Default)]
enum SectionOctantResult {
    Subtree {
        section_octants: Vec<Octant<u32>>,
        root: OctantId,
    },
    #[default]
    Empty,
    Lod(u32),
}

impl SectionOctantBuilder {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn section_data_to_octants(
        mut self,
        morton_order_section_data: &[Option<NonZeroU32>; 4096],
    ) -> SectionOctantResult {
        let (chunks, remainder) = morton_order_section_data.as_chunks::<CHILD_COUNT>();
        assert!(remainder.is_empty());

        chunks.iter().for_each(|depth_1_octant| {
            let child = self.leaves_to_child(depth_1_octant);
            self.insert_child_and_compact(child);
        });

        let root_buffer = &self.child_buffers[0];
        if root_buffer.is_compactable() {
            match root_buffer.buffer()[0] {
                Child::None => SectionOctantResult::Empty,
                Child::Lod(data) => SectionOctantResult::Lod(data),
                _ => unreachable!(),
            }
        } else {
            let root_octant = Octant {
                child_count: root_buffer.child_count(),
                children: *root_buffer.buffer(),
            };
            let octant_id = self.octants.len();
            self.octants.push(root_octant);

            self.octants.iter_mut().for_each(|octant| {
                octant.children.iter_mut().for_each(|child| {
                    if let Child::Octant(id) = child {
                        let new_id = (octant_id as u32) - *id;
                        *id = new_id;
                    }
                });
            });

            self.octants.reverse();

            SectionOctantResult::Subtree {
                section_octants: self.octants,
                root: 0,
            }
        }
    }
    fn leaves_to_child(&mut self, data: &[Option<NonZeroU32>; 8]) -> Child<u32> {
        let first = &data[0];
        let mut uncompactable = false;
        let mut child_count = 0;
        data.iter().for_each(|item| {
            if item != first {
                uncompactable = true;
            }
            if item.is_some() {
                child_count += 1;
            }
        });

        let resulting_child = if uncompactable {
            let new_octant = Octant {
                child_count: child_count as u8,
                children: data
                    .iter()
                    .map(|opt| {
                        if let Some(leaf) = opt {
                            Child::Leaf(leaf.get())
                        } else {
                            Child::None
                        }
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            };
            let octant_id = self.octants.len() as u32;
            self.octants.push(new_octant);
            Child::Octant(octant_id)
        } else if let Some(leaf) = first {
            Child::Lod(leaf.get())
        } else {
            Child::None
        };

        resulting_child
    }

    fn insert_child_and_compact(&mut self, mut new_child: Child<u32>) {
        let mut search_depth = SECTION_OCTREE_DEPTH - 2;
        loop {
            let current_buffer = &mut self.child_buffers[search_depth];
            if current_buffer.insert_child(&new_child) {
                break;
            } else {
                let first_child = current_buffer.buffer[0];
                new_child = if current_buffer.is_compactable() {
                    first_child
                } else {
                    let octant_id = self.octants.len();
                    let new_octant = Octant {
                        child_count: current_buffer.child_count,
                        children: *current_buffer.buffer(),
                    };
                    self.octants.push(new_octant);
                    Child::Octant(octant_id as u32)
                };
                current_buffer.clear();

                search_depth -= 1;
            }
        }
    }
}

pub fn section_to_compacted_octree(
    section: &Section<'_, '_>,
    remapped_palette: &[u32],
) -> SectionOctantResult {
    if remapped_palette.len() < 2 {
        return if remapped_palette.len() == 0 {
            //this shouldn't happen, but we'll treat the section as full of air
            SectionOctantResult::Empty
        } else {
            let section_fill_block = remapped_palette.get(0).unwrap();
            if *section_fill_block == 0 {
                //UNWRAP: we've ensured the length is 1
                SectionOctantResult::Empty
            } else {
                SectionOctantResult::Lod(*section_fill_block)
            }
        };
    }
    let mut morton_order_data: [Option<NonZeroU32>; 4096] = [Option::None; 4096];

    for (i, palette_index) in section.iter_block_indices().enumerate() {
        let (x, y, z) = section_index_to_block_coordinates(i as u16);
        let morton_code = encode_morton(x as u64, y as u64, z as u64);

        let value = remapped_palette
            .get(palette_index as usize)
            //remap palettes
            .expect("index should be in range of palette");

        morton_order_data[morton_code as usize] = NonZeroU32::new(*value);
    }

    let builder = SectionOctantBuilder::new();

    builder.section_data_to_octants(&morton_order_data)
}

#[inline]
fn encode_morton(x: u64, y: u64, z: u64) -> u64 {
    (part_by_2(z) << 2) + (part_by_2(y) << 1) + part_by_2(x)
}

#[inline]
fn decode_morton(val: u64) -> (u64, u64, u64) {
    (
        compact_by_2(val),
        (compact_by_2(val >> 1)),
        (compact_by_2(val >> 2)),
    )
}

#[inline]
fn part_by_2(val: u64) -> u64 {
    let mut x = val & 0x1fffff; // we only look at the first 21 bits
    x = (x | x << 32) & 0x1f00000000ffff; // shift left 32 bits, OR with self, and 00011111000000000000000000000000000000001111111111111111
    x = (x | x << 16) & 0x1f0000ff0000ff; // shift left 32 bits, OR with self, and 00011111000000000000000011111111000000000000000011111111
    x = (x | x << 8) & 0x100f00f00f00f00f; // shift left 32 bits, OR with self, and 0001000000001111000000001111000000001111000000001111000000000000
    x = (x | x << 4) & 0x10c30c30c30c30c3; // shift left 32 bits, OR with self, and 0001000011000011000011000011000011000011000011000011000100000000
    x = (x | x << 2) & 0x1249249249249249;
    x
}

#[inline]
fn compact_by_2(val: u64) -> u64 {
    let mut x = val & 0x1249249249249249;
    x = (x | x >> 2) & 0x10c30c30c30c30c3;
    x = (x | x >> 4) & 0x100f00f00f00f00f;
    x = (x | x >> 8) & 0x1f0000ff0000ff;
    x = (x | x >> 16) & 0x1f00000000ffff;
    x = (x | x >> 32) & 0x1fffff;
    x
}

#[inline]
fn section_index_to_block_coordinates(index: u16) -> (u16, u16, u16) {
    assert!(index < 4096);
    const X_BITS: u16 = 0xF;
    const Y_BITS: u16 = 0xF00;
    const Z_BITS: u16 = 0x0F0;
    const BITS_PER_COORD: u16 = 4;
    let (x, y, z) = (
        index & X_BITS,
        (index & Y_BITS) >> (BITS_PER_COORD * 2),
        (index & Z_BITS) >> BITS_PER_COORD,
    );
    (x, y, z)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn section_test() {
        construct_all();
    }
    #[test]
    pub fn morton_code_bit_pattern() {
        let coord = (1, 0, 1);

        let code = encode_morton(coord.0, coord.1, coord.2);

        let decoded_coords = decode_morton(code);

        assert_eq!(coord, decoded_coords);
    }
}
