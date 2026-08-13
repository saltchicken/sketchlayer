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
use std::path::PathBuf;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    // 1. Establish the log directory
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("sketchlayer")
        .join("logs");

    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create log directory at {:?}: {}", log_dir, e);
    }

    // 2. Set up the rolling file appender (creates a new file daily)
    let file_appender = tracing_appender::rolling::daily(&log_dir, "sketchlayer.log");
    
    // 3. Wrap it in a non-blocking writer
    // The `_guard` must remain in scope for the lifetime of `main` to ensure logs are flushed on exit.
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

    // 4. Compose layers: console output + file output
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer()) // stdout
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_appender)) // file
        .init();

    info!("Sketchlayer started. Logs are being written to {:?}", log_dir);

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
            warn!("App is already running. Please close it before loading a new state from the command line.");
        } else {
            ui::build_ui(app, path);
        }
    });

    app.run();
}
