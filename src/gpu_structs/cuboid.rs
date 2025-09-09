use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

pub struct CuboidFlags(u32);

bitflags! {
    impl CuboidFlags:u32{

        const WEST_FACE_HIDDEN = 0b00000000000000000000000000000001;
        const EAST_FACE_HIDDEN = 0b00000000000000000000000000000010;
        const DOWN_FACE_HIDDEN = 0b00000000000000000000000000000100;
        const UP_FACE_HIDDEN = 0b00000000000000000000000000001000;
        const NORTH_FACE_HIDDEN = 0b00000000000000000000000000010000;
        const SOUTH_FACE_HIDDEN = 0b00000000000000000000000000100000;
    }
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Cuboid {
    flags: u32,
    matrix_id: u32,         //index to a matrix for rotation,scale,translation
    material_ids: [u32; 6], // 24 bytes  = 32 bytes(aligned)
    uvs: [u32; 12],         // 48 bytes (aligned)
}
