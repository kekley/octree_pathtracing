use crate::vec3::Vec3;

pub struct Texture {
    pub albedo: Vec3,
}

impl Texture {
    pub fn value(&self, u: f64, v: f64) -> Vec3 {
        self.albedo
    }


}


