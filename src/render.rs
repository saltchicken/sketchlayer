use gtk::ApplicationWindow;
use gtk::prelude::*;
use gtk4 as gtk;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;

use crate::state::{AppState, Stroke};
use crate::config::Config;

pub fn save_sketch(window: &ApplicationWindow, state: &AppState) {
    let width = window.width() as f64;
    let height = window.height() as f64;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
        
    let save_dir = state.config.get_resolved_save_dir();
    
    if !save_dir.exists() {
        if let Err(e) = fs::create_dir_all(&save_dir) {
            eprintln!("❌ Failed to create save directory: {:?}", e);
            return;
        }
    }

    let filename = format!("sketchlayer_{}.svg", timestamp);
    let full_path = save_dir.join(&filename);
    
    let path_str = match full_path.to_str() {
        Some(s) => s,
        None => {
            eprintln!("❌ Failed to save SVG: Path contains invalid UTF-8");
            return;
        }
    };

    let is_white_bg = state.white_background;

    match gtk::cairo::SvgSurface::new(width, height, Some(path_str)) {
        Ok(surface) => {
            let cr = gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");

            for stroke in &state.strokes {
                render_stroke(&cr, stroke.as_ref(), is_white_bg, &state.config);
            }

            if let Some(current) = &state.current_stroke {
                render_stroke(&cr, current, is_white_bg, &state.config);
            }

            surface.finish();
            println!("✅ Sketch saved to {}", full_path.display());
        }
        Err(e) => eprintln!("❌ Failed to save SVG: {:?}", e),
    }
}

pub fn render_stroke(cr: &gtk::cairo::Context, stroke: &Stroke, is_white_bg: bool, config: &Config) {
    if stroke.points.len() < 2 {
        return;
    }

    if stroke.is_eraser {
        if is_white_bg {
            cr.set_operator(gtk::cairo::Operator::Over);
        } else {
            cr.set_operator(gtk::cairo::Operator::DestOut);
        }
    } else {
        cr.set_operator(gtk::cairo::Operator::Over);
    }

    cr.set_line_cap(gtk::cairo::LineCap::Butt);
    cr.set_line_join(gtk::cairo::LineJoin::Miter);

    let (r, g, b) = stroke.color;

    let set_style = |pressure: f64| {
        let alpha = pressure.clamp(0.1, 1.0);
        if stroke.is_eraser {
            cr.set_line_width(config.base_eraser_width + (pressure * config.eraser_pressure_mult)); 
            if is_white_bg {
                cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
            } else {
                cr.set_source_rgba(0.0, 0.0, 0.0, alpha); 
            }
        } else {
            cr.set_line_width(config.base_pen_width + (pressure * config.pen_pressure_mult));
            cr.set_source_rgba(r, g, b, alpha);
        }
    };

    if stroke.points.len() == 2 {
        let p1 = &stroke.points[0];
        let p2 = &stroke.points[1];

        set_style(p1.pressure);
        cr.move_to(p1.x, p1.y);
        cr.line_to(p2.x, p2.y);
        cr.stroke().expect("Failed to stroke path");
        return;
    }

    let mut start_x = stroke.points[0].x;
    let mut start_y = stroke.points[0].y;

    for i in 1..(stroke.points.len() - 1) {
        let p_ctrl = &stroke.points[i];
        let p_next = &stroke.points[i + 1];

        let end_x = (p_ctrl.x + p_next.x) / 2.0;
        let end_y = (p_ctrl.y + p_next.y) / 2.0;

        let cp1_x = start_x + (2.0 / 3.0) * (p_ctrl.x - start_x);
        let cp1_y = start_y + (2.0 / 3.0) * (p_ctrl.y - start_y);
        let cp2_x = end_x + (2.0 / 3.0) * (p_ctrl.x - end_x);
        let cp2_y = end_y + (2.0 / 3.0) * (p_ctrl.y - end_y);

        set_style(p_ctrl.pressure);

        cr.move_to(start_x, start_y);
        cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, end_x, end_y);
        cr.stroke().expect("Failed to stroke path");

        start_x = end_x;
        start_y = end_y;
    }

    let p_last = &stroke.points[stroke.points.len() - 1];
    set_style(p_last.pressure);
    cr.move_to(start_x, start_y);
    cr.line_to(p_last.x, p_last.y);
    cr.stroke().expect("Failed to stroke path");
}
