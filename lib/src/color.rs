pub fn to_cmyk(r: u8, g: u8, b: u8) -> (f32, f32, f32, f32) {
    let max = r.max(g.max(b)) as f32 / 255.0;
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let black = 1.0 - max;
    let white = 1.0 - black;

    if white == 0.0 {
        return (0.0, 0.0, 0.0, 1.0);
    }

    (
        (1.0 - r - black) / white,
        (1.0 - g - black) / white,
        (1.0 - b - black) / white,
        black,
    )
}

pub fn to_black(r: u8, g: u8, b: u8) -> f32 {
    1.0 - (r.max(g.max(b)) as f32 / 255.0)
}

pub fn invert(c: (f32, f32, f32, f32)) -> Vec<f32> {
    vec![1.0 - c.0, 1.0 - c.1, 1.0 - c.2, 1.0 - c.3]
}
