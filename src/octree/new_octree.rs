use hashbrown::HashMap;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Mutex;
use std::{fmt::Debug, sync::Arc, time::Instant};

use spider_eye::{
    borrow::nbt_compound::RootNBTCompound, chunk::borrow::Chunk, coords::block::BlockCoords,
    owned::nbt_string::NBTString, region::borrow::Region, section::borrow::Section,
};

#[derive(Default)]
//max depth of 21
pub struct Octree {
    root: Option<OctantId>,
    octants: Vec<Octant>,
    depth: u8,
}

impl Octree {
    pub fn new_octant(&mut self) -> OctantId {
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
    child_mask: u16,
    children: [u32; 8],
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
    fn clear(&mut self) {
        *self = Octant::default()
    }

    ///Gets the index of the first empty child slot
    #[inline]
    fn free_slot(&self) -> Option<u8> {
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

fn calculate_loading_range(position: &BlockCoords, octree_depth: u8) {
    let world_size = 2_u32.pow(octree_depth as u32);

    let half_world_size = world_size / 2;

    let _start_x = position.x - half_world_size as i64;
    let _start_z = position.z - half_world_size as i64;
    todo!()
}

pub fn construct_all() {
    let path = PathBuf::from("./assets/worlds/test_world/r.1.0.mca");

    let bytes = std::fs::read(&path).unwrap();

    let region = Region::from_bytes(
        &bytes,
        spider_eye::coords::region::RegionCoords { x: 1, z: 0 },
    );

    let blockstate_map = Arc::new(Mutex::new(HashMap::new()));

    let air = NBTString::new_from_str("minecraft:air#normal");
    blockstate_map.lock().unwrap().insert(air, 0);

    let start = Instant::now();
    let _two = build_region_octree(region, blockstate_map);
    let end = Instant::now();

    println!("total time: {:?}", end.duration_since(start));
}

const LOWEST_SECTION_INDEX: i8 = -4;

const HIGHEST_SECTION_INDEX: i8 = 19;

pub fn build_region_octree(
    region: Region,
    blockstate_map: Arc<Mutex<HashMap<NBTString, u32>>>,
) -> Option<Octree> {
    //TODO maybe redo blockstate hash function
    let start = Instant::now();
    let region_chunk_data = region.load_all_chunk_data();
    let end = Instant::now();
    println!("time loading chunks: {:?}", end.duration_since(start));

    let start = Instant::now();
    let mut nbts: [Option<RootNBTCompound<'_>>; 1024] = [const { None }; 1024];
    nbts.iter_mut()
        .zip(region_chunk_data.iter())
        .for_each(|(nbt, chunk_data)| {
            if let Some(chunk_data) = chunk_data {
                *nbt = RootNBTCompound::from_bytes(chunk_data)
                    .map_err(|err| println!("{err:?}"))
                    .ok()
            }
        });

    let end = Instant::now();

    println!("time parsing nbt: {:?}", end.duration_since(start));

    let start = Instant::now();

    let mut chunks: [Option<Chunk<'_>>; 1024] = [const { None }; 1024];
    chunks.iter_mut().zip(nbts).for_each(|(chunk, nbt)| {
        if let Some(nbt) = nbt {
            *chunk = Chunk::from_compound(nbt);
        }
    });
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
            let morton_code = encode_morton_lut(x, y, z);
            (morton_code, section_to_compacted_octree(&section, &palette))
        })
        .collect::<Vec<_>>();

    let end = Instant::now();
    println!("time to build octrees: {:?}", end.duration_since(start));
    sections.sort_unstable_by_key(|octree| octree.0);

    println!("number of sections: {count}", count = sections.len());

    let builder = RegionOctreeBuilder::new();
    let start = Instant::now();
    let tree = builder.build(sections);

    let end = Instant::now();

    println!("time to build region tree:{:?}", end.duration_since(start));

    tree
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
    octants: Vec<Octant>,
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
    pub fn build(
        mut self,
        mut morton_codes_and_sections: Vec<(u64, SectionOctantResult)>,
    ) -> Option<Octree> {
        let tree_depth = REGION_OCTREE_DEPTH - SECTION_OCTREE_DEPTH; //we are using local
        //coordinates and a region
        //is 32x32 on the x and z
        //axes, so depth is 5

        let result =
            self.recursive_build(tree_depth as u8, morton_codes_and_sections.as_mut_slice());

        println!("octants final len: {}", self.octants.len());

        println!(
            "memory footprint: {}kb",
            (self.octants.len() * size_of::<Octant>()) / 1000
        );

        match result {
            RegionSubtreeResult::Empty => None,
            RegionSubtreeResult::Lod(data) => {
                //this will pretty much never happen
                let octants = vec![Octant {
                    child_mask: u16::MAX,
                    children: [data; 8],
                }];
                Some(Octree {
                    root: Some(0),
                    octants,
                    depth: 9,
                })
            }
            RegionSubtreeResult::Octant(id) => Some(Octree {
                root: Some(id),
                octants: self.octants,
                depth: REGION_OCTREE_DEPTH as u8,
            }),
        }
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
        let mut octant = Octant::default();
        octant.init_children_with(|child_index| {
            let child_index = child_index as u64;
            let data = data_opt.take().unwrap();

            let prefix = (child_index << prefix_shift_amount) | prefix_base;

            if new_depth > 0 {
                let slice_end_index = data.partition_point(|(value, _)| *value <= prefix);

                let (subtree_slice, new_data) = data.split_at_mut(slice_end_index);
                data_opt = Some(new_data);
                if subtree_slice.is_empty() {
                    return (ChildType::Empty, 0);
                    //*child_mut = Child::None;
                }

                let child = self.recursive_build(new_depth, subtree_slice);

                match child {
                    RegionSubtreeResult::Empty => (ChildType::Empty, 0),
                    RegionSubtreeResult::Lod(data) => {
                        child_count += 1;
                        (ChildType::Leaf, data)
                    }
                    RegionSubtreeResult::Octant(octant) => {
                        child_count += 1;
                        (ChildType::Octant, octant)
                    }
                }
            } else {
                assert!(data.len() <= 8);
                let ret_val = if let Some((_, section)) = data.get_mut(child_index as usize) {
                    match section {
                        SectionOctantResult::Subtree {
                            section_octants,
                            root,
                        } => {
                            child_count += 1;
                            let current_octants_len = self.octants.len() as u32;

                            let new_root = *root + current_octants_len;

                            section_octants.iter_mut().for_each(|octant| {
                                octant.iter_children_mut().for_each(|(child_type, value)| {
                                    if child_type == ChildType::Octant {
                                        *value += current_octants_len;
                                    }
                                });
                            });
                            let taken = std::mem::take(section_octants);
                            self.octants.extend(taken);
                            (ChildType::Octant, new_root)
                        }
                        SectionOctantResult::Empty => (ChildType::Empty, 0),
                        SectionOctantResult::Lod(data) => {
                            child_count += 1;
                            (ChildType::Leaf, *data)
                        }
                    }
                } else {
                    (ChildType::Empty, 0)
                };
                data_opt = Some(data);
                ret_val
            }
        });

        if octant.is_compactable() {
            if octant.is_empty() {
                RegionSubtreeResult::Empty
            } else {
                RegionSubtreeResult::Lod(octant.get_child(0).1)
            }
        } else {
            let octant_id = self.octants.len();
            self.octants.push(octant);
            RegionSubtreeResult::Octant(octant_id as u32)
        }
    }
}

pub const SECTION_OCTREE_DEPTH: usize = 4;
pub const CHILD_COUNT: usize = 8;

#[derive(Default, Debug)]
struct SectionOctantBuilder {
    octants: Vec<Octant>,
    child_buffers: [Octant; SECTION_OCTREE_DEPTH - 1],
}

#[derive(Debug, Default)]
pub enum SectionOctantResult {
    Subtree {
        section_octants: Vec<Octant>,
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

        let root_octant = &self.child_buffers[0];
        if root_octant.is_compactable() {
            let child = root_octant.get_child(0);
            match child.0 {
                ChildType::Empty => SectionOctantResult::Empty,
                ChildType::Leaf => SectionOctantResult::Lod(child.1),
                ChildType::Octant => unreachable!(),
            }
        } else {
            let octants_len = self.octants.len();

            self.octants.push(root_octant.clone());

            self.octants.iter_mut().for_each(|octant| {
                octant
                    .iter_children_mut()
                    .for_each(|(child_type, child_data)| {
                        if child_type == ChildType::Octant {
                            let new_id = (octants_len as u32) - *child_data;
                            *child_data = new_id;
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
    fn leaves_to_child(&mut self, leaf_data: &[Option<NonZeroU32>; 8]) -> (ChildType, u32) {
        let first = &leaf_data[0];
        let mut octant = Octant::default();
        octant.init_children_with(|i| {
            if let Some(value) = leaf_data[i as usize] {
                (ChildType::Leaf, value.get())
            } else {
                (ChildType::Empty, 0)
            }
        });

        if octant.is_compactable() {
            if first.is_some() {
                (ChildType::Leaf, first.unwrap().get())
            } else {
                (ChildType::Empty, 0)
            }
        } else {
            let octant_id = self.octants.len() as u32;
            self.octants.push(octant);
            (ChildType::Octant, octant_id)
        }
    }

    fn insert_child_and_compact(&mut self, mut new_child: (ChildType, u32)) {
        let mut search_depth = SECTION_OCTREE_DEPTH - 2;
        loop {
            let current_octant = &mut self.child_buffers[search_depth];
            if let Some(free_slot_index) = current_octant.free_slot() {
                current_octant.overwrite_child(new_child.0, new_child.1, free_slot_index);
                break;
            } else {
                let first_child = current_octant.get_child(0);
                new_child = if current_octant.is_compactable() {
                    first_child
                } else {
                    let octant_id = self.octants.len();

                    self.octants.push(current_octant.clone());
                    (ChildType::Octant, octant_id as u32)
                };
                current_octant.clear();
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
        return if remapped_palette.is_empty() {
            //this shouldn't happen, but we'll treat the section as full of air
            SectionOctantResult::Empty
        } else {
            //palette is known to contain one element
            if let Some(section_fill_block) = remapped_palette.first() {
                if *section_fill_block == 0 {
                    //UNWRAP: we've ensured the length is 1
                    SectionOctantResult::Empty
                } else {
                    SectionOctantResult::Lod(*section_fill_block)
                }
            } else {
                unreachable!()
            }
        };
    }
    let mut morton_order_data: [Option<NonZeroU32>; 4096] = [Option::None; 4096];

    for (i, palette_index) in section.iter_block_indices().enumerate() {
        let (x, y, z) = section_index_to_block_coordinates(i as u16);
        let morton_code = encode_morton_lut(x as u64, y as u64, z as u64);

        let value = remapped_palette
            .get(palette_index as usize)
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

static MORTON_ARRAY_X: [u32; 4096] = section_morton_code_array_x();

static MORTON_ARRAY_Y: [u32; 4096] = section_morton_code_array_y();

static MORTON_ARRAY_Z: [u32; 4096] = section_morton_code_array_z();

const fn section_morton_code_array_x() -> [u32; 4096] {
    let mut array_x = [0u32; 4096];
    let mut i = 0_usize;
    loop {
        array_x[i] = (part_by_2(i as u64)) as u32;
        i += 1;
        if i > 4095 {
            break;
        }
    }
    array_x
}

const fn section_morton_code_array_y() -> [u32; 4096] {
    let mut array_y = [0u32; 4096];
    let mut i = 0_usize;
    loop {
        array_y[i] = (part_by_2(i as u64) << 1) as u32;
        i += 1;
        if i > 4095 {
            break;
        }
    }
    array_y
}

const fn section_morton_code_array_z() -> [u32; 4096] {
    let mut array_z = [0u32; 4096];
    let mut i = 0_usize;
    loop {
        array_z[i] = (part_by_2(i as u64) << 2) as u32;
        i += 1;
        if i > 4095 {
            break;
        }
    }
    array_z
}

fn encode_morton_lut(x: u64, y: u64, z: u64) -> u64 {
    (MORTON_ARRAY_Z[z as usize] + MORTON_ARRAY_Y[y as usize] + MORTON_ARRAY_X[x as usize]) as u64
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
const fn part_by_2(val: u64) -> u64 {
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
    pub fn sizes() {
        println!("size of Octant: {size}", size = size_of::<Octant>());
    }

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
    #[test]
    pub fn morton_code_lut_test() {
        for x in 0..1024 {
            for y in 0..1024 {
                for z in 0..1024 {
                    assert_eq!(encode_morton(x, y, z), encode_morton_lut(x, y, z))
                }
            }
        }
    }
}
