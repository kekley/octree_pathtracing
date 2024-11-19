use crate::vec3::Vec3;

#[derive(Debug)]
pub enum Texture {
    Color(Vec3),
    Image(),
    CheckerBoard {
        inv_scale: f64,
        a: Box<Texture>,
        b: Box<Texture>,
    },
}

impl Texture {
    pub fn value(&self, u: f64, v: f64, point: &Vec3) -> Vec3 {
        match self {
            Texture::Color(color) => return *color,
            Texture::Image() => todo!(),
            Texture::CheckerBoard { inv_scale, a, b } => {
                let x_int = (point.x * inv_scale).floor();
                let y_int = (point.y * inv_scale).floor();
                let z_int = (point.z * inv_scale).floor();

                let is_even = (x_int + y_int + z_int) as u64 % 2 == 0;

                if is_even {
                    a.value(u, v, point)
                } else {
                    b.value(u, v, point)
                }
            }
        }
    }
}
