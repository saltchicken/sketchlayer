use cairo::{Context, Operator};
use crate::state::{Stroke, AppState};
use crate::config::Config;

pub fn render_stroke(ctx: &Context, stroke: &Stroke, config: &Config) {
    if stroke.points.is_empty() { return; }

    if stroke.is_eraser {
        ctx.set_operator(Operator::DestOut);
        ctx.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    } else {
        ctx.set_operator(Operator::Over);
        ctx.set_source_rgb(stroke.color.0, stroke.color.1, stroke.color.2);
    }

    let pts = &stroke.points;
    
    if pts.len() == 1 {
        let p = &pts[0];
        let width = if stroke.is_eraser { config.base_eraser_width + (p.pressure * config.eraser_pressure_mult) }
                    else { config.base_pen_width + (p.pressure * config.pen_pressure_mult) };
        ctx.set_line_width(width);
        ctx.set_line_cap(cairo::LineCap::Round);
        ctx.move_to(p.x, p.y);
        ctx.line_to(p.x, p.y);
        let _ = ctx.stroke();
    } else {
        for i in 0..pts.len()-1 {
            let p1 = &pts[i];
            let p2 = &pts[i+1];
            
            let width = if stroke.is_eraser { config.base_eraser_width + (p1.pressure * config.eraser_pressure_mult) }
                        else { config.base_pen_width + (p1.pressure * config.pen_pressure_mult) };
            
            ctx.set_line_width(width);
            ctx.set_line_cap(cairo::LineCap::Round);
            ctx.set_line_join(cairo::LineJoin::Round);
            ctx.move_to(p1.x, p1.y);
            
            let mid_x = (p1.x + p2.x) / 2.0;
            let mid_y = (p1.y + p2.y) / 2.0;
            let cp1_x = p1.x + (mid_x - p1.x) * 0.66;
            let cp1_y = p1.y + (mid_y - p1.y) * 0.66;
            let cp2_x = p2.x + (mid_x - p2.x) * 0.66;
            let cp2_y = p2.y + (mid_y - p2.y) * 0.66;
            
            ctx.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, p2.x, p2.y);
            let _ = ctx.stroke();
        }
    }
}

pub fn export_svg(state: &AppState, filepath: &std::path::Path, width: f64, height: f64) {
    if let Ok(surface) = cairo::SvgSurface::new(width, height, Some(filepath)) {
        let ctx = Context::new(&surface).unwrap();
        if !state.config.transparent_background {
            let (r, g, b, a) = state.config.background_color;
            ctx.set_source_rgba(r, g, b, a);
            ctx.paint().unwrap();
        }
        ctx.translate(state.offset_x, state.offset_y);
        ctx.scale(state.zoom, state.zoom);
        for stroke in &state.strokes { render_stroke(&ctx, stroke, &state.config); }
        surface.finish();
    }
}

pub fn export_grids(state: &AppState, dir: &std::path::Path) {
    let cell_w = state.config.grid_cell_width;
    let cell_h = state.config.grid_cell_height;
    
    for stroke in &state.strokes {
        if let Some(ref bbox) = stroke.bbox {
            let start_col = ((bbox.min_x - state.config.grid_offset_x) / cell_w).floor() as i32;
            let end_col = ((bbox.max_x - state.config.grid_offset_x) / cell_w).floor() as i32;
            let start_row = ((bbox.min_y - state.config.grid_offset_y) / cell_h).floor() as i32;
            let end_row = ((bbox.max_y - state.config.grid_offset_y) / cell_h).floor() as i32;
            
            for col in start_col..=end_col {
                for row in start_row..=end_row {
                    let filename = dir.join(format!("grid_{}_{}.svg", col, row));
                    if !filename.exists() {
                        let surface = cairo::SvgSurface::new(cell_w, cell_h, Some(&filename)).unwrap();
                        let ctx = Context::new(&surface).unwrap();
                        ctx.translate(-((col as f64) * cell_w + state.config.grid_offset_x), 
                                      -((row as f64) * cell_h + state.config.grid_offset_y));
                        for s in &state.strokes { render_stroke(&ctx, s, &state.config); }
                        surface.finish();
                    }
                }
            }
        }
    }
}
