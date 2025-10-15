use anyhow::Chain;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use spider_eye::face::common::face_name::FaceName;

pub struct CuboidFlags(u32);

bitflags! {
    impl CuboidFlags:u32{
        const WEST_FACE_SHOWN = 0b00000000000000000000000000000001;
        const EAST_FACE_SHOWN = 0b00000000000000000000000000000010;
        const DOWN_FACE_SHOWN = 0b00000000000000000000000000000100;
        const UP_FACE_SHOWN = 0b00000000000000000000000000001000;
        const NORTH_FACE_SHOWN = 0b00000000000000000000000000010000;
        const SOUTH_FACE_SHOWN = 0b00000000000000000000000000100000;
    }
}

impl CuboidFlags {
    pub const ALL_FACES: CuboidFlags = CuboidFlags::all();
}

impl From<FaceName> for CuboidFlags {
    fn from(value: FaceName) -> Self {
        match value {
            FaceName::Down => CuboidFlags::DOWN_FACE_SHOWN,
            FaceName::Up => CuboidFlags::UP_FACE_SHOWN,
            FaceName::North => CuboidFlags::NORTH_FACE_SHOWN,
            FaceName::South => CuboidFlags::SOUTH_FACE_SHOWN,
            FaceName::West => CuboidFlags::WEST_FACE_SHOWN,
            FaceName::East => CuboidFlags::EAST_FACE_SHOWN,
        }
    }
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Cuboid {
    pub flags: u32,
    pub matrix_id: u32,         //index to a matrix for rotation,scale,translation
    pub material_ids: [u32; 6], // 24 bytes  = 32 bytes(aligned)
    pub uvs: [u32; 12],         // 48 bytes (aligned)
}
