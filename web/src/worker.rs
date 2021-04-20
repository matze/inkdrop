use anyhow::Result;
use image::io::Reader;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use yew::worker::{Agent, AgentLink, HandlerId, Public};

#[derive(Serialize, Deserialize)]
pub struct ComputeData {
    pub data: Vec<u8>,
    pub num_points: usize,
    pub voronoi_iterations: usize,
    pub compute_path: bool,
    pub tsp_iterations: usize,
}

#[derive(Serialize, Deserialize)]
pub enum Request {
    Compute(ComputeData),
}

#[derive(Serialize, Deserialize)]
pub struct UpdateData {
    pub width: u32,
    pub height: u32,
    pub points: Vec<(f64, f64)>,
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Points(UpdateData),
    Path(UpdateData),
    Done,
}

pub enum Msg {}

pub struct Worker {
    link: AgentLink<Worker>,
}

impl UpdateData {
    fn from(width: u32, height: u32, point_sets: &Vec<Vec<inkdrop::point::Point>>) -> Self {
        Self {
            width,
            height,
            points: point_sets.iter().flatten().map(|p| (p.x, p.y)).collect(),
        }
    }
}

impl Agent for Worker {
    type Reach = Public<Self>;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        Self { link }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {}
    }

    fn handle_input(&mut self, msg: Self::Input, who: HandlerId) {
        match msg {
            Request::Compute(data) => {
                let image = Reader::new(Cursor::new(data.data))
                    .with_guessed_format()
                    .unwrap()
                    .decode()
                    .unwrap();

                let (width, height) = image.dimensions();

                let mut point_sets = inkdrop::sample_points(&image, data.num_points, 1.0, false);

                self.link.respond(
                    who,
                    Response::Points(UpdateData::from(width, height, &point_sets)),
                );

                for _ in 0..data.voronoi_iterations {
                    point_sets = point_sets
                        .into_iter()
                        .map(|ps| inkdrop::voronoi::move_points(ps, &image))
                        .collect::<Result<Vec<_>>>()
                        .unwrap();

                    self.link.respond(
                        who,
                        Response::Points(UpdateData::from(width, height, &point_sets)),
                    );
                }

                if data.compute_path {
                    point_sets = point_sets
                        .into_iter()
                        .map(|p| inkdrop::tsp::make_nn_tour(p))
                        .collect();

                    self.link.respond(
                        who,
                        Response::Path(UpdateData::from(width, height, &point_sets)),
                    );

                    for _ in 0..data.tsp_iterations {
                        let (optimized, _): (Vec<_>, Vec<_>) = point_sets
                            .into_iter()
                            .map(|ps| inkdrop::tsp::optimize_two_opt_tour(ps))
                            .unzip();

                        point_sets = optimized;

                        self.link.respond(
                            who,
                            Response::Path(UpdateData::from(width, height, &point_sets)),
                        );
                    }
                }

                self.link.respond(who, Response::Done);
            }
        }
    }

    fn name_of_resource() -> &'static str {
        "worker.js"
    }
}
