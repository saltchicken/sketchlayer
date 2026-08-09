use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, CssProvider, DrawingArea, gdk,
};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

use crate::menu::build_context_menu;
use crate::events::{setup_stylus_events, setup_keyboard_events};
use crate::render::render_stroke;
use crate::state::AppState;

pub fn build_ui(app: &Application) {
    setup_css();

    let state = AppState::new();

    let drawing_area = DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);
    drawing_area.set_cursor_from_name(Some("none"));

    let popover = build_context_menu(&drawing_area, state.clone());

    setup_drawing_area(&drawing_area, state.clone());
    setup_stylus_events(&drawing_area, state.clone(), popover);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Minimal Sketch")
        .child(&drawing_area)
        .build();

    window.set_cursor_from_name(Some("none"));
    window.init_layer_shell();
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
}

fn setup_drawing_area(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) {
    drawing_area.set_draw_func(move |_area, cr, _width, _height| {
        let state = state.borrow();
        let is_white_bg = state.white_background;

        if is_white_bg {
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.set_operator(gtk::cairo::Operator::Source);
        } else {
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.set_operator(gtk::cairo::Operator::Clear);
        }
        
        cr.paint().expect("Failed to paint background");
        cr.set_operator(gtk::cairo::Operator::Over);

        for stroke in &state.strokes {
            render_stroke(cr, stroke.as_ref(), is_white_bg, &state.config);
        }

        if let Some(current) = &state.current_stroke {
            render_stroke(cr, current, is_white_bg, &state.config);
        }
    });
}
