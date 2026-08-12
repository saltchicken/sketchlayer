use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider, DrawingArea, gdk};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

use crate::events::{setup_keyboard_events, setup_stylus_events, setup_view_events};
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
}

fn setup_drawing_area(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) {
    drawing_area.set_draw_func(move |_area, cr, width, height| {
        let mut state = state.borrow_mut();
        let is_transparent = state.config.transparent_background;
        let [bg_r, bg_g, bg_b, bg_a] = state.config.background_color;
        let w = width as i32;
        let h = height as i32;

        let needs_rebuild = match &state.cached_surface {
            Some(surf) => surf.width() != w || surf.height() != h || state.needs_full_redraw,
            None => true,
        };

        if needs_rebuild {
            let surf = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, w, h)
                .expect("Failed to create ImageSurface");
            let cache_cr = gtk::cairo::Context::new(&surf).expect("Failed to create cache context");

            if is_transparent {
                cache_cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                cache_cr.set_operator(gtk::cairo::Operator::Clear);
            } else {
                cache_cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
                cache_cr.set_operator(gtk::cairo::Operator::Source);
            }
            cache_cr
                .paint()
                .expect("Failed to paint background to cache");
            cache_cr.set_operator(gtk::cairo::Operator::Over);

            cache_cr.translate(state.offset_x, state.offset_y);
            cache_cr.scale(state.zoom, state.zoom);

            for stroke in &state.strokes {
                render_stroke(&cache_cr, stroke.as_ref(), is_transparent, &state.config);
            }

            state.cached_surface = Some(surf);
            state.rendered_strokes_count = state.strokes.len();
            state.needs_full_redraw = false;
        } else if state.strokes.len() > state.rendered_strokes_count {
            if let Some(surf) = &state.cached_surface {
                let cache_cr =
                    gtk::cairo::Context::new(surf).expect("Failed to create cache context");

                cache_cr.translate(state.offset_x, state.offset_y);
                cache_cr.scale(state.zoom, state.zoom);

                for i in state.rendered_strokes_count..state.strokes.len() {
                    render_stroke(
                        &cache_cr,
                        state.strokes[i].as_ref(),
                        is_transparent,
                        &state.config,
                    );
                }
                state.rendered_strokes_count = state.strokes.len();
            }
        }

        if let Some(surf) = &state.cached_surface {
            cr.set_source_surface(surf, 0.0, 0.0)
                .expect("Failed to set source surface");
            cr.set_operator(gtk::cairo::Operator::Source);
            cr.paint().expect("Failed to paint cache to window");
            cr.set_operator(gtk::cairo::Operator::Over);
        }

        cr.save().expect("Failed to save cairo state");
        cr.translate(state.offset_x, state.offset_y);
        cr.scale(state.zoom, state.zoom);

        if let Some(current) = &state.current_stroke {
            render_stroke(cr, current, is_transparent, &state.config);
        }

        if state.config.show_grid {
            let cell_w = state.config.grid_cell_width;
            let cell_h = state.config.grid_cell_height;
            let off_x = state.config.grid_offset_x;
            let off_y = state.config.grid_offset_y;

            if cell_w > 0.0 && cell_h > 0.0 {
                cr.set_operator(gtk::cairo::Operator::Over);

                if is_transparent {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.15);
                } else {
                    let luminance = 0.299 * bg_r + 0.587 * bg_g + 0.114 * bg_b;
                    if luminance > 0.5 {
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.15);
                    } else {
                        cr.set_source_rgba(1.0, 1.0, 1.0, 0.15);
                    }
                }

                cr.set_line_width(1.0 / state.zoom);

                let (min_cx, min_cy) = state.screen_to_canvas(0.0, 0.0);
                let (max_cx, max_cy) = state.screen_to_canvas(width as f64, height as f64);

                let start_x = off_x + ((min_cx - off_x) / cell_w).floor() * cell_w;
                let start_y = off_y + ((min_cy - off_y) / cell_h).floor() * cell_h;

                let mut x = start_x;
                while x <= max_cx {
                    cr.move_to(x, min_cy);
                    cr.line_to(x, max_cy);
                    x += cell_w;
                }

                let mut y = start_y;
                while y <= max_cy {
                    cr.move_to(min_cx, y);
                    cr.line_to(max_cx, y);
                    y += cell_h;
                }

                cr.stroke().expect("Failed to draw grid");
            }
        }

        cr.restore().expect("Failed to restore cairo state");
    });
}
