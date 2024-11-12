use std::{
    iter::Product,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    /// All zeroes.
    pub const ZERO: Self = Self::splat(0.0);

    /// All ones.
    pub const ONE: Self = Self::splat(1.0);

    /// All negative ones.
    pub const NEG_ONE: Self = Self::splat(-1.0);

    /// All `f64::MIN`.
    pub const MIN: Self = Self::splat(f64::MIN);

    /// All `f64::MAX`.
    pub const MAX: Self = Self::splat(f64::MAX);

    /// All `f64::NAN`.
    pub const NAN: Self = Self::splat(f64::NAN);

    /// All `f64::INFINITY`.
    pub const INFINITY: Self = Self::splat(f64::INFINITY);

    /// All `f64::NEG_INFINITY`.
    pub const NEG_INFINITY: Self = Self::splat(f64::NEG_INFINITY);
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    #[inline]
    pub const fn splat(val: f64) -> Self {
        Self {
            x: val,
            y: val,
            z: val,
        }
    }

    #[inline]
    pub fn get_axis(&self, n: u8) -> f64 {
        if n == 1 {
            return self.y;
        }
        if n == 2 {
            return self.z;
        }
        self.x
    }
    pub fn write_as_text_to_stream(vec: &Vec3, stream: &mut dyn std::io::Write) {
        stream
            .write(format!("{} {} {}\n", vec.x as i64, vec.y as i64, vec.z as i64).as_bytes())
            .expect("Unable to write to stream");
    }

    #[inline]
    pub fn length(&self) -> f64 {
        f64::sqrt(self.length_squared())
    }
    #[inline]
    pub fn length_squared(&self) -> f64 {
        self.dot(*self)
    }
    #[inline]
    pub fn dot(self, rhs: Self) -> f64 {
        (self.x * rhs.x) + (self.y * rhs.y) + (self.z * rhs.z)
    }
    #[inline]
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - rhs.y * self.z,
            y: self.z * rhs.x - rhs.z * self.x,
            z: self.x * rhs.y - rhs.x * self.y,
        }
    }
    #[inline]
    pub fn normalize(&self) -> Self {
        self / self.length()
    }

    #[inline]
    pub fn near_zero(&self) -> bool {
        let s = 1e-8;
        f64::abs(self.x) < s && f64::abs(self.y) < s && f64::abs(self.z) < s
    }

    #[inline]
    pub fn reflect(&self, n: Vec3) -> Self {
        self - 2f64 * self.dot(n) * n
    }
    #[inline]
    pub fn refract(&self, normal: Vec3, etai_over_etat: f64) -> Vec3 {
        let cos_theta = f64::min((-self).dot(normal), 1.0);

        let r_out_perp = etai_over_etat * (self + cos_theta * normal);
        let r_out_parallel = -(1.0 - r_out_perp.length_squared()).abs().sqrt() * normal;

        r_out_perp + r_out_parallel
    }
}

impl Default for Vec3 {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}
impl Sub<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x.sub(rhs.x),
            y: self.y.sub(rhs.y),
            z: self.z.sub(rhs.z),
        }
    }
}

impl Sub<&Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: &Vec3) -> Vec3 {
        self.sub(*rhs)
    }
}

impl Sub<&Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: &Vec3) -> Vec3 {
        (*self).sub(*rhs)
    }
}

impl Sub<Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: Vec3) -> Vec3 {
        (*self).sub(rhs)
    }
}

impl SubAssign<Vec3> for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec3) {
        self.x.sub_assign(rhs.x);
        self.y.sub_assign(rhs.y);
        self.z.sub_assign(rhs.z);
    }
}

impl SubAssign<&Self> for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        self.sub_assign(*rhs)
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x.mul(rhs.x),
            y: self.y.mul(rhs.y),
            z: self.z.mul(rhs.z),
        }
    }
}

impl Mul<&Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: &Vec3) -> Vec3 {
        self.mul(*rhs)
    }
}

impl Mul<&Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: &Vec3) -> Vec3 {
        (*self).mul(*rhs)
    }
}

impl Mul<Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        (*self).mul(rhs)
    }
}

impl MulAssign<Vec3> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x.mul_assign(rhs.x);
        self.y.mul_assign(rhs.y);
        self.z.mul_assign(rhs.z);
    }
}

impl MulAssign<&Self> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Self) {
        self.mul_assign(*rhs)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self {
        Self {
            x: self.x.mul(rhs),
            y: self.y.mul(rhs),
            z: self.z.mul(rhs),
        }
    }
}

impl Mul<&f64> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: &f64) -> Vec3 {
        self.mul(*rhs)
    }
}

impl Mul<&f64> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: &f64) -> Vec3 {
        (*self).mul(*rhs)
    }
}

impl Mul<f64> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: f64) -> Vec3 {
        (*self).mul(rhs)
    }
}

impl MulAssign<f64> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        self.x.mul_assign(rhs);
        self.y.mul_assign(rhs);
        self.z.mul_assign(rhs);
    }
}

impl MulAssign<&f64> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: &f64) {
        self.mul_assign(*rhs)
    }
}

impl Mul<Vec3> for f64 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.mul(rhs.x),
            y: self.mul(rhs.y),
            z: self.mul(rhs.z),
        }
    }
}

impl Mul<&Vec3> for f64 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: &Vec3) -> Vec3 {
        self.mul(*rhs)
    }
}

impl Mul<&Vec3> for &f64 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: &Vec3) -> Vec3 {
        (*self).mul(*rhs)
    }
}

impl Mul<Vec3> for &f64 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        (*self).mul(rhs)
    }
}

impl Product for Vec3 {
    #[inline]
    fn product<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::ONE, Self::mul)
    }
}

impl<'a> Product<&'a Self> for Vec3 {
    #[inline]
    fn product<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(Self::ONE, |a, &b| Self::mul(a, b))
    }
}

impl Div<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x.div(rhs.x),
            y: self.y.div(rhs.y),
            z: self.z.div(rhs.z),
        }
    }
}

impl Div<&Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: &Vec3) -> Vec3 {
        self.div(*rhs)
    }
}

impl Div<&Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: &Vec3) -> Vec3 {
        (*self).div(*rhs)
    }
}

impl Div<Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: Vec3) -> Vec3 {
        (*self).div(rhs)
    }
}

impl DivAssign<Vec3> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x.div_assign(rhs.x);
        self.y.div_assign(rhs.y);
        self.z.div_assign(rhs.z);
    }
}

impl DivAssign<&Self> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: &Self) {
        self.div_assign(*rhs)
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self {
        Self {
            x: self.x.div(rhs),
            y: self.y.div(rhs),
            z: self.z.div(rhs),
        }
    }
}

impl Div<&f64> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: &f64) -> Vec3 {
        self.div(*rhs)
    }
}

impl Div<&f64> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: &f64) -> Vec3 {
        (*self).div(*rhs)
    }
}

impl Div<f64> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: f64) -> Vec3 {
        (*self).div(rhs)
    }
}

impl DivAssign<f64> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        self.x.div_assign(rhs);
        self.y.div_assign(rhs);
        self.z.div_assign(rhs);
    }
}

impl DivAssign<&f64> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: &f64) {
        self.div_assign(*rhs)
    }
}

impl Div<Vec3> for f64 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.div(rhs.x),
            y: self.div(rhs.y),
            z: self.div(rhs.z),
        }
    }
}

impl Div<&Vec3> for f64 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: &Vec3) -> Vec3 {
        self.div(*rhs)
    }
}

impl Div<&Vec3> for &f64 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: &Vec3) -> Vec3 {
        (*self).div(*rhs)
    }
}

impl Div<Vec3> for &f64 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: Vec3) -> Vec3 {
        (*self).div(rhs)
    }
}

impl Add<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x.add(rhs.x),
            y: self.y.add(rhs.y),
            z: self.z.add(rhs.z),
        }
    }
}

impl Add<&Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: &Vec3) -> Vec3 {
        self.add(*rhs)
    }
}

impl Add<&Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: &Vec3) -> Vec3 {
        (*self).add(*rhs)
    }
}

impl Add<Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: Vec3) -> Vec3 {
        (*self).add(rhs)
    }
}

impl AddAssign<Vec3> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
        self.z.add_assign(rhs.z);
    }
}

impl AddAssign<&Self> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        self.add_assign(*rhs)
    }
}

impl Add<f64> for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: f64) -> Self {
        Self {
            x: self.x.add(rhs),
            y: self.y.add(rhs),
            z: self.z.add(rhs),
        }
    }
}

impl Add<&f64> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: &f64) -> Vec3 {
        self.add(*rhs)
    }
}

impl Add<&f64> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: &f64) -> Vec3 {
        (*self).add(*rhs)
    }
}

impl Add<f64> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: f64) -> Vec3 {
        (*self).add(rhs)
    }
}

impl AddAssign<f64> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: f64) {
        self.x.add_assign(rhs);
        self.y.add_assign(rhs);
        self.z.add_assign(rhs);
    }
}

impl AddAssign<&f64> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: &f64) {
        self.add_assign(*rhs)
    }
}

impl Add<Vec3> for f64 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.add(rhs.x),
            y: self.add(rhs.y),
            z: self.add(rhs.z),
        }
    }
}

impl Add<&Vec3> for f64 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: &Vec3) -> Vec3 {
        self.add(*rhs)
    }
}

impl Add<&Vec3> for &f64 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: &Vec3) -> Vec3 {
        (*self).add(*rhs)
    }
}

impl Add<Vec3> for &f64 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: Vec3) -> Vec3 {
        (*self).add(rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: self.x.neg(),
            y: self.y.neg(),
            z: self.z.neg(),
        }
    }
}

impl Neg for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        (*self).neg()
    }
}
