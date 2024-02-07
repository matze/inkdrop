use anyhow::Result;
use clap::Parser;
use image::io::Reader;
use image::GenericImageView;
use inkdrop::{tsp, voronoi, Point};
use log::info;
use rayon::prelude::*;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Options {
    #[arg(long, short)]
    input: PathBuf,

    #[arg(long, short)]
    svg: Option<PathBuf>,

    #[arg(long, short)]
    json: Option<PathBuf>,

    #[arg(long, short, default_value = "20000")]
    num_points: usize,

    #[arg(long)]
    draw_points: bool,

    #[arg(long, default_value = "0")]
    voronoi_iterations: usize,

    #[arg(long, default_value = "0")]
    tsp_improvement: f64,

    #[arg(long, default_value = "1")]
    gamma: f32,

    #[arg(long)]
    cmyk: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let opt = Options::parse();
    let img = Reader::open(&opt.input)?.decode()?;
    let (width, height) = img.dimensions();

    info!("Sample points");
    let mut point_sets = inkdrop::sample_points(&img, opt.num_points, opt.gamma, opt.cmyk);

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
        if let Some(path) = opt.svg {
            inkdrop::svg::write_points(&path, &point_sets, width, height)?;
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

        if let Some(path) = opt.svg {
            inkdrop::svg::write_path(&path, &tours, width, height)?;
        }
        if let Some(path) = opt.json {
            // serialize channels
            let fh = std::fs::File::create(path)?;
            serde_json::to_writer_pretty(&fh, &tours)?;
        }
    }

    Ok(())
}
