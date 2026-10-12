//! Small f64 vectors in the C0 frame.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    #[inline]
    pub const fn new(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }
    #[inline]
    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    /// z of the 3D cross product
    #[inline]
    pub fn cross(self, o: Vec2) -> f64 {
        self.x * o.y - self.y * o.x
    }
    #[inline]
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    #[inline]
    pub fn dist(self, o: Vec2) -> f64 {
        (self - o).len()
    }
    /// the unit vector, or zero for a zero vector
    #[inline]
    pub fn norm(self) -> Vec2 {
        let l = self.len();
        if l > 0.0 { self / l } else { Vec2::ZERO }
    }
    #[inline]
    pub fn extend(self, z: f64) -> Vec3 {
        Vec3::new(self.x, self.y, z)
    }
    /// the unit vector of an azimuth (from +x towards +y)
    #[inline]
    pub fn from_azimuth(a: f64) -> Vec2 {
        Vec2::new(crate::math::cos(a), crate::math::sin(a))
    }
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const Z: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }
    #[inline]
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    #[inline]
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    #[inline]
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    #[inline]
    pub fn norm(self) -> Vec3 {
        let l = self.len();
        if l > 0.0 { self / l } else { Vec3::ZERO }
    }
    #[inline]
    pub fn xy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

macro_rules! ops {
    ($t:ident { $($f:ident),+ }) => {
        impl Add for $t { type Output = $t; #[inline] fn add(self, o: $t) -> $t { $t { $($f: self.$f + o.$f),+ } } }
        impl Sub for $t { type Output = $t; #[inline] fn sub(self, o: $t) -> $t { $t { $($f: self.$f - o.$f),+ } } }
        impl Mul<f64> for $t { type Output = $t; #[inline] fn mul(self, k: f64) -> $t { $t { $($f: self.$f * k),+ } } }
        impl Div<f64> for $t { type Output = $t; #[inline] fn div(self, k: f64) -> $t { $t { $($f: self.$f / k),+ } } }
        impl Neg for $t { type Output = $t; #[inline] fn neg(self) -> $t { $t { $($f: -self.$f),+ } } }
        impl AddAssign for $t { #[inline] fn add_assign(&mut self, o: $t) { $(self.$f += o.$f;)+ } }
        impl SubAssign for $t { #[inline] fn sub_assign(&mut self, o: $t) { $(self.$f -= o.$f;)+ } }
    };
}
ops!(Vec2 { x, y });
ops!(Vec3 { x, y, z });
