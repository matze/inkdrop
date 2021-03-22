use crate::color::to_black;
use crate::point::Point;
use anyhow::{anyhow, Result};
use image::GenericImageView;
use voronator::delaunator;

fn weighted_centroid(points: Vec<delaunator::Point>, img: &image::DynamicImage) -> Point {
    // Minimum weight to avoid rejecting pure vertices.
    const MIN_WEIGHT: f64 = 0.0000000001;

    let points = points
        .into_iter()
        .map(|p| Point::from(p))
        .collect::<Vec<_>>();

    // Use vertices of the hull as sample points and final weights for the points. However, we
    // should use the entire cell or image as a density function. In some cases points may wander
    // off ...
    let weights = points
        .iter()
        .map(|p| img.get_pixel((p.x - 0.001).floor() as u32, (p.y - 0.001).floor() as u32))
        .map(|c| (to_black(c[0], c[1], c[2]) as f64).clamp(MIN_WEIGHT, 1.0))
        .collect::<Vec<_>>();

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
        .map(|p| delaunator::Point { x: p.x, y: p.y })
        .collect::<Vec<_>>();

    let diagram = voronator::VoronoiDiagram::new(
        &delaunator::Point { x: 0.0, y: 0.0 },
        &delaunator::Point {
            x: width as f64,
            y: height as f64,
        },
        &points,
    )
    .ok_or(anyhow!("Failed to generate Voronoi diagram"))?;

    Ok(diagram
        .cells
        .into_iter()
        .map(|c| weighted_centroid(c, &img))
        .collect::<Vec<_>>())
}
