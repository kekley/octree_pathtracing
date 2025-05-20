use super::{ray::Ray, resource_manager::MaterialID};
use glam::{Affine3A, Vec2, Vec3, Vec3A, Vec4};
use spider_eye::{block_face::FaceName, block_texture::Uv};

#[derive(Debug, Clone, Default)]
pub struct Quad {
    pub origin: Vec3A,
    v: Vec3A,
    u: Vec3A,
    w: Vec3A,
    d: f32,
    pub normal: Vec3A,
    pub material_id: MaterialID,
    pub tint: Vec4,
    texture_u_range: Vec2,
    texture_v_range: Vec2,
}
impl Quad {
    pub fn from_face_name(
        face: &FaceName,
        uv: &Option<Uv>,
        from: &[f32; 3],
        to: &[f32; 3],
        material_id: MaterialID,
    ) -> Self {
        let from = Vec3A::from_slice(from) / 16.0;

        let to = Vec3A::from_slice(to) / 16.0;
        let (origin, u, v) = match face {
            FaceName::Down => {
                let origin = Vec3A::new(from.x, from.y, from.z);
                let u = Vec3A::new(to.x - from.x, 0.0, 0.0);
                let v = Vec3A::new(0.0, 0.0, to.z - from.z);
                (origin, u, v)
            }

            FaceName::Up => {
                let origin = Vec3A::new(to.x, to.y, from.z);
                let u = Vec3A::new(from.x - to.x, 0.0, 0.0);
                let v = Vec3A::new(0.0, 0.0, to.z - from.z);
                (origin, u, v)
            }

            FaceName::North => {
                let origin = Vec3A::new(to.x, from.y, from.z);
                let u = Vec3A::new(from.x - to.x, 0.0, 0.0); // note: negative delta in X
                let v = Vec3A::new(0.0, to.y - from.y, 0.0);
                (origin, u, v)
            }

            FaceName::South => {
                let origin = Vec3A::new(from.x, from.y, to.z);
                let u = Vec3A::new(to.x - from.x, 0.0, 0.0);
                let v = Vec3A::new(0.0, to.y - from.y, 0.0);
                (origin, u, v)
            }

            FaceName::West => {
                let origin = Vec3A::new(from.x, from.y, from.z);
                let u = Vec3A::new(0.0, 0.0, to.z - from.z);
                let v = Vec3A::new(0.0, to.y - from.y, 0.0);
                (origin, u, v)
            }

            FaceName::East => {
                let origin = Vec3A::new(to.x, from.y, to.z);
                let u = Vec3A::new(0.0, 0.0, from.z - to.z);
                let v = Vec3A::new(0.0, to.y - from.y, 0.0);
                (origin, u, v)
            }
        };
        let texture_u_range = uv
            .as_ref()
            .map(|uv| Vec2::new(uv.x1, uv.x2))
            .unwrap_or(Vec2::new(0.0, 16.0))
            / 16.0;
        let texture_v_range = uv
            .as_ref()
            .map(|uv| Vec2::new(uv.y1, uv.y2))
            .unwrap_or(Vec2::new(0.0, 16.0))
            / 16.0;
        Quad::new(origin, u, v, texture_u_range, texture_v_range, material_id)
    }
}

impl Quad {
    pub fn new(
        origin: Vec3A,
        u: Vec3A,
        v: Vec3A,
        texture_u_range: Vec2,
        texture_v_range: Vec2,
        material_id: MaterialID,
    ) -> Self {
        let n = u.cross(v);
        let normal = n.normalize();
        let w = n / n.dot(n);
        let d = normal.dot(origin);

        Quad {
            origin: origin,
            v: v,
            u: u,
            w: w,
            normal: normal,
            material_id,
            tint: Vec4::ONE,
            texture_u_range,
            texture_v_range,
            d,
        }
    }
    pub fn transform_about_pivot(&mut self, matrix: &Affine3A, pivot: Vec3A) {
        self.origin -= pivot;
        self.origin = matrix.transform_point3a(self.origin);
        self.origin += pivot;
        self.u = matrix.transform_vector3a(self.u);
        self.v = matrix.transform_vector3a(self.v);
        let n = self.u.cross(self.v);
        self.normal = matrix.transform_vector3a(self.normal).normalize();
        self.d = self.normal.dot(self.origin);
        self.w = n / n.dot(n);
    }

    pub fn transform(&mut self, matrix: &Affine3A) {
        /*         self.origin = matrix.transform_point3a(self.origin);
        self.u = matrix.transform_vector3a(self.u);
        self.v = matrix.transform_vector3a(self.v);
        let n = self.u.cross(self.v);

        self.normal = n.normalize();
        self.d = self.normal.dot(self.origin);

        self.w = n / n.dot(n); */
    }
    /*    pub fn hit(&self, ray: &mut Ray, octree_intersect_result: &OctreeIntersectResult<u32>) -> bool {
        // ISSUE WHERE ray.at(Ray::OFFSET).floor() DOESN'T EQUAL VOXEL POS
        let test =
            if octree_intersect_result.face == Face::Top && self.material.name.contains("top") {
                true
            } else {
                false
            };
        let (u, v): (f32, f32);
        let mut i = ray.origin - ray.at(Ray::OFFSET).floor();
        let denominator = ray.get_direction().dot(self.normal);
        if denominator < -Ray::EPSILON || (denominator > Ray::EPSILON && true) {
            let t = -((i * self.normal).element_sum() + self.d) / denominator;
            if test {
                dbg!(i);
                dbg!(ray.at(Ray::OFFSET).floor());
                dbg!(octree_intersect_result);
            }
            if t > -Ray::EPSILON && t < ray.hit.t {
                //plane interesction confirmed
                i = i + ray.get_direction() * t - self.origin;
                u = i.dot(self.xv) * self.xvl;
                v = i.dot(self.yv) * self.yvl;
                if u >= 0.0 && u <= 1.0 && v >= 0.0 && v <= 1.0 {
                    ray.hit.u = self.uv.x + u * self.uv.y;
                    ray.hit.v = self.uv.z + v * self.uv.w;
                    ray.hit.t_next = t;

                    return true;
                }
            }
        }
        return false;
    } */
    pub fn hit(&self, ray: &mut Ray, voxel_position: &Vec3A) -> bool {
        let translated_ray_origin = ray.origin - voxel_position;
        let denom = ray.get_direction().dot(self.normal);
        // ray parallel to plane or backside of quad
        if denom >= -Ray::EPSILON {
            return false;
        }

        let t = (self.d - self.normal.dot(translated_ray_origin)) / denom;
        if t <= 0.0 || t > ray.hit.t_next {
            return false;
        }
        let intersection = translated_ray_origin + ray.get_direction() * t;
        let planar_hit_point = intersection - self.origin;
        let alpha = self.w.dot(planar_hit_point.cross(self.v));
        let beta = self.w.dot(self.u.cross(planar_hit_point));

        if alpha < 0.0 || alpha > 1.0 || beta < 0.0 || beta > 1.0 {
            return false;
        }

        ray.hit.t_next = t;
        ray.hit.u =
            self.texture_u_range.x + alpha * (self.texture_u_range.y - self.texture_u_range.x);
        ray.hit.v =
            self.texture_v_range.x + beta * (self.texture_v_range.y - self.texture_v_range.x);

        true
    }
}
