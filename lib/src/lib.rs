pub mod color;
#[cfg(feature = "gcode")]
pub mod gcode;
pub mod point;
#[cfg(feature = "svg")]
pub mod svg;
pub mod tsp;
pub mod voronoi;

use image::GenericImageView;
use rand::Rng;

pub fn sample_points(
    img: &image::DynamicImage,
    num_points: usize,
    gamma: f32,
    cmyk: bool,
) -> Vec<Vec<point::Point>> {
    let (width, height) = img.dimensions();
    let mut rng = rand::thread_rng();

    // Store points for each channel
    let mut ps = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    while ps.iter().map(|points| points.len()).sum::<usize>() < num_points {
        let x = rng.gen::<f64>() * width as f64;
        let y = rng.gen::<f64>() * height as f64;
        let channels = img.get_pixel(x as u32, y as u32);
        let sample: f32 = rng.gen();

        if cmyk {
            let cmyk = color::invert(color::to_cmyk(channels[0], channels[1], channels[2]));

            for (points, color) in ps.iter_mut().zip(cmyk.iter()) {
                if sample >= color.powf(gamma) {
                    points.push(point::Point::new(x, y));
                }
            }
        } else {
            let black = 1.0 - color::to_black(channels[0], channels[1], channels[2]);

            if sample >= black.powf(gamma) {
                ps[3].push(point::Point::new(x, y));
            }
        }
    }

    ps
}
