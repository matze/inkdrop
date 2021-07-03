#![recursion_limit = "1024"]

pub mod worker;

use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::worker::{Bridge, Bridged};
use yew::ChangeData;

enum ComputeResult {
    Points(Vec<(f64, f64)>),
    Path(Vec<(f64, f64)>),
}

pub struct Model {
    link: ComponentLink<Self>,
    tasks: Vec<ReaderTask>,
    width: u32,
    height: u32,
    num_points: usize,
    result: ComputeResult,
    voronoi_iterations: usize,
    worker: Box<dyn Bridge<worker::Worker>>,
    computing: bool,
    data: Option<FileData>,
    draw_path: bool,
    tsp_iterations: usize,
}

pub enum Msg {
    Open(Vec<File>),
    Opened(FileData),
    UpdateNumPoints(usize),
    UpdateVoronoiIterations(usize),
    UpdateTspIterations(usize),
    ResultComputed(worker::Response),
    UpdateDrawStyle,
}

fn view_point(point: &(f64, f64)) -> Html {
    html! {
        <circle cx=point.0.to_string() cy=point.1.to_string() r="1" fill="black"/>
    }
}

fn view_path(path: &Vec<(f64, f64)>) -> Html {
    let remaining = path
        .iter()
        .skip(1)
        .map(|p| format!("L{},{}", p.0, p.1))
        .collect::<Vec<String>>();

    let data = format!("M{},{} {}", path[0].0, path[0].1, remaining.join(" "));

    html! {
        <path d=data fill="none" stroke="black" stroke-width="1.0"/>
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
            width: 150,
            height: 150,
            num_points: 1000,
            result: ComputeResult::Points(vec![]),
            voronoi_iterations: 0,
            worker,
            computing: false,
            data: None,
            draw_path: false,
            tsp_iterations: 5,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Open(files) => {
                for file in files.into_iter() {
                    let task = {
                        let callback = self.link.callback(Msg::Opened);
                        ReaderService::read_file(file, callback).unwrap()
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
            Msg::UpdateTspIterations(num) => {
                self.tsp_iterations = num;
                self.maybe_compute();
                return true;
            }
            Msg::UpdateDrawStyle => {
                self.draw_path = !self.draw_path;
                self.maybe_compute();
                return true;
            }
            Msg::ResultComputed(response) => {
                match response {
                    worker::Response::Points(data) => {
                        self.width = data.width;
                        self.height = data.height;
                        self.result = ComputeResult::Points(data.points);
                    }
                    worker::Response::Path(data) => {
                        self.width = data.width;
                        self.height = data.height;
                        self.result = ComputeResult::Path(data.points);
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
                    <svg width=self.width.to_string() height=self.height.to_string() viewBox=format!("0 0 {} {}", self.width, self.height) xmlns="http://www.w3.org/2000/svg">
                    {
                        match &self.result {
                            ComputeResult::Points(p) => {
                                p.iter().map(view_point).collect::<Html>()
                            }
                            ComputeResult::Path(p) => {
                                view_path(&p)
                            }
                        }
                    }
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
                        value=self.voronoi_iterations.to_string()
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
                        value=self.num_points.to_string()
                        disabled=self.computing
                        onchange=self.link.callback(move |value| {
                        if let ChangeData::Value(value) = value {
                            return Msg::UpdateNumPoints(value.parse::<usize>().unwrap());
                        }

                        Msg::UpdateNumPoints(1000)
                    })/>
                    <label for="num_points">{ self.num_points }</label>
                </div>

                <div>
                    <input type="radio"
                        id="points"
                        name="draw_style"
                        checked=!self.draw_path
                        disabled=self.computing
                        onchange=self.link.callback(move |_| { Msg::UpdateDrawStyle })
                    />
                    <label for="points">{ "Points" }</label>

                    <input type="radio"
                        id="path"
                        name="draw_style"
                        checked=self.draw_path
                        disabled=self.computing
                        onchange=self.link.callback(move |_| { Msg::UpdateDrawStyle })
                    />
                    <label for="path">{ "Path" }</label>
                </div>

                <div>
                    <input type="range"
                        id="tsp_iterations"
                        min="0"
                        max="20"
                        step="1"
                        value=self.tsp_iterations.to_string()
                        disabled={ self.computing || !self.draw_path }
                        onchange=self.link.callback(move |value| {
                        if let ChangeData::Value(value) = value {
                            return Msg::UpdateTspIterations(value.parse::<usize>().unwrap());
                        }

                        Msg::UpdateTspIterations(0)
                    })/>
                    <label for="tsp_iterations">{ self.tsp_iterations }</label>
                </div>
            </div>
        }
    }
}

impl Model {
    fn maybe_compute(&mut self) {
        if !self.computing {
            if let Some(data) = self.data.as_ref() {
                let data = worker::ComputeData {
                    data: data.content.clone(),
                    num_points: self.num_points,
                    voronoi_iterations: self.voronoi_iterations,
                    compute_path: self.draw_path,
                    tsp_iterations: self.tsp_iterations,
                };

                self.worker.send(worker::Request::Compute(data));
                self.computing = true;
            }
        }
    }
}
