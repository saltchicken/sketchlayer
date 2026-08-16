use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider, DrawingArea, gdk};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::path::PathBuf;
use tracing::{error, info};

use crate::events::keyboard::setup_keyboard_events;
use crate::events::pointer::{setup_stylus_events, setup_view_events};
use crate::ui::menu::build_context_menu;
use crate::ui::canvas::setup_drawing_area;
use crate::state::app_state::AppState;

pub fn build_ui(app: &Application, load_path: Option<PathBuf>) {
    setup_css();

    let state = AppState::new();

    // Prefer command line path, fallback to config path
    let config_load_path = state.borrow().config.get_resolved_load_file();
    let final_path = load_path.or(config_load_path);

    if let Some(path) = final_path {
        if let Err(e) = state.borrow_mut().load_state(&path) {
            error!("Failed to load state from {}: {:?}", path.display(), e);
        } else {
            info!("Loaded state from {}", path.display());
        }
    }

    let drawing_area = DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);
    drawing_area.set_cursor_from_name(Some("none"));

    let popover = build_context_menu(&drawing_area, state.clone());

    setup_drawing_area(&drawing_area, state.clone());
    setup_stylus_events(&drawing_area, state.clone(), popover);
    setup_view_events(&drawing_area, state.clone());

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Minimal Sketch")
        .child(&drawing_area)
        .build();

    window.set_cursor_from_name(Some("none"));
    window.init_layer_shell();

    // Check config for a targeted monitor, otherwise default to focused monitor
    if let Some(target) = &state.borrow().config.target_monitor {
        if !target.is_empty() {
            let display = gdk::Display::default().expect("Could not connect to a display.");
            let monitors = display.monitors();
            for i in 0..monitors.n_items() {
                if let Some(monitor) = monitors.item(i).and_downcast::<gdk::Monitor>() {
                    if let Some(connector) = monitor.connector() {
                        if connector.as_str() == target {
                            window.set_monitor(Some(&monitor));
                            break;
                        }
                    }
                }
            }
        }
    }

    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    setup_keyboard_events(&window, &drawing_area, state.clone());
    window.present();
}

fn setup_css() {
    let provider = CssProvider::new();
    provider.load_from_data("window { background-color: transparent; }");

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(true);
    }
}
