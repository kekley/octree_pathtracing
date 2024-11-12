use crate::util::INFINITY;

#[derive(Debug, Clone, Copy)]
pub struct Interval {
    pub min: f64,
    pub max: f64,
}

impl Interval {
    pub const EMPTY: Interval = Interval::new(INFINITY, -INFINITY);
    pub const UNIVERSE: Interval = Interval::new(-INFINITY, INFINITY);
    pub const ZEROISH_TO_INFINITY: Interval = Interval::new(0.001f64, INFINITY);
    #[inline]

    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }
    pub fn from_intervals(a: &Interval, b: &Interval) -> Self {
        let min = if a.min <= b.min { a.min } else { b.min };
        let max = if a.max >= b.max { a.max } else { b.max };
        Self { min, max }
    }
    #[inline]
    pub fn size(&self) -> f64 {
        self.max - self.min
    }
    #[inline]

    pub fn conains(&self, x: f64) -> bool {
        x >= self.min && x <= self.max
    }
    #[inline]

    pub fn surrounds(&self, x: f64) -> bool {
        x > self.min && x < self.max
    }
    #[inline]
    pub fn clamp(&self, x: f64) -> f64 {
        if x < self.min {
            return self.min;
        } else if x > self.max {
            return self.max;
        }

        return x;
    }
    #[inline]
    pub fn expand(&self, delta: f64) -> Interval {
        let padding = delta / 2.0;
        Interval {
            min: self.min - padding,
            max: self.max + padding,
        }
    }
}

impl Default for Interval {
    #[inline]

    fn default() -> Self {
        Self {
            min: INFINITY,
            max: -INFINITY,
        }
    }
}
