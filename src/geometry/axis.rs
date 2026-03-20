#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
}
impl Axis {
    pub fn iter() -> AxisIter {
        AxisIter { current: Axis::X }
    }
}

impl IntoIterator for Axis {
    type Item = Axis;

    type IntoIter = AxisIter;

    fn into_iter(self) -> Self::IntoIter {
        AxisIter { current: self }
    }
}

pub struct AxisIter {
    current: Axis,
}
impl Iterator for AxisIter {
    type Item = Axis;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            Axis::X => Some(Axis::Y),
            Axis::Y => Some(Axis::Z),
            Axis::Z => None,
        }
    }
}
