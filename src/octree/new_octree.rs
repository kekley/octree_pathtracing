use std::{
    fmt::Debug,
    hint::black_box,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Instant,
};

use eframe::{glow::DEPTH, wgpu::Instance};
use hashbrown::HashMap;
use lasso::Spur;
use nonany::NonAnyU32;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use spider_eye::{
    blockstate::borrow::BlockState,
    borrow::{nbt_compound::RootNBTCompound, nbt_string::NBTStr},
    chunk::{self, borrow::Chunk},
    owned::nbt_string::NBTString,
    region::borrow::Region,
    section::borrow::{Palette, Section},
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

#[derive(Debug, Default)]
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
            Self::Octant(arg0) => Self::Octant(arg0.clone()),
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

pub fn construct(target_depth: u8) {
    let region = Region::load_from_file("./world/r.0.0.mca").expect("Could not load region");

    let start = Instant::now();
    let region_chunk_data = region.load_all_chunk_data();
    let end = Instant::now();
    println!("time loading chunks: {:?}", end.duration_since(start));

    let start = Instant::now();
    let nbts = region_chunk_data
        .par_iter()
        .flatten()
        .map(|chunk_data| RootNBTCompound::from_bytes(chunk_data.as_slice()).unwrap())
        .collect::<Vec<_>>();
    let end = Instant::now();

    println!("time parsing nbt: {:?}", end.duration_since(start));

    let start = Instant::now();
    let chunks = nbts
        .into_iter()
        .flat_map(Chunk::from_compound)
        .collect::<Vec<_>>();
    let end = Instant::now();

    println!("time creating chunks: {:?}", end.duration_since(start));

    let mut blockstate_map: HashMap<NBTString, usize> = HashMap::new();

    let air = NBTString::new_from_str("minecraft:air#normal");

    blockstate_map.insert(air, 0);

    let start = Instant::now();

    let sections = chunks
        .iter()
        .filter_map(|chunk| {
            let sections = chunk.get_sections()?;

            Some(sections.iter_sections())
        })
        .flatten()
        .collect::<Vec<_>>();

    let sections_and_palettes: (Vec<Section<'_, '_>>, Vec<Vec<usize>>) = sections
        .into_iter()
        .map(|section| {
            let palette = section.get_palette();
            let mapped_palette: Vec<usize> = palette
                .iter()
                .map(|blockstate| {
                    let mapped_state = blockstate.to_mapped_state();
                    let current_len = blockstate_map.len();
                    let value = blockstate_map
                        .entry(mapped_state)
                        .or_insert_with(|| current_len);
                    *value
                })
                .collect();
            (section, mapped_palette)
        })
        .collect::<(Vec<_>, Vec<_>)>();
    for block in blockstate_map.keys() {
        println!("{}", block.as_str());
    }

    let end = Instant::now();

    println!(
        "time remapping section palettes: {:?}",
        end.duration_since(start)
    );
    let start = Instant::now();
    let octrees: Vec<Octree<usize>> = sections_and_palettes
        .par_iter()
        .map(|(section, palette)| section_to_octree(section, palette))
        .collect();
    let end = Instant::now();

    println!("time to build octrees: {:?}", end.duration_since(start));
}

pub fn section_to_octree(section: &Section<'_, '_>, palette: &Vec<usize>) -> Octree<usize> {
    let mut octants = vec![];
    const TARGET_DEPTH: usize = 4;

    let mut morton_order_data: [Option<NonZeroUsize>; 4096] = [Option::None; 4096];

    for (i, palette_index) in section.iter_block_indices().enumerate() {
        let (x, y, z) = index_to_coordinates(i as u64);
        let morton_code = calculate_morton_code(x, y, z);

        let value = palette
            .get(palette_index as usize)
            .expect("index should be in range of palette");

        morton_order_data[morton_code as usize] = NonZeroUsize::new(*value);
    }

    let mut child_buffers: [[Option<Child<usize>>; 8]; TARGET_DEPTH] =
        [Default::default(); TARGET_DEPTH];

    let mut voxels_iterated = 0;

    while voxels_iterated < 4096 {
        let deepest_buffer = child_buffers
            .get_mut(TARGET_DEPTH - 1)
            .expect("octant buffer should be of size TARGET_DEPTH");
        let mut child_count = 0;
        (0..8).for_each(|child_index| {
            if let Some(data_opt) = morton_order_data.get(voxels_iterated) {
                if let Some(data) = data_opt {
                    deepest_buffer[child_index] = Some(Child::Leaf(data.get()));
                    child_count += 1;
                } else {
                    deepest_buffer[child_index] = Some(Child::None);
                }
            }
            voxels_iterated += 1;
        });
        let mut new_child: Child<usize> = if child_count > 0 {
            let new_octant = Octant {
                child_count: child_count as u8,
                children: child_buffers[TARGET_DEPTH - 1].map(|opt| opt.unwrap()),
            };
            let octant_id = octants.len() as u32;
            octants.push(new_octant);
            Child::Octant(octant_id)
        } else {
            Child::None
        };
        child_buffers[TARGET_DEPTH - 1]
            .iter_mut()
            .for_each(|child| *child = None);

        let mut search_depth = TARGET_DEPTH - 2;
        if voxels_iterated > 4090 {
            black_box(());
        };

        loop {
            let mut free_slot: Option<&mut Option<Child<usize>>> = None;
            let mut child_count = 0;
            for child in &mut child_buffers[search_depth] {
                if child.is_none() {
                    if free_slot.is_none() {
                        free_slot = Some(child);
                        break;
                    }
                } else if !child.unwrap().is_none() {
                    child_count += 1
                }
            }

            if let Some(free_slot) = free_slot {
                *free_slot = Some(new_child);
                break;
            } else {
                new_child = if child_count > 0 {
                    let new_octant = Octant {
                        child_count: child_count as u8,
                        children: child_buffers[search_depth].map(|opt| opt.unwrap()),
                    };
                    let octant_id = octants.len() as u32;
                    octants.push(new_octant);
                    Child::Octant(octant_id)
                } else {
                    Child::None
                };
                child_buffers[search_depth]
                    .iter_mut()
                    .for_each(|child| *child = None);

                search_depth -= 1;
            }
        }
    }

    let child_count = child_buffers[0]
        .iter()
        .filter(|child| child.is_some_and(|child| !child.is_none()))
        .count();
    if child_count > 0 {
        let new_octant = Octant {
            child_count: child_count as u8,
            children: child_buffers[0].map(|opt| {
                if let Some(child) = opt {
                    child
                } else {
                    Child::None
                }
            }),
        };
        let octant_id = octants.len() as u32;
        octants.push(new_octant);

        Octree {
            scale: f32::exp(-4.0),
            root: Some(octant_id),
            octants,
            depth: 4,
        }
    } else {
        Octree {
            scale: 0.0,
            root: None,
            octants,
            depth: 0,
        }
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
    x
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
