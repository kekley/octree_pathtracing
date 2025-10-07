use bytemuck::{Pod, Zeroable};

#[repr(C, align(16))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub camera_scaled_view_dir: [f32; 3],
    pub traversal_start_idx: u32,
    pub camera_scaled_view_right: [f32; 3],
    pub scale: u32,
    pub camera_view_up_ortho: [f32; 3],
    pub inv_image_size_x: f32,
    pub camera_world_position: [f32; 3],
    pub inv_image_size_y: f32,
}
