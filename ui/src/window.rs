use crate::application::Application;
use crate::config::{APP_ID, PROFILE};
use anyhow::Result;
use glib::clone;
use glib::signal::Inhibit;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use image::io::Reader;
use image::GenericImageView;
use inkdrop::point::Point;
use log::warn;
use std::path::PathBuf;
use std::thread;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/net/bloerg/inkdrop/window.ui")]
    pub struct ApplicationWindow {
        #[template_child]
        pub filename: TemplateChild<gtk::Label>,
        #[template_child]
        pub drawing_area: TemplateChild<gtk::DrawingArea>,
        #[template_child]
        pub num_points: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub num_voronoi_iterations: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub button_cmyk: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub button_path: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub tsp_opt: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
        pub save_dialog: gtk::FileChooserNative,
        pub open_dialog: gtk::FileChooserNative,
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationWindow {
        const NAME: &'static str = "ApplicationWindow";
        type Type = super::ApplicationWindow;
        type ParentType = gtk::ApplicationWindow;

        fn new() -> Self {
            let open_dialog = gtk::FileChooserNativeBuilder::new()
                .title(&"Open image")
                .modal(true)
                .action(gtk::FileChooserAction::Open)
                .accept_label(&"_Open")
                .cancel_label(&"_Cancel")
                .build();

            let save_dialog = gtk::FileChooserNativeBuilder::new()
                .title(&"Save SVG")
                .modal(true)
                .action(gtk::FileChooserAction::Save)
                .accept_label(&"_Save")
                .cancel_label(&"_Cancel")
                .build();

            Self {
                filename: TemplateChild::default(),
                drawing_area: TemplateChild::default(),
                num_points: TemplateChild::default(),
                num_voronoi_iterations: TemplateChild::default(),
                button_cmyk: TemplateChild::default(),
                button_path: TemplateChild::default(),
                tsp_opt: TemplateChild::default(),
                save_button: TemplateChild::default(),
                save_dialog: save_dialog,
                open_dialog: open_dialog,
                settings: gio::Settings::new(APP_ID),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("win.open", None, move |win, _, _| {
                let dialog = &imp::ApplicationWindow::from_instance(&win).open_dialog;

                dialog.connect_response(clone!(@weak win => move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        let path = dialog.get_file().unwrap().get_path().unwrap();
                        let filename = &imp::ApplicationWindow::from_instance(&win).filename;
                        filename.set_text(&path.to_string_lossy());
                    }
                }));

                dialog.set_transient_for(Some(win));
                dialog.show();
            });
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ApplicationWindow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let builder = gtk::Builder::from_resource("/net/bloerg/inkdrop/shortcuts.ui");
            let shortcuts = builder.get_object("shortcuts").unwrap();
            obj.set_help_overlay(Some(&shortcuts));

            // Devel Profile
            if PROFILE == "Devel" {
                obj.get_style_context().add_class("devel");
            }

            // load latest window state
            obj.load_window_size();
        }
    }

    impl WidgetImpl for ApplicationWindow {}
    impl WindowImpl for ApplicationWindow {
        // save window state on delete event
        fn close_request(&self, obj: &Self::Type) -> Inhibit {
            if let Err(err) = obj.save_window_size() {
                warn!("Failed to save window state, {}", &err);
            }
            Inhibit(false)
        }
    }

    impl ApplicationWindowImpl for ApplicationWindow {}
}

glib::wrapper! {
    pub struct ApplicationWindow(ObjectSubclass<imp::ApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

struct ComputeParameters {
    filename: String,
    num_points: usize,
    num_iterations: usize,
    tsp_opt: f64,
    cmyk: bool,
}

enum ComputeRequest {
    Points(ComputeParameters),
    Path(ComputeParameters),
}

impl ComputeRequest {
    fn from_window(window: &imp::ApplicationWindow) -> Option<Self> {
        let filename = window.filename.get_text();

        if filename == "" {
            return None;
        }

        let parameters = ComputeParameters {
            filename: filename.to_string(),
            num_points: window.num_points.get_value() as usize,
            num_iterations: window.num_voronoi_iterations.get_value() as usize,
            tsp_opt: window.tsp_opt.get_value(),
            cmyk: window.button_cmyk.get_active(),
        };

        if window.button_path.get_active() {
            return Some(Self::Path(parameters));
        }

        Some(Self::Points(parameters))
    }
}

#[derive(Clone)]
struct DrawData {
    width: u32,
    height: u32,
    point_sets: Vec<Vec<Point>>,
}

impl DrawData {
    fn new(width: u32, height: u32, point_sets: Vec<Vec<Point>>) -> Self {
        Self {
            width,
            height,
            point_sets,
        }
    }
}

#[derive(Clone)]
enum ComputeResult {
    Points(DrawData),
    Path(DrawData),
}

enum Message {
    DrawPoints(DrawData),
    DrawPath(DrawData),
    ScheduleComputeRequest,
    ComputeFinished(ComputeResult),
    SaveResult,
}

const CMYK_AS_RGB: [(f64, f64, f64); 4] = [
    (0.0, 1.0, 1.0),
    (1.0, 0.0, 1.0),
    (1.0, 1.0, 0.0),
    (0.0, 0.0, 0.0),
];

fn compute_point_distribution(
    sender: &glib::Sender<Message>,
    parameters: &ComputeParameters,
) -> (u32, u32, Vec<Vec<Point>>) {
    let sender = sender.clone();
    let path = PathBuf::from(&parameters.filename);
    let img = Reader::open(path).unwrap().decode().unwrap();
    let (w, h) = img.dimensions();
    let mut pss = inkdrop::sample_points(&img, parameters.num_points, 1.0, parameters.cmyk);

    for _ in 0..parameters.num_iterations {
        pss = pss
            .into_iter()
            .map(|ps| inkdrop::voronoi::move_points(ps, &img))
            .collect::<Result<Vec<_>>>()
            .unwrap();

        sender
            .send(Message::DrawPoints(DrawData::new(w, h, pss.clone())))
            .unwrap();
    }

    sender
        .send(Message::DrawPoints(DrawData::new(w, h, pss.clone())))
        .unwrap();

    (w, h, pss)
}

fn compute_points_request(sender: glib::Sender<Message>, parameters: ComputeParameters) {
    let (w, h, pss) = compute_point_distribution(&sender, &parameters);
    let result = ComputeResult::Points(DrawData::new(w, h, pss));
    sender.send(Message::ComputeFinished(result)).unwrap();
}

fn compute_path_request(sender: glib::Sender<Message>, parameters: ComputeParameters) {
    let (w, h, mut pss) = compute_point_distribution(&sender, &parameters);

    pss = pss
        .into_iter()
        .map(|points| inkdrop::tsp::make_nn_tour(points))
        .collect();

    sender
        .send(Message::DrawPath(DrawData::new(w, h, pss.clone())))
        .unwrap();

    if parameters.tsp_opt != 0.0 {
        loop {
            let (new_pps, improvements): (Vec<_>, Vec<_>) = pss
                .into_iter()
                .map(|ps| inkdrop::tsp::optimize_two_opt_tour(ps))
                .unzip();

            pss = new_pps;
            sender
                .send(Message::DrawPath(DrawData::new(w, h, pss.clone())))
                .unwrap();

            if improvements.iter().all(|&i| i < parameters.tsp_opt) {
                break;
            }
        }

        sender
            .send(Message::DrawPath(DrawData::new(w, h, pss.clone())))
            .unwrap();
    }

    let result = ComputeResult::Path(DrawData::new(w, h, pss));
    sender.send(Message::ComputeFinished(result)).unwrap();
}

impl ApplicationWindow {
    pub fn new(app: &Application) -> Self {
        let window: Self =
            glib::Object::new(&[]).expect("Failed to create ApplicationWindow");

        window.set_application(Some(app));

        gtk::Window::set_default_icon_name(APP_ID);

        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let compute_sender = sender.clone();

        let mut compute_ongoing = false;
        let mut compute_result: Option<ComputeResult> = None;

        receiver.attach(
            None,
            clone!(@strong window => move |message| {
                match message {
                    Message::DrawPoints(request) => {
                        window.draw_points(request);
                    },
                    Message::DrawPath(request) => {
                        window.draw_path(request);
                    },
                    Message::ScheduleComputeRequest => {
                        if compute_ongoing {
                            return glib::Continue(true);
                        }

                        let request = ComputeRequest::from_window(&imp::ApplicationWindow::from_instance(&window));
                        let sender = compute_sender.clone();
                        compute_ongoing = request.is_some();

                        request.map(move |request| {
                            thread::spawn(move || {
                                match request {
                                    ComputeRequest::Points(p) => { compute_points_request(sender, p); },
                                    ComputeRequest::Path(p) => { compute_path_request(sender, p); },
                                }
                            });
                        });
                    },
                    Message::ComputeFinished(result) => {
                        compute_ongoing = false;
                        compute_result = Some(result);
                    },
                    Message::SaveResult => {
                        if let Some(result) = &compute_result {
                            let dialog = &imp::ApplicationWindow::from_instance(&window).save_dialog;

                            let result = result.clone();

                            dialog.connect_response(clone!(@weak window => move |dialog, response| {
                                if response == gtk::ResponseType::Accept {
                                    let path = dialog.get_file().unwrap().get_path().unwrap();

                                    match &result {
                                        ComputeResult::Points(p) => {
                                            inkdrop::write_points(&path, &p.point_sets, p.width, p.height).unwrap();
                                        },
                                        ComputeResult::Path(p) => {
                                            inkdrop::write_path(&path, &p.point_sets, p.width, p.height).unwrap();
                                        },
                                    };
                                }
                            }));

                            dialog.set_transient_for(Some(&window));
                            dialog.show();
                        }
                    },
                }

                glib::Continue(true)
            }),
        );

        let filename = &imp::ApplicationWindow::from_instance(&window).filename;

        filename.connect_property_label_notify(clone!(@weak window, @strong sender => move |_| {
            sender.clone().send(Message::ScheduleComputeRequest).unwrap();
        }));

        let num_points = &imp::ApplicationWindow::from_instance(&window).num_points;

        num_points.connect_value_changed(clone!(@weak window, @strong sender => move |_| {
            sender.clone().send(Message::ScheduleComputeRequest).unwrap();
        }));

        let num_voronoi_iterations =
            &imp::ApplicationWindow::from_instance(&window).num_voronoi_iterations;

        num_voronoi_iterations.connect_value_changed(
            clone!(@weak window, @strong sender => move |_| {
                sender.clone().send(Message::ScheduleComputeRequest).unwrap();
            }),
        );

        let save_button = &imp::ApplicationWindow::from_instance(&window).save_button;

        save_button.connect_clicked(clone!(@weak window => move |_| {
            sender.clone().send(Message::SaveResult).unwrap();
        }));

        window
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = &imp::ApplicationWindow::from_instance(self).settings;

        let size = self.get_default_size();

        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = &imp::ApplicationWindow::from_instance(self).settings;

        let width = settings.get_int("window-width");
        let height = settings.get_int("window-height");
        let is_maximized = settings.get_boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn draw_points(&self, data: DrawData) {
        let area = &imp::ApplicationWindow::from_instance(self).drawing_area;
        area.set_content_width(data.width as i32);
        area.set_content_height(data.height as i32);

        area.set_draw_func(move |_, cr, width, height| {
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            cr.fill();

            for (points, color) in data.point_sets.iter().zip(CMYK_AS_RGB.iter()) {
                if points.len() < 1 {
                    continue;
                }

                cr.set_source_rgba(color.0, color.1, color.2, 1.0);

                for point in points {
                    cr.arc(point.x, point.y, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill();
                }
            }
        });
    }

    fn draw_path(&self, data: DrawData) {
        let area = &imp::ApplicationWindow::from_instance(self).drawing_area;
        area.set_content_width(data.width as i32);
        area.set_content_height(data.height as i32);

        area.set_draw_func(move |_, cr, width, height| {
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            cr.fill();

            for (points, color) in data.point_sets.iter().zip(CMYK_AS_RGB.iter()) {
                if points.len() < 2 {
                    continue;
                }

                cr.set_source_rgba(color.0, color.1, color.2, 1.0);
                cr.move_to(points[0].x, points[0].y);

                for point in points.iter().skip(1) {
                    cr.line_to(point.x, point.y);
                }

                cr.stroke();
            }
        });
    }
}
