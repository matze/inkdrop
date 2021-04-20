#![recursion_limit = "512"]

pub mod worker;

use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::worker::{Bridge, Bridged};
use yew::ChangeData;

pub struct Model {
    link: ComponentLink<Self>,
    tasks: Vec<ReaderTask>,
    reader: ReaderService,
    width: u32,
    height: u32,
    num_points: usize,
    points: Vec<(f64, f64)>,
    voronoi_iterations: usize,
    worker: Box<dyn Bridge<worker::Worker>>,
    computing: bool,
    data: Option<FileData>,
}

pub enum Msg {
    Open(Vec<File>),
    Opened(FileData),
    UpdateNumPoints(usize),
    UpdateVoronoiIterations(usize),
    ResultComputed(worker::Response),
}

fn view_point(point: &(f64, f64)) -> Html {
    html! {
        <circle cx=point.0 cy=point.1 r="1" fill="black"/>
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let callback = link.callback(|r| Msg::ResultComputed(r));
        let worker = worker::Worker::bridge(callback);

        Self {
            link,
            tasks: vec![],
            reader: ReaderService::new(),
            width: 150,
            height: 150,
            num_points: 1000,
            points: vec![],
            voronoi_iterations: 0,
            worker,
            computing: false,
            data: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Open(files) => {
                for file in files.into_iter() {
                    let task = {
                        let callback = self.link.callback(Msg::Opened);
                        self.reader.read_file(file, callback).unwrap()
                    };

                    self.tasks.push(task);
                }

                return false;
            }
            Msg::Opened(data) => {
                self.data = Some(data);
                self.maybe_compute();
                return true;
            }
            Msg::UpdateNumPoints(num) => {
                self.num_points = num;
                self.maybe_compute();
                return true;
            }
            Msg::UpdateVoronoiIterations(num) => {
                self.voronoi_iterations = num;
                self.maybe_compute();
                return true;
            }
            Msg::ResultComputed(response) => {
                match response {
                    worker::Response::Update(data) => {
                        self.width = data.width;
                        self.height = data.height;
                        self.points = data.points;
                    }
                    worker::Response::Done => {
                        self.computing = false;
                    }
                }
                return true;
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <div>
                    <svg width=self.width height=self.height viewBox=format!("0 0 {} {}", self.width, self.height) xmlns="http://www.w3.org/2000/svg">
                        { self.points.iter().map(view_point).collect::<Html>() }
                    </svg>
                </div>
                <input type="file" onchange=self.link.callback(move |value| {
                    let mut result = Vec::new();

                    if let ChangeData::Files(files) = value {
                        let files = js_sys::try_iter(&files)
                            .unwrap()
                            .unwrap()
                            .map(|v| File::from(v.unwrap()));

                        result.extend(files);
                    }

                    Msg::Open(result)
                })/>

                <div>
                    <input type="range"
                        id="voronoi_iterations"
                        min="0"
                        max="100"
                        step="1"
                        value=self.voronoi_iterations
                        disabled=self.computing
                        onchange=self.link.callback(move |value| {
                        if let ChangeData::Value(value) = value {
                            return Msg::UpdateVoronoiIterations(value.parse::<usize>().unwrap());
                        }

                        Msg::UpdateVoronoiIterations(0)
                    })/>
                    <label for="voronoi_iterations">{ self.voronoi_iterations }</label>
                </div>

                <div>
                    <input type="range"
                        id="num_points"
                        min="1000"
                        max="100000"
                        step="1"
                        value=self.num_points
                        disabled=self.computing
                        onchange=self.link.callback(move |value| {
                        if let ChangeData::Value(value) = value {
                            return Msg::UpdateNumPoints(value.parse::<usize>().unwrap());
                        }

                        Msg::UpdateNumPoints(1000)
                    })/>
                    <label for="num_points">{ self.num_points }</label>
                </div>
            </div>
        }
    }
}

impl Model {
    fn maybe_compute(&mut self) {
        if let Some(data) = self.data.as_ref() {
            let data = worker::ComputeData {
                data: data.content.clone(),
                num_points: self.num_points,
                voronoi_iterations: self.voronoi_iterations,
            };

            if !self.computing {
                self.worker.send(worker::Request::Compute(data));
                self.computing = true;
            }
        }
    }
}
