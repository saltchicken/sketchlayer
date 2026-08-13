use std::rc::Rc;
use std::cell::RefCell;
use gtk4::prelude::*;
use gtk4::{DrawingArea, ApplicationWindow};
use cairo::{ImageSurface, Format, Operator, Context};
use crate::state::AppState;
use crate::render::render_stroke;
use crate::events::setup_events;

pub fn build_ui(app: &gtk4::Application, state: Rc<RefCell<AppState>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Sketchlayer")
        .decorated(false)
        .build();

    gtk4_layer_shell::init_for_window(&window);
    gtk4_layer_shell::set_layer(&window, gtk4_layer_shell::Layer::Overlay);
    gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Top, true);
    gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Left, true);
    gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Right, true);
    gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Bottom, true);
    gtk4_layer_shell::set_keyboard_mode(&window, gtk4_layer_shell::KeyboardMode::OnDemand);
    gtk4_layer_shell::set_namespace(&window, "sketchlayer");
    
    let drawing_area = DrawingArea::new();
    let state_clone = state.clone();
    
    drawing_area.set_draw_func(move |_, ctx, width, height| {
        let mut s = state_clone.borrow_mut();
        
        if !s.config.transparent_background {
            let (r, g, b, a) = s.config.background_color;
            ctx.set_source_rgba(r, g, b, a);
            ctx.paint().unwrap();
        } else {
            ctx.set_operator(Operator::Clear);
            ctx.paint().unwrap();
            ctx.set_operator(Operator::Over);
        }

        if s.cached_surface.is_none() || s.needs_full_redraw {
            let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
            let cache_ctx = Context::new(&surface).unwrap();
            cache_ctx.translate(s.offset_x, s.offset_y);
            cache_ctx.scale(s.zoom, s.zoom);

            for stroke in &s.strokes {
                render_stroke(&cache_ctx, stroke, &s.config);
            }
            s.cached_surface = Some(surface);
            s.rendered_strokes_count = s.strokes.len();
            s.needs_full_redraw = false;
        } else if s.strokes.len() > s.rendered_strokes_count {
            let surface = s.cached_surface.as_ref().unwrap();
            let cache_ctx = Context::new(surface).unwrap();
            cache_ctx.translate(s.offset_x, s.offset_y);
            cache_ctx.scale(s.zoom, s.zoom);
            for i in s.rendered_strokes_count..s.strokes.len() {
                render_stroke(&cache_ctx, &s.strokes[i], &s.config);
            }
            s.rendered_strokes_count = s.strokes.len();
        }

        if let Some(surface) = &s.cached_surface {
            ctx.set_source_surface(surface, 0.0, 0.0).unwrap();
            ctx.paint().unwrap();
        }

        if let Some(active) = &s.active_stroke {
            ctx.save().unwrap();
            ctx.translate(s.offset_x, s.offset_y);
            ctx.scale(s.zoom, s.zoom);
            render_stroke(ctx, active, &s.config);
            ctx.restore().unwrap();
        }

        if s.config.show_grid {
            let cw = s.config.grid_cell_width * s.zoom;
            let ch = s.config.grid_cell_height * s.zoom;
            let ox = (s.config.grid_offset_x * s.zoom) + s.offset_x;
            let oy = (s.config.grid_offset_y * s.zoom) + s.offset_y;
            
            ctx.set_source_rgba(1.0, 1.0, 1.0, 0.2); 
            ctx.set_line_width(1.0);
            
            let mut x = ox % cw;
            while x < width as f64 {
                ctx.move_to(x, 0.0);
                ctx.line_to(x, height as f64);
                x += cw;
            }
            let mut y = oy % ch;
            while y < height as f64 {
                ctx.move_to(0.0, y);
                ctx.line_to(width as f64, y);
                y += ch;
            }
            let _ = ctx.stroke();
        }
    });

    setup_events(&drawing_area, state.clone(), &window);

    window.set_child(Some(&drawing_area));
    window.present();
}
