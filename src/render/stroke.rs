use anyhow::{Context, Result};
use gtk4 as gtk;
use std::f64::consts::PI;

use crate::config::Config;
use crate::state::app_state::AppState;
use crate::state::geometry::Stroke;

pub fn render_stroke(
    cr: &gtk::cairo::Context,
    stroke: &Stroke,
    is_transparent: bool,
    config: &Config,
) {
    if stroke.points.len() < 2 {
        return;
    }

    let [bg_r, bg_g, bg_b, _] = config.background_color;

    if stroke.is_eraser {
        if !is_transparent {
            cr.set_operator(gtk::cairo::Operator::Over);
        } else {
            cr.set_operator(gtk::cairo::Operator::DestOut);
        }
    } else {
        cr.set_operator(gtk::cairo::Operator::Over);
    }

    // Use Butt caps to prevent internal segments from overlapping and stacking opacity
    cr.set_line_cap(gtk::cairo::LineCap::Butt);
    cr.set_line_join(gtk::cairo::LineJoin::Miter);

    let (r, g, b) = stroke.color;

    let set_style = |pressure: f64| {
        let alpha = pressure.clamp(0.1, 1.0);
        if stroke.is_eraser {
            cr.set_line_width(config.base_eraser_width + (pressure * config.eraser_pressure_mult));
            if !is_transparent {
                cr.set_source_rgba(bg_r, bg_g, bg_b, alpha);
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
        let radius = cr.line_width() / 2.0;
        let theta = (p2.y - p1.y).atan2(p2.x - p1.x);

        // Explicit Start cap (backward facing semi-circle)
        cr.arc(p1.x, p1.y, radius, theta + PI / 2.0, theta + PI * 1.5);
        cr.fill().expect("Failed to fill start cap");

        cr.move_to(p1.x, p1.y);
        cr.line_to(p2.x, p2.y);
        cr.stroke().expect("Failed to stroke path");

        // Explicit End cap (forward facing semi-circle)
        cr.arc(p2.x, p2.y, radius, theta - PI / 2.0, theta + PI / 2.0);
        cr.fill().expect("Failed to fill end cap");
        return;
    }

    let p0 = &stroke.points[0];
    let p1 = &stroke.points[1];
    
    set_style(p0.pressure);
    let radius_start = cr.line_width() / 2.0;
    let theta_start = (p1.y - p0.y).atan2(p1.x - p0.x);

    // Explicit Start cap
    cr.arc(p0.x, p0.y, radius_start, theta_start + PI / 2.0, theta_start + PI * 1.5);
    cr.fill().expect("Failed to fill start cap");

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

    // Explicit End cap 
    let radius_end = cr.line_width() / 2.0;
    let theta_end = (p_last.y - start_y).atan2(p_last.x - start_x);
    cr.arc(p_last.x, p_last.y, radius_end, theta_end - PI / 2.0, theta_end + PI / 2.0);
    cr.fill().expect("Failed to fill end cap");
}

pub fn render_scene(
    cr: &gtk::cairo::Context,
    state: &AppState,
    config: &Config,
    is_transparent: bool,
) -> Result<()> {
    if is_transparent {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
        cr.paint().context("Failed to paint clear background")?;
        cr.set_operator(gtk::cairo::Operator::Over);
    } else {
        let [bg_r, bg_g, bg_b, bg_a] = config.background_color;
        cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
        cr.set_operator(gtk::cairo::Operator::Source);
        cr.paint().context("Failed to paint solid background")?;
        cr.set_operator(gtk::cairo::Operator::Over);
    }

    for stroke in &state.strokes {
        render_stroke(cr, stroke.as_ref(), is_transparent, config);
    }

    if let Some(current) = &state.current_stroke {
        render_stroke(cr, current, is_transparent, config);
    }

    Ok(())
}
