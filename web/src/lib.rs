#![recursion_limit = "256"]

use anyhow::Result;
use image::io::Reader;
use image::GenericImageView;
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::ChangeData;

struct Model {
    link: ComponentLink<Self>,
    tasks: Vec<ReaderTask>,
    reader: ReaderService,
    width: u32,
    height: u32,
    point_sets: Vec<inkdrop::point::Point>,
}

enum Msg {
    Open(Vec<File>),
    Opened(FileData),
}

fn view_point(point: &inkdrop::point::Point) -> Html {
    html! {
        <circle cx=point.x cy=point.y r="1" fill="black"/>
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            tasks: vec![],
            reader: ReaderService::new(),
            width: 150,
            height: 150,
            point_sets: vec![],
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
                let image = Reader::new(Cursor::new(data.content))
                    .with_guessed_format()
                    .unwrap()
                    .decode()
                    .unwrap();

                let (width, height) = image.dimensions();

                self.width = width;
                self.height = height;

                let mut point_sets = inkdrop::sample_points(&image, 20000, 1.0, false);

                for _ in 0..10 {
                    point_sets = point_sets
                        .into_iter()
                        .map(|ps| inkdrop::voronoi::move_points(ps, &image))
                        .collect::<Result<Vec<_>>>()
                        .unwrap();
                }

                // for now just handle all of colors as black
                self.point_sets = point_sets.into_iter().flatten().collect();

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
                        { self.point_sets.iter().map(view_point).collect::<Html>() }
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
                })
                />
            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}
