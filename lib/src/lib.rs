pub mod color;
pub mod point;
pub mod tsp;
pub mod voronoi;

use anyhow::Result;
use image::GenericImageView;
use rand::Rng;
use std::path;
use svg::node::element::path::Data;
use svg::node::element::Circle;
use svg::node::element::Path;
use svg::Document;

const COLORS: [&str; 4] = ["cyan", "magenta", "yellow", "black"];

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

fn draw_path(document: Document, path: &Vec<point::Point>, color: &str) -> Document {
    if path.is_empty() {
        return document;
    }

    let mut data = Data::new().move_to((path[0].x, path[0].y));

    for point in path.into_iter().skip(1) {
        data = data.line_to((point.x, point.y));
    }

    let path = Path::new()
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", "1.0")
        .set("d", data);

    document.add(path)
}

fn draw_points(document: Document, points: &Vec<point::Point>, color: &str) -> Document {
    let mut document = document;

    for point in points {
        document = document.add(
            Circle::new()
                .set("fill", color)
                .set("cx", point.x)
                .set("cy", point.y)
                .set("r", 1.0),
        );
    }

    document
}

pub fn write_path(
    filename: &path::Path,
    point_sets: &Vec<Vec<point::Point>>,
    width: u32,
    height: u32,
) -> Result<()> {
    let mut document = Document::new().set("viewBox", (0, 0, width, height));

    for (path, color) in point_sets.into_iter().zip(COLORS.iter()) {
        document = draw_path(document, path, color);
    }

    svg::save(filename, &document)?;

    Ok(())
}

pub fn write_points(
    filename: &path::Path,
    point_sets: &Vec<Vec<point::Point>>,
    width: u32,
    height: u32,
) -> Result<()> {
    let mut document = Document::new().set("viewBox", (0, 0, width, height));

    for (points, color) in point_sets.into_iter().zip(COLORS.iter()) {
        document = draw_points(document, points, color);
    }

    svg::save(filename, &document)?;

    Ok(())
}
