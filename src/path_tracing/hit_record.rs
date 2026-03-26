use glam::{Vec2, Vec3A};

pub struct HitRecord {
    t_enter: f32,
    t_exit: f32,
    uv: Vec2,
    normal: Vec3A,
}

impl HitRecord {
    pub const MISS: Self = Self {
        t_enter: f32::INFINITY,
        t_exit: f32::NEG_INFINITY,
        uv: Vec2::ZERO,
        normal: Vec3A::Y,
    };
    pub fn new(t_enter: f32, t_exit: f32, uv: Vec2, normal: Vec3A) -> Self {
        if t_enter > t_exit {
            panic!("Invalid Hit Record: {t_enter}, {t_exit}");
        }
        HitRecord {
            t_enter,
            t_exit,
            uv,
            normal,
        }
    }
    pub fn get_t_enter() -> f32 {
        todo!();
    }
    pub fn get_t_exit() -> f32 {
        todo!();
    }
    pub fn calc_uv() -> Vec2 {
        todo!();
    }
    pub fn calc_normal() -> Vec3A {
        todo!();
    }

    pub fn is_hit() -> bool {
        todo!();
    }
}
