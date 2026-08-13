mod config;
mod events;
mod menu;
mod render;
mod state;
mod ui;

use gtk::Application;
use gtk::gio;
use gtk::prelude::*;
use gtk4 as gtk;

fn main() {
    let app = Application::builder()
        .application_id("com.github.minimal_sketch")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(|app| {
        let windows = app.windows();

        if let Some(window) = windows.first() {
            if window.is_visible() {
                window.set_visible(false);
            } else {
                window.set_visible(true);
                window.present();
            }
        } else {
            ui::build_ui(app, None);
        }
    });

    app.connect_open(|app, files, _hint| {
        let path = files.first().and_then(|f| f.path());
        let windows = app.windows();
        
        if let Some(window) = windows.first() {
            window.set_visible(true);
            window.present();
            println!("App is already running. Please close it before loading a new state from the command line.");
        } else {
            ui::build_ui(app, path);
        }
    });

    app.run();
}
