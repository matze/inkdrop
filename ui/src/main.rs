mod application;
#[rustfmt::skip]
mod config;
mod window;

use anyhow::Result;
use application::Application;
use config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};
use gettextrs::*;
use gtk::gio;

fn main() -> Result<()> {
    pretty_env_logger::init();

    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR)?;
    textdomain(GETTEXT_PACKAGE)?;

    gtk::glib::set_application_name("Inkdrop");
    gtk::glib::set_prgname(Some("inkdrop"));

    gtk::init().expect("Unable to start GTK4");
    adw::init();

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = Application::new();
    app.run();

    Ok(())
}
