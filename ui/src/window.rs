use crate::application::Application;
use crate::config::{APP_ID, PROFILE};
use anyhow::Result;
use adw::subclass::prelude::*;
use glib::clone;
use glib::signal::Inhibit;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use image::io::Reader;
use image::GenericImageView;
use inkdrop::point::Point;
use log::warn;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
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
        pub log_progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub info_bar: TemplateChild<gtk::InfoBar>,
        #[template_child]
        pub info_label: TemplateChild<gtk::Label>,
        pub save_dialog: gtk::FileChooserNative,
        pub open_dialog: gtk::FileChooserNative,
        pub gesture_drag: gtk::GestureDrag,
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationWindow {
        const NAME: &'static str = "ApplicationWindow";
        type Type = super::ApplicationWindow;
        type ParentType = adw::ApplicationWindow;

        fn new() -> Self {
            let open_dialog = gtk::builders::FileChooserNativeBuilder::new()
                .title(&"Open image")
                .modal(true)
                .action(gtk::FileChooserAction::Open)
                .accept_label(&"_Open")
                .cancel_label(&"_Cancel")
                .build();

            let save_dialog = gtk::builders::FileChooserNativeBuilder::new()
                .title(&"Save SVG")
                .modal(true)
                .action(gtk::FileChooserAction::Save)
                .accept_label(&"_Save")
                .cancel_label(&"_Cancel")
                .build();

            let gesture_drag = gtk::builders::GestureDragBuilder::new()
                .propagation_phase(gtk::PropagationPhase::Bubble)
                .build();

            Self {
                filename: TemplateChild::default(),
                drawing_area: TemplateChild::default(),
                num_points: TemplateChild::default(),
                num_voronoi_iterations: TemplateChild::default(),
                button_cmyk: TemplateChild::default(),
                button_path: TemplateChild::default(),
                tsp_opt: TemplateChild::default(),
                log_progress_bar: TemplateChild::default(),
                info_bar: TemplateChild::default(),
                info_label: TemplateChild::default(),
                save_button: TemplateChild::default(),
                save_dialog,
                open_dialog,
                gesture_drag,
                settings: gio::Settings::new(APP_ID),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("win.open", None, move |win, _, _| {
                let dialog = &imp::ApplicationWindow::from_instance(&win).open_dialog;

                dialog.connect_response(clone!(@weak win => move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        let path = dialog.file().unwrap().path().unwrap();
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
            let shortcuts = builder.object("shortcuts").unwrap();
            obj.set_help_overlay(Some(&shortcuts));

            // Devel Profile
            if PROFILE == "Devel" {
                obj.style_context().add_class("devel");
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
    impl AdwApplicationWindowImpl for ApplicationWindow {}
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
        let filename = window.filename.text();

        if filename == "" {
            return None;
        }

        let parameters = ComputeParameters {
            filename: filename.to_string(),
            num_points: window.num_points.value() as usize,
            num_iterations: window.num_voronoi_iterations.value() as usize,
            tsp_opt: window.tsp_opt.value(),
            cmyk: window.button_cmyk.is_active(),
        };

        if window.button_path.is_active() {
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
    UpdateProgress(String, f64),
    ComputeFinished(ComputeResult),
    SaveResult,
    ShowError(String),
    HideInfoBar,
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
    progress_fraction: f64,
) -> Result<(u32, u32, Vec<Vec<Point>>)> {
    let sender = sender.clone();
    let path = PathBuf::from(&parameters.filename);
    let img = Reader::open(path).unwrap().decode()?;
    let (w, h) = img.dimensions();

    sender.send(Message::UpdateProgress("Sample points".to_string(), 0.0))?;

    let mut pss = inkdrop::sample_points(&img, parameters.num_points, 1.0, parameters.cmyk);

    for i in 0..parameters.num_iterations {
        sender.send(Message::UpdateProgress(
            format!("Voronoi iteration {}/{}", i + 1, parameters.num_iterations,),
            progress_fraction * (i + 1) as f64 / parameters.num_iterations as f64,
        ))?;

        pss = pss
            .into_iter()
            .map(|ps| inkdrop::voronoi::move_points(ps, &img))
            .collect::<Result<Vec<_>>>()?;

        sender.send(Message::DrawPoints(DrawData::new(w, h, pss.clone())))?;
    }

    sender.send(Message::DrawPoints(DrawData::new(w, h, pss.clone())))?;

    Ok((w, h, pss))
}

fn compute_points_request(
    sender: glib::Sender<Message>,
    parameters: ComputeParameters,
) -> Result<()> {
    let (w, h, pss) = compute_point_distribution(&sender, &parameters, 1.0)?;
    let result = ComputeResult::Points(DrawData::new(w, h, pss));
    sender.send(Message::ComputeFinished(result))?;
    Ok(())
}

fn compute_path_request(
    sender: glib::Sender<Message>,
    parameters: ComputeParameters,
) -> Result<()> {
    let (w, h, mut pss) = compute_point_distribution(&sender, &parameters, 0.3)?;

    pss = pss
        .into_iter()
        .map(|points| inkdrop::tsp::make_nn_tour(points))
        .collect();

    sender.send(Message::DrawPath(DrawData::new(w, h, pss.clone())))?;
    sender.send(Message::UpdateProgress("Improve path".to_string(), 0.5))?;

    if parameters.tsp_opt != 0.0 {
        loop {
            let (new_pps, improvements): (Vec<_>, Vec<_>) = pss
                .into_iter()
                .map(|ps| inkdrop::tsp::optimize_two_opt_tour(ps))
                .unzip();

            pss = new_pps;
            sender.send(Message::DrawPath(DrawData::new(w, h, pss.clone())))?;

            if improvements.iter().all(|&i| i < parameters.tsp_opt) {
                break;
            }

            let progress = 1.0 - (improvements.iter().sum::<f64>() / improvements.len() as f64);

            sender.send(Message::UpdateProgress(
                "Improve path".to_string(),
                progress,
            ))?;
        }

        sender.send(Message::DrawPath(DrawData::new(w, h, pss.clone())))?;
    }

    let result = ComputeResult::Path(DrawData::new(w, h, pss));
    sender.send(Message::ComputeFinished(result))?;

    Ok(())
}

impl ApplicationWindow {
    pub fn new(app: &Application) -> Self {
        let window: Self = glib::Object::new(&[]).expect("Failed to create ApplicationWindow");

        window.set_application(Some(app));

        gtk::Window::set_default_icon_name(APP_ID);

        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let message_sender = sender.clone();

        let mut compute_ongoing = false;
        let mut compute_result: Option<ComputeResult> = None;

        receiver.attach(
            None,
            clone!(@strong window => move |message| {
                let imp = &imp::ApplicationWindow::from_instance(&window);

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

                        let request = ComputeRequest::from_window(imp);
                        let sender = message_sender.clone();
                        compute_ongoing = request.is_some();

                        request.map(move |request| {
                            thread::spawn(move || {
                                let error_sender = sender.clone();

                                let result = match request {
                                    ComputeRequest::Points(p) => { compute_points_request(sender, p) },
                                    ComputeRequest::Path(p) => { compute_path_request(sender, p) },
                                };

                                if let Err(err) = result {
                                    error_sender.send(Message::ShowError(err.to_string())).unwrap();
                                }
                            });
                        });
                    },
                    Message::ComputeFinished(result) => {
                        imp.log_progress_bar.set_visible(false);
                        compute_ongoing = false;
                        compute_result = Some(result);
                    },
                    Message::SaveResult => {
                        if let Some(result) = &compute_result {
                            let dialog = &imp.save_dialog;
                            let result = result.clone();

                            dialog.connect_response(clone!(@weak window => move |dialog, response| {
                                if response == gtk::ResponseType::Accept {
                                    let path = dialog.file().unwrap().path().unwrap();

                                    match &result {
                                        ComputeResult::Points(p) => {
                                            inkdrop::svg::write_points(&path, &p.point_sets, p.width, p.height).unwrap();
                                        },
                                        ComputeResult::Path(p) => {
                                            inkdrop::svg::write_path(&path, &p.point_sets, p.width, p.height).unwrap();
                                        },
                                    };
                                }
                            }));

                            dialog.set_transient_for(Some(&window));
                            dialog.show();
                        }
                    },
                    Message::UpdateProgress(message, fraction) => {
                        imp.log_progress_bar.set_visible(true);
                        imp.log_progress_bar.set_fraction(fraction);
                        imp.log_progress_bar.set_text(Some(&message));
                    },
                    Message::ShowError(message) => {
                        imp.info_label.set_text(&message);
                        imp.info_bar.set_revealed(true);
                        let sender = message_sender.clone();

                        glib::source::timeout_add_seconds(2, move || {
                            sender.send(Message::HideInfoBar).unwrap();
                            glib::Continue(false)
                        });
                    },
                    Message::HideInfoBar => {
                        imp.info_bar.set_revealed(false);
                    },
                }

                glib::Continue(true)
            }),
        );

        let imp = &imp::ApplicationWindow::from_instance(&window);

        imp.filename.connect_label_notify(
            clone!(@weak window, @strong sender => move |_| {
                sender.clone().send(Message::ScheduleComputeRequest).unwrap();
            }),
        );

        imp.num_points
            .connect_value_changed(clone!(@weak window, @strong sender => move |_| {
                sender.clone().send(Message::ScheduleComputeRequest).unwrap();
            }));

        imp.num_voronoi_iterations.connect_value_changed(
            clone!(@weak window, @strong sender => move |_| {
                sender.clone().send(Message::ScheduleComputeRequest).unwrap();
            }),
        );

        imp.tsp_opt
            .connect_value_changed(clone!(@weak window, @strong sender => move |_| {
                sender.clone().send(Message::ScheduleComputeRequest).unwrap();
            }));

        imp.button_cmyk
            .connect_toggled(clone!(@weak window, @strong sender => move |_| {
                sender.clone().send(Message::ScheduleComputeRequest).unwrap();
            }));

        imp.button_path
            .connect_toggled(clone!(@weak window, @strong sender => move |_| {
                sender.clone().send(Message::ScheduleComputeRequest).unwrap();
            }));

        imp.save_button
            .connect_clicked(clone!(@weak window => move |_| {
                sender.clone().send(Message::SaveResult).unwrap();
            }));

        imp.drawing_area.add_controller(&imp.gesture_drag);

        let offset_x = Rc::new(RefCell::new(0.0));
        let offset_y = Rc::new(RefCell::new(0.0));

        fn get_viewport(gesture_drag: &gtk::GestureDrag) -> gtk::Viewport {
            gesture_drag
                .widget()
                .unwrap()
                .parent()
                .unwrap()
                .downcast::<gtk::Viewport>()
                .unwrap()
        }

        imp.gesture_drag.connect_drag_update(
            clone!(@weak offset_x, @weak offset_y => move |gesture_drag, dx, dy| {
                    let viewport = get_viewport(gesture_drag);
                    viewport.hadjustment().unwrap().set_value(*offset_x.borrow() - dx);
                    viewport.vadjustment().unwrap().set_value(*offset_y.borrow() - dy);
            }),
        );

        imp.gesture_drag
            .connect_drag_begin(move |gesture_drag, _, _| {
                let view_port = get_viewport(gesture_drag);
                offset_x.replace(view_port.hadjustment().unwrap().value());
                offset_y.replace(view_port.vadjustment().unwrap().value());
            });

        window
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = &imp::ApplicationWindow::from_instance(self).settings;

        let size = self.default_size();

        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = &imp::ApplicationWindow::from_instance(self).settings;

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

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
            cr.fill().expect("cannot fill");

            for (points, color) in data.point_sets.iter().zip(CMYK_AS_RGB.iter()) {
                if points.len() < 1 {
                    continue;
                }

                cr.set_source_rgba(color.0, color.1, color.2, 1.0);

                for point in points {
                    cr.arc(point.x, point.y, 1.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill().expect("cannot fill");
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
            cr.fill().expect("cannot fill");

            for (points, color) in data.point_sets.iter().zip(CMYK_AS_RGB.iter()) {
                if points.len() < 2 {
                    continue;
                }

                cr.set_source_rgba(color.0, color.1, color.2, 1.0);
                cr.move_to(points[0].x, points[0].y);

                for point in points.iter().skip(1) {
                    cr.line_to(point.x, point.y);
                }

                cr.stroke().expect("cannot stroke");
            }
        });
    }
}
