use crate::color::to_black;
use crate::Point;
use anyhow::{anyhow, Result};
use image::GenericImageView;
use voronator::delaunator;

fn weighted_centroid(points: &[delaunator::Point], img: &image::DynamicImage) -> Point {
    // Minimum weight to avoid rejecting pure vertices.
    const MIN_WEIGHT: f64 = 0.0000000001;

    let (width, height) = img.dimensions();

    let points = points.iter().map(Point::from).collect::<Vec<_>>();

    let sample_point = |p: &Point| -> f64 {
        if p.x as u32 == width - 1 || p.y as u32 == height - 1 {
            return 0.0;
        }

        let x = (p.x - 0.5).floor() as u32;
        let y = (p.y - 0.5).floor() as u32;
        let dx = if x == width - 1 { 0 } else { 1 };
        let dy = if y == height - 1 { 0 } else { 1 };

        let c1 = img.get_pixel(x, y);
        let c2 = img.get_pixel(x + dx, y);
        let c3 = img.get_pixel(x + dx, y + dy);
        let c4 = img.get_pixel(x, y + dy);

        let b1 = to_black(c1[0], c1[1], c1[2]) as f64;
        let b2 = to_black(c2[0], c2[1], c2[2]) as f64;
        let b3 = to_black(c3[0], c3[1], c3[2]) as f64;
        let b4 = to_black(c4[0], c4[1], c4[2]) as f64;

        (b1 + b2 + b3 + b4) / 4.0
    };

    // Use vertices of the hull as sample points and final weights for the points. However, we
    // should use the entire cell or image as a density function. In some cases points may wander
    // off ...
    let weights = points.iter().map(sample_point).collect::<Vec<_>>();

    let center = points.iter().fold(Point::origin(), |acc, p| acc + *p);

    let len = weights.len() as f64;
    let cx = center.x / len;
    let cy = center.y / len;
    let c = img.get_pixel((cx - 0.5) as u32, (cy - 0.5) as u32);
    let center_weight = (to_black(c[0], c[1], c[2]) as f64).clamp(MIN_WEIGHT, 1.0);
    let sum = weights.iter().sum::<f64>() + center_weight;

    let mut result = Point::new(cx * center_weight / sum, cy * center_weight / sum);

    for (point, weight) in points.into_iter().zip(weights) {
        result += point * weight / sum;
    }

    result
}

pub fn move_points(points: Vec<Point>, img: &image::DynamicImage) -> Result<Vec<Point>> {
    if points.len() < 3 {
        return Ok(points);
    }

    let (width, height) = img.dimensions();

    let points = points
        .iter()
        .filter_map(|p| {
            (!p.x.is_nan() && !p.y.is_nan()).then_some(delaunator::Point { x: p.x, y: p.y })
        })
        .collect::<Vec<_>>();

    let diagram = voronator::VoronoiDiagram::new(
        &delaunator::Point { x: 0.0, y: 0.0 },
        &delaunator::Point {
            x: width as f64,
            y: height as f64,
        },
        &points,
    )
    .ok_or_else(|| anyhow!("Failed to generate Voronoi diagram"))?;

    Ok(diagram
        .cells()
        .iter()
        .map(|c| weighted_centroid(c.points(), img))
        .collect::<Vec<_>>())
}
