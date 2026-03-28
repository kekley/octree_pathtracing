#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X = 0,
    Y = 1,
    Z = 2,
}
impl Axis {
    pub fn iter() -> AxisIter {
        AxisIter {
            axis: Some(Axis::X),
        }
    }
}

impl IntoIterator for Axis {
    type Item = Axis;

    type IntoIter = AxisIter;

    fn into_iter(self) -> Self::IntoIter {
        AxisIter { axis: Some(self) }
    }
}

impl Into<usize> for Axis {
    fn into(self) -> usize {
        self as usize
    }
}

pub struct AxisIter {
    axis: Option<Axis>,
}
impl Iterator for AxisIter {
    type Item = Axis;

    fn next(&mut self) -> Option<Self::Item> {
        let ret_val = self.axis;
        self.axis = match self.axis? {
            Axis::X => Some(Axis::Y),
            Axis::Y => Some(Axis::Z),
            Axis::Z => None,
        };

        ret_val
    }
}
