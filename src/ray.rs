use crate::vec3::Vec3;
#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub time: f64,
}

impl Ray {
    #[inline]
    pub fn at(&self, t: f64) -> Vec3 {
        self.origin + self.direction * t
    }
    #[inline]

    pub fn new(point: Vec3, direction: Vec3) -> Self {
        Self {
            origin: point,
            direction,
            time: 0.0,
        }
    }
    #[inline]
    pub fn create_at(point: Vec3, direction: Vec3, time: f64) -> Self {
        Self {
            origin: point,
            direction: direction,
            time: time,
        }
    }
}
