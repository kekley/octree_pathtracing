use crate::octree::builders::region::encode_morton_lut;
use crate::octree::builders::region::section_index_to_block_coordinates;
use crate::octree::world::ChildType;
use crate::octree::world::Octant;
use crate::octree::world::OctantId;
use std::num::NonZeroU32;

use mc_utils::section::borrow::Section;

pub const SECTION_OCTREE_DEPTH: usize = 4;
pub const CHILD_COUNT: usize = 8;

#[derive(Default, Debug)]
struct SectionOctantBuilder {
    octants: Vec<Octant>,
    octant_building_buffers: [Octant; SECTION_OCTREE_DEPTH - 1],
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
    pub fn build_section_octant_from_morton_data(
        mut self,
        morton_order_section_data: &[Option<NonZeroU32>; 4096],
    ) -> SectionOctantResult {
        //Split the array of section data in morton order into chunks of 8 for building our octants
        let (chunks, remainder) = morton_order_section_data.as_chunks::<CHILD_COUNT>();

        assert!(remainder.is_empty());

        chunks.iter().for_each(|depth_1_octant| {
            let child = self.leaves_to_child(depth_1_octant);
            self.insert_child_and_compact(child);
        });

        let root_octant = &self.octant_building_buffers[0];

        if root_octant.is_compactable() {
            let child = root_octant.get_child(0);
            match child.0 {
                ChildType::Empty => SectionOctantResult::Empty,
                ChildType::Leaf => SectionOctantResult::Lod(child.1),
                ChildType::Octant => unreachable!(),
            }
        } else {
            //The resulting tree is in reverse morton order, so reverse the the octants vector and
            //update indices to match
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
    //Find the lowest level of the octree we are building we can insert `new_child` at. Handles
    fn insert_child_and_compact(&mut self, mut new_child: (ChildType, u32)) {
        let mut compaction_depth = SECTION_OCTREE_DEPTH - 2;
        loop {
            let current_search_octant = &mut self.octant_building_buffers[compaction_depth];
            if let Some(free_slot_index) = current_search_octant.free_slot() {
                current_search_octant.overwrite_child(new_child.0, new_child.1, free_slot_index);
                break;
            } else {
                //This octant at this depth is full, compact it and move it up the tree

                //If every child of this octant contains the same leaf data, propogate the leaf
                //value up
                new_child = if current_search_octant.is_compactable() {
                    let ret = current_search_octant.get_child(0);
                    current_search_octant.clear();
                    current_search_octant.overwrite_child(new_child.0, new_child.1, 0);
                    ret
                } else {
                    let octant_id = self.octants.len();
                    self.octants.push(current_search_octant.clone());
                    current_search_octant.clear();
                    current_search_octant.overwrite_child(new_child.0, new_child.1, 0);
                    (ChildType::Octant, octant_id as u32)
                };
                compaction_depth -= 1;
            }
        }
    }
}

pub(crate) fn section_to_compacted_octree(
    section: &Section<'_, '_>,
    remapped_palette: &[u32],
) -> SectionOctantResult {
    if remapped_palette.len() < 2 {
        // Early return for a palette with only one entry
        return if remapped_palette.is_empty() {
            //this shouldn't happen, but we'll treat the section as full of air
            SectionOctantResult::Empty
        } else {
            //UNWRAP: palette is known to contain only one element
            let section_fill_block = remapped_palette.first().unwrap();
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
        let morton_code = encode_morton_lut(x as u64, y as u64, z as u64);

        let value = remapped_palette
            .get(palette_index as usize)
            .expect("index should be in range of palette");

        morton_order_data[morton_code as usize] = NonZeroU32::new(*value);
    }

    let builder = SectionOctantBuilder::new();

    builder.build_section_octant_from_morton_data(&morton_order_data)
}
