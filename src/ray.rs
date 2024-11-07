use crate::vec3::Vec3;
#[derive(Debug, Clone, Default)]
pub struct Ray {
    pub point: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn at(point: Vec3) -> Self {
        Self {
            point: point,
            direction: Vec3::default(),
        }
    }

    pub fn new(point: Vec3, direction: Vec3) -> Self {
        Self { point, direction }
    }
}
