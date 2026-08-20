use gtk::prelude::*;
use gtk::DrawingArea;
use gtk4 as gtk;
use std::cell::RefCell;
use std::rc::Rc;

use crate::render::stroke::render_stroke;
use crate::state::app_state::AppState;

pub fn setup_drawing_area(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) {
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

        // Draw vanishing points helper
        // Active when enabled, hovering over the window, and not currently making a stroke
        if state.config.show_vanishing_points && state.current_stroke.is_none() {
            if let Some((hx, hy)) = state.hover_pos {
                cr.save().expect("Failed to save VP state");
                let is_dark_bg = {
                    let luminance = 0.299 * bg_r + 0.587 * bg_g + 0.114 * bg_b;
                    !is_transparent && luminance < 0.5
                };
                
                if is_dark_bg {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.5);
                } else {
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
                }

                cr.set_line_width(1.0 / state.zoom);
                let dash_len = 5.0 / state.zoom;
                cr.set_dash(&[dash_len, dash_len], 0.0);

                let frame_w = state.config.frame_width;
                let frame_h = state.config.frame_height;

                // Find the main frame relative to the center of the viewport
                let mut start_x = state.config.frame_offset_x % frame_w;
                if start_x < 0.0 { start_x += frame_w; }
                let mut start_y = state.config.frame_offset_y % frame_h;
                if start_y < 0.0 { start_y += frame_h; }

                let (canvas_center_x, canvas_center_y) = state.screen_to_canvas(w as f64 / 2.0, h as f64 / 2.0);

                let c = ((canvas_center_x - start_x) / frame_w).floor() as i32;
                let r = ((canvas_center_y - start_y) / frame_h).floor() as i32;

                let main_x = start_x + c as f64 * frame_w;
                let main_y = start_y + r as f64 * frame_h;

                // Offset the configured vanishing points by the main frame's origin
                let vps = [
                    [main_x + state.config.vp1[0], main_y + state.config.vp1[1]],
                    [main_x + state.config.vp2[0], main_y + state.config.vp2[1]],
                    [main_x + state.config.vp3[0], main_y + state.config.vp3[1]],
                ];

                for vp in vps {
                    cr.move_to(hx, hy);
                    cr.line_to(vp[0], vp[1]);
                }
                
                cr.stroke().expect("Failed to draw vanishing point guides");
                cr.restore().expect("Failed to restore VP state");
            }
        }

        if state.config.show_frames {
            let frame_w = state.config.frame_width;
            let frame_h = state.config.frame_height;
            let off_x = state.config.frame_offset_x;
            let off_y = state.config.frame_offset_y;

            if frame_w > 0.0 && frame_h > 0.0 {
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

                let start_x = off_x + ((min_cx - off_x) / frame_w).floor() * frame_w;
                let start_y = off_y + ((min_cy - off_y) / frame_h).floor() * frame_h;

                let mut x = start_x;
                while x <= max_cx {
                    cr.move_to(x, min_cy);
                    cr.line_to(x, max_cy);
                    x += frame_w;
                }

                let mut y = start_y;
                while y <= max_cy {
                    cr.move_to(min_cx, y);
                    cr.line_to(max_cx, y);
                    y += frame_h;
                }

                cr.stroke().expect("Failed to draw frames");
            }
        }

        cr.restore().expect("Failed to restore cairo state");
    });
}
