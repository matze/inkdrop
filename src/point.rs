#[derive(Copy, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn distance(&self, other: &Self) -> f64 {
        let xs = self.x - other.x;
        let ys = self.y - other.y;
        ((xs * xs) + (ys * ys)).sqrt()
    }
}
