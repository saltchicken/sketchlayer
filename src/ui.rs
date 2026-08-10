use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider, DrawingArea, gdk};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

use crate::events::{setup_keyboard_events, setup_stylus_events};
use crate::menu::build_context_menu;
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
    drawing_area.set_draw_func(move |_area, cr, width, height| {
        let mut state = state.borrow_mut();
        let is_white_bg = state.config.white_background;
        let w = width as i32;
        let h = height as i32;

        let needs_rebuild = match &state.cached_surface {
            Some(surf) => surf.width() != w || surf.height() != h || state.needs_full_redraw,
            None => true,
        };

        if needs_rebuild {
            // Rebuild the entire cache from scratch
            let surf = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, w, h)
                .expect("Failed to create ImageSurface");
            let cache_cr = gtk::cairo::Context::new(&surf).expect("Failed to create cache context");

            if is_white_bg {
                cache_cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cache_cr.set_operator(gtk::cairo::Operator::Source);
            } else {
                cache_cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                cache_cr.set_operator(gtk::cairo::Operator::Clear);
            }
            cache_cr
                .paint()
                .expect("Failed to paint background to cache");
            cache_cr.set_operator(gtk::cairo::Operator::Over);

            for stroke in &state.strokes {
                render_stroke(&cache_cr, stroke.as_ref(), is_white_bg, &state.config);
            }

            state.cached_surface = Some(surf);
            state.rendered_strokes_count = state.strokes.len();
            state.needs_full_redraw = false;
        } else if state.strokes.len() > state.rendered_strokes_count {
            // Append only the newly completed strokes to the existing cache
            if let Some(surf) = &state.cached_surface {
                let cache_cr =
                    gtk::cairo::Context::new(surf).expect("Failed to create cache context");
                for i in state.rendered_strokes_count..state.strokes.len() {
                    render_stroke(
                        &cache_cr,
                        state.strokes[i].as_ref(),
                        is_white_bg,
                        &state.config,
                    );
                }
                state.rendered_strokes_count = state.strokes.len();
            }
        }

        // 1. Draw the Cached Surface directly to the window completely overriding the background
        if let Some(surf) = &state.cached_surface {
            cr.set_source_surface(surf, 0.0, 0.0)
                .expect("Failed to set source surface");
            cr.set_operator(gtk::cairo::Operator::Source);
            cr.paint().expect("Failed to paint cache to window");
            cr.set_operator(gtk::cairo::Operator::Over);
        }

        // 2. Draw the actively dragged stroke, if any
        if let Some(current) = &state.current_stroke {
            render_stroke(cr, current, is_white_bg, &state.config);
        }

        // 3. Draw Grid LAST (so it overlays everything and cannot be erased)
        if state.config.show_grid {
            let cell_w = state.config.grid_cell_width;
            let cell_h = state.config.grid_cell_height;
            let off_x = state.config.grid_offset_x;
            let off_y = state.config.grid_offset_y;

            if cell_w > 0.0 && cell_h > 0.0 {
                cr.set_operator(gtk::cairo::Operator::Over);

                if is_white_bg {
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.15);
                } else {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.15);
                }

                cr.set_line_width(1.0);

                // Calculate the starting position for the first vertical line
                let mut start_x = off_x % cell_w;
                if start_x < 0.0 {
                    start_x += cell_w;
                }

                // Calculate the starting position for the first horizontal line
                let mut start_y = off_y % cell_h;
                if start_y < 0.0 {
                    start_y += cell_h;
                }

                // Draw Vertical lines
                let mut x = start_x;
                while x < width as f64 {
                    cr.move_to(x, 0.0);
                    cr.line_to(x, height as f64);
                    x += cell_w;
                }

                // Draw Horizontal lines
                let mut y = start_y;
                while y < height as f64 {
                    cr.move_to(0.0, y);
                    cr.line_to(width as f64, y);
                    y += cell_h;
                }

                cr.stroke().expect("Failed to draw grid");
            }
        }
    });
}
