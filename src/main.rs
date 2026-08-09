mod render;
mod state;
mod ui;

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::Application;

fn main() {
    let app = Application::builder()
        .application_id("com.github.minimal_sketch")
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
            ui::build_ui(app);
        }
    });

    app.run();
}
