use crate::{interval::Interval, ray::Ray, vec3::Vec3};

#[derive(Debug, Default)]
pub struct AABB {
    pub x_interval: Interval,
    pub y_interval: Interval,
    pub z_interval: Interval,
}

impl AABB {
    #[inline]
    pub fn new(interval_x: Interval, interval_y: Interval, interval_z: Interval) -> Self {
        Self {
            x_interval: interval_x,
            y_interval: interval_y,
            z_interval: interval_z,
        }
    }
    pub fn from_boxes(a: &AABB, b: &AABB) -> Self {
        let x = Interval::from_intervals(&a.x_interval, &b.x_interval);
        let y = Interval::from_intervals(&a.y_interval, &b.y_interval);
        let z = Interval::from_intervals(&a.y_interval, &b.y_interval);
        Self {
            x_interval: x,
            y_interval: y,
            z_interval: z,
        }
    }
    pub fn from_points(a: Vec3, b: Vec3) -> Self {
        let x_interval = if a.x <= b.x {
            Interval::new(a.x, b.x)
        } else {
            Interval::new(b.x, a.x)
        };
        let y_interval = if a.y <= b.y {
            Interval::new(a.y, b.y)
        } else {
            Interval::new(b.y, a.y)
        };
        let z_interval = if a.z <= b.z {
            Interval::new(a.z, b.z)
        } else {
            Interval::new(b.z, a.z)
        };

        Self {
            x_interval,
            y_interval,
            z_interval,
        }
    }

    pub fn get_interval(&self, n: u8) -> &Interval {
        if n == 1 {
            return &self.y_interval;
        }
        if n == 2 {
            return &self.z_interval;
        }

        return &self.x_interval;
    }

    pub fn hit(&self, ray: &Ray, mut ray_t: Interval) -> bool {
        let ray_origin: &Vec3 = &ray.origin;
        let ray_dir: &Vec3 = &ray.direction;
        for axis in 0..3 {
            let axis_interval = self.get_interval(axis);
            let axis_dir_inverse = 1.0 / ray_dir.get_axis(axis);

            let t0 = (axis_interval.min - ray_origin.get_axis(axis)) * axis_dir_inverse;
            let t1 = (axis_interval.max - ray_origin.get_axis(axis)) * axis_dir_inverse;

            if t0 < t1 {
                ray_t.min = t0.max(ray_t.min);

                ray_t.max = t1.min(ray_t.max);
            } else {
                ray_t.min = t1.max(ray_t.min);

                ray_t.max = t0.min(ray_t.max);
            }
            if ray_t.max <= ray_t.min {
                return false;
            }
        }
        return true;
    }
}
