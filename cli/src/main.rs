use anyhow::Result;
use image::io::Reader;
use image::GenericImageView;
use inkdrop::color::{invert, to_black, to_cmyk};
use inkdrop::point::Point;
use inkdrop::tsp;
use inkdrop::voronoi;
use log::info;
use rand::Rng;
use rayon::prelude::*;
use std::path::PathBuf;
use structopt::StructOpt;
use svg::node::element::path::Data;
use svg::node::element::Circle;
use svg::node::element::Path;
use svg::Document;

#[derive(StructOpt)]
pub struct Options {
    #[structopt(long, short, parse(from_os_str))]
    input: PathBuf,

    #[structopt(long, short, parse(from_os_str))]
    output: PathBuf,

    #[structopt(long, short, default_value = "20000")]
    num_points: usize,

    #[structopt(long)]
    draw_points: bool,

    #[structopt(long, default_value = "0")]
    voronoi_iterations: usize,

    #[structopt(long, default_value = "0")]
    tsp_improvement: f64,

    #[structopt(long, default_value = "1")]
    gamma: f32,

    #[structopt(long)]
    cmyk: bool,
}

fn sample_points(img: &image::DynamicImage, opt: &Options) -> Vec<Vec<Point>> {
    let (width, height) = img.dimensions();
    let mut rng = rand::thread_rng();

    // Store points for each channel
    let mut ps = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    while ps.iter().map(|points| points.len()).sum::<usize>() < opt.num_points {
        let x = rng.gen::<f64>() * width as f64;
        let y = rng.gen::<f64>() * height as f64;
        let channels = img.get_pixel(x as u32, y as u32);
        let sample: f32 = rng.gen();

        if opt.cmyk {
            let cmyk = invert(to_cmyk(channels[0], channels[1], channels[2]));

            for (points, color) in ps.iter_mut().zip(cmyk.iter()) {
                if sample >= color.powf(opt.gamma) {
                    points.push(Point { x, y });
                }
            }
        } else {
            let black = 1.0 - to_black(channels[0], channels[1], channels[2]);

            if sample >= black.powf(opt.gamma) {
                ps[3].push(Point { x, y });
            }
        }
    }

    ps
}

fn draw_path(document: Document, tour: Vec<Point>, color: &str) -> Document {
    if tour.is_empty() {
        return document;
    }

    let mut data = Data::new().move_to((tour[0].x, tour[0].y));

    for point in tour.into_iter().skip(1) {
        data = data.line_to((point.x, point.y));
    }

    let path = Path::new()
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", "1.0")
        .set("d", data);

    document.add(path)
}

fn draw_points(document: Document, points: Vec<Point>, color: &str) -> Document {
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

fn main() -> Result<()> {
    env_logger::init();

    let opt = Options::from_args();
    let img = Reader::open(&opt.input)?.decode()?;
    let (width, height) = img.dimensions();
    let colors = ["cyan", "magenta", "yellow", "black"];

    let mut document = Document::new().set("viewBox", (0, 0, width, height));

    info!("Sample points");
    let mut point_sets: Vec<Vec<Point>> = sample_points(&img, &opt);

    if opt.voronoi_iterations > 0 {
        info!("Move points");

        for _ in 0..opt.voronoi_iterations {
            point_sets = point_sets
                .into_iter()
                .map(|ps| voronoi::move_points(ps, &img))
                .collect::<Result<Vec<_>>>()?;
        }
    }

    if opt.draw_points {
        for (points, color) in point_sets.into_iter().zip(colors.iter()) {
            document = draw_points(document, points, color);
        }
    } else {
        info!("Make NN tours");
        let tours: Vec<Vec<Point>> = point_sets
            .into_par_iter()
            .map(|points| {
                if opt.tsp_improvement != 0.0 {
                    tsp::optimize(tsp::make_nn_tour(points), opt.tsp_improvement)
                } else {
                    tsp::make_nn_tour(points)
                }
            })
            .collect();

        for (tour, color) in tours.into_iter().zip(colors.iter()) {
            document = draw_path(document, tour, color);
        }
    }

    svg::save(&opt.output, &document)?;

    Ok(())
}
