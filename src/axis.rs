use glam::Vec3A as Vec3;

pub const LEFT: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
pub const RIGHT: Vec3 = Vec3::new(1.0, 0.0, 0.0);
pub const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);
pub const DOWN: Vec3 = Vec3::new(0.0, -1.0, 0.0);
pub const FORWARD: Vec3 = Vec3::new(0.0, 0.0, 1.0);
pub const BACK: Vec3 = Vec3::new(0.0, 0.0, -1.0);

#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    Forward,
    Back,
}

impl Direction {
    pub fn iter() -> std::slice::Iter<'static, Direction> {
        static DIRECTIONS: [Direction; 6] = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
            Direction::Forward,
            Direction::Back,
        ];

        DIRECTIONS.iter()
    }
}

impl Axis {
    pub fn iter() -> std::slice::Iter<'static, Axis> {
        static AXES: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

        AXES.iter()
    }
}

pub trait AxisOps {
    fn get_axis(&self, axis: Axis) -> f32;
}

impl AxisOps for Vec3 {
    #[inline]
    fn get_axis(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }
}
