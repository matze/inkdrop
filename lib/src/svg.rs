use anyhow::Result;

use crate::point::Point;
use std::path;
use svg::node::element::path::Data;
use svg::node::element::Circle;
use svg::node::element::Path;
use svg::Document;

const COLORS: [&str; 4] = ["cyan", "magenta", "yellow", "black"];

fn draw_path(document: Document, path: &Vec<Point>, color: &str) -> Document {
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

fn draw_points(document: Document, points: &Vec<Point>, color: &str) -> Document {
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
    point_sets: &Vec<Vec<Point>>,
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
    point_sets: &Vec<Vec<Point>>,
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
