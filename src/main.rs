mod config;
mod state;
mod ui;
mod events;
mod menu;
mod render;

use gtk4::prelude::*;
use gtk4::Application;
use state::AppState;

fn main() {
    let app = Application::builder()
        .application_id("org.sketchlayer.app")
        .build();

    app.connect_activate(move |app| {
        let config = config::Config::load();
        let state = AppState::new(config);
        
        ui::build_ui(app, state);
    });

    app.run();
}
