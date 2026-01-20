use crate::octree::section::SECTION_OCTREE_DEPTH;
use crate::octree::section::SectionOctantResult;
use crate::octree::section::section_to_compacted_octree;
use crate::octree::world::ChildType;
use crate::octree::world::Octant;
use crate::octree::world::OctantId;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use mc_utils::{
    borrow::nbt_compound::RootNBTCompound, chunk::borrow::Chunk, owned::nbt_string::NBTString,
    region::borrow::Region,
};

const LOWEST_SECTION_INDEX: i8 = -4;

const HIGHEST_SECTION_INDEX: i8 = 19;

pub fn build_region_octree(
    region: Region,
    blockstate_map: Arc<Mutex<hashbrown::HashMap<NBTString, u32>>>,
) -> Option<(Vec<Octant>, u32)> {
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
pub struct RegionOctreeBuilder {
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
    ) -> Option<(Vec<Octant>, u32)> {
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
                Some((octants, 0))
            }
            RegionSubtreeResult::Octant(id) => Some((self.octants, id)),
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

pub(crate) fn encode_morton_lut(x: u64, y: u64, z: u64) -> u64 {
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
pub(crate) fn section_index_to_block_coordinates(index: u16) -> (u16, u16, u16) {
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
        crate::octree::world::construct_all();
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
