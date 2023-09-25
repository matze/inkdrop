use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul};
use voronator::delaunator;

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    pub fn origin() -> Self {
        Point::new(0.0, 0.0)
    }

    pub fn distance(&self, other: &Self) -> f64 {
        let xs = self.x - other.x;
        let ys = self.y - other.y;
        ((xs * xs) + (ys * ys)).sqrt()
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        *self = Point::new(self.x + other.x, self.y + other.y);
    }
}

impl Mul<f64> for Point {
    type Output = Self;

    fn mul(self, scale: f64) -> Self {
        Point::new(self.x * scale, self.y * scale)
    }
}

impl Div<f64> for Point {
    type Output = Self;

    fn div(self, divisor: f64) -> Self {
        Point::new(self.x / divisor, self.y / divisor)
    }
}

impl From<delaunator::Point> for Point {
    fn from(p: delaunator::Point) -> Self {
        Point::new(p.x, p.y)
    }
}

impl From<&delaunator::Point> for Point {
    fn from(p: &delaunator::Point) -> Self {
        Point::new(p.x, p.y)
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.distance(other) < 0.00001
    }
}
