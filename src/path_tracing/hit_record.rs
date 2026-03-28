use glam::{Vec2, Vec3A};

pub struct AABBHitRecord {
    pub t_enter: f32,
    pub t_exit: f32,
    pub uv: Vec2,
    pub normal: Vec3A,
    pub face_id: u32,
}
