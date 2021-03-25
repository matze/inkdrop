use anyhow::Result;
use crate::application::ExampleApplication;
use crate::config::{APP_ID, PROFILE};
use glib::clone;
use glib::signal::Inhibit;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use image::io::Reader;
use image::GenericImageView;
use log::warn;
use std::path::Path;
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/net/bloerg/inkdrop/window.ui")]
    pub struct ExampleApplicationWindow {
        #[template_child]
        pub drawing_area: TemplateChild<gtk::DrawingArea>,
        #[template_child]
        pub num_points: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub num_voronoi_iterations: TemplateChild<gtk::Adjustment>,
        pub dialog: gtk::FileChooserNative,
        pub settings: gio::Settings,
        pub points: Rc<Vec<Vec<inkdrop::point::Point>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ExampleApplicationWindow {
        const NAME: &'static str = "ExampleApplicationWindow";
        type Type = super::ExampleApplicationWindow;
        type ParentType = gtk::ApplicationWindow;

        fn new() -> Self {
            let dialog = gtk::FileChooserNativeBuilder::new()
                .title(&"Open image")
                .modal(true)
                .action(gtk::FileChooserAction::Open)
                .accept_label(&"_Open")
                .cancel_label(&"_Cancel")
                .build();

            Self {
                drawing_area: TemplateChild::default(),
                num_points: TemplateChild::default(),
                num_voronoi_iterations: TemplateChild::default(),
                dialog,
                settings: gio::Settings::new(APP_ID),
                points: Rc::new(vec![]),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("win.open", None, move |win, _, _| {
                let dialog = &imp::ExampleApplicationWindow::from_instance(&win).dialog;

                dialog.connect_response(clone!(@weak win => move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        let path = dialog.get_file().unwrap().get_path().unwrap();
                        win.update_image(&path);
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

    impl ObjectImpl for ExampleApplicationWindow {
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

    impl WidgetImpl for ExampleApplicationWindow {}
    impl WindowImpl for ExampleApplicationWindow {
        // save window state on delete event
        fn close_request(&self, obj: &Self::Type) -> Inhibit {
            if let Err(err) = obj.save_window_size() {
                warn!("Failed to save window state, {}", &err);
            }
            Inhibit(false)
        }
    }

    impl ApplicationWindowImpl for ExampleApplicationWindow {}
}

glib::wrapper! {
    pub struct ExampleApplicationWindow(ObjectSubclass<imp::ExampleApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl ExampleApplicationWindow {
    pub fn new(app: &ExampleApplication) -> Self {
        let window: Self =
            glib::Object::new(&[]).expect("Failed to create ExampleApplicationWindow");
        window.set_application(Some(app));

        gtk::Window::set_default_icon_name(APP_ID);

        window
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = &imp::ExampleApplicationWindow::from_instance(self).settings;

        let size = self.get_default_size();

        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = &imp::ExampleApplicationWindow::from_instance(self).settings;

        let width = settings.get_int("window-width");
        let height = settings.get_int("window-height");
        let is_maximized = settings.get_boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn update_image(&self, path: &Path) {
        let num_points = &imp::ExampleApplicationWindow::from_instance(self).num_points;
        let num_points = num_points.get_value() as usize;

        let img = Reader::open(path).unwrap().decode().unwrap();
        let (width, height) = img.dimensions();
        let mut point_sets = inkdrop::sample_points(&img, num_points, 1.0, false);

        let num_iterations = &imp::ExampleApplicationWindow::from_instance(self).num_voronoi_iterations;
        let num_iterations = num_iterations.get_value() as usize;

        for _ in 0..num_iterations {
            point_sets = point_sets
                .into_iter()
                .map(|ps| inkdrop::voronoi::move_points(ps, &img))
                .collect::<Result<Vec<_>>>().unwrap();
        }

        let area = &imp::ExampleApplicationWindow::from_instance(self).drawing_area;
        area.set_content_width(width as i32);
        area.set_content_height(height as i32);

        area.set_draw_func(move |_, cr, width, height| {
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            cr.fill();

            for ps in &point_sets {
                cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);

                for point in ps {
                    cr.arc(point.x, point.y, 1.0, 0.0, 2.0 * 3.1);
                    cr.fill();
                }
            }
        });
    }
}
