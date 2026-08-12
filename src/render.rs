use gtk::ApplicationWindow;
use gtk::prelude::*;
use gtk4 as gtk;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::state::{AppState, Stroke};

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

    let is_transparent = state.config.transparent_background;

    match gtk::cairo::SvgSurface::new(width, height, Some(path_str)) {
        Ok(surface) => {
            let cr = gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");

            for stroke in &state.strokes {
                render_stroke(&cr, stroke.as_ref(), is_transparent, &state.config);
            }

            if let Some(current) = &state.current_stroke {
                render_stroke(&cr, current, is_transparent, &state.config);
            }

            surface.finish();
            println!("✅ Sketch saved to {}", full_path.display());
        }
        Err(e) => eprintln!("❌ Failed to save SVG: {:?}", e),
    }
}

pub fn save_sketch_png(window: &ApplicationWindow, state: &AppState) {
    let width = window.width();
    let height = window.height();

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

    let filename = format!("sketchlayer_{}.png", timestamp);
    let full_path = save_dir.join(&filename);

    let is_transparent = state.config.transparent_background;
    let [bg_r, bg_g, bg_b, bg_a] = state.config.background_color;

    let surface = match gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height) {
        Ok(surf) => surf,
        Err(e) => {
            eprintln!("❌ Failed to create surface for PNG: {:?}", e);
            return;
        }
    };

    let cr = gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");

    if is_transparent {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
    } else {
        cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
        cr.set_operator(gtk::cairo::Operator::Source);
    }
    cr.paint().expect("Failed to paint background");
    cr.set_operator(gtk::cairo::Operator::Over);

    // Render all strokes
    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &state.config);
    }

    // Render active stroke if currently drawing
    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &state.config);
    }

    surface.flush();

    let mut file = match fs::File::create(&full_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("❌ Failed to create PNG file: {:?}", e);
            return;
        }
    };

    if let Err(e) = surface.write_to_png(&mut file) {
        eprintln!("❌ Failed to encode surface to PNG: {:?}", e);
    } else {
        println!("✅ Sketch saved to {}", full_path.display());
    }
}

pub fn save_grids(state: &AppState) {
    let cell_w = state.config.grid_cell_width;
    let cell_h = state.config.grid_cell_height;

    if cell_w <= 0.0 || cell_h <= 0.0 {
        eprintln!("❌ Invalid grid dimensions");
        return;
    }

    // Determine grid layout start position, mirroring ui.rs logic
    let mut start_x = state.config.grid_offset_x % cell_w;
    if start_x < 0.0 {
        start_x += cell_w;
    }
    let mut start_y = state.config.grid_offset_y % cell_h;
    if start_y < 0.0 {
        start_y += cell_h;
    }

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

    // Calculate maximum stroke padding so lines on borders are appropriately exported in adjacent cells
    let max_pen = state.config.base_pen_width + state.config.pen_pressure_mult;
    let max_eraser = state.config.base_eraser_width + state.config.eraser_pressure_mult;
    let padding = max_pen.max(max_eraser) / 2.0;

    let mut active_cells = std::collections::HashSet::new();

    let get_cells_for_bbox = |bbox: &crate::state::BoundingBox| {
        let min_c = ((bbox.min_x - padding - start_x) / cell_w).floor() as i32;
        let max_c = ((bbox.max_x + padding - start_x) / cell_w).floor() as i32;
        let min_r = ((bbox.min_y - padding - start_y) / cell_h).floor() as i32;
        let max_r = ((bbox.max_y + padding - start_y) / cell_h).floor() as i32;
        (min_c, max_c, min_r, max_r)
    };

    // Evaluate which cells have strokes
    for stroke in &state.strokes {
        let (min_c, max_c, min_r, max_r) = get_cells_for_bbox(&stroke.bbox);
        for c in min_c..=max_c {
            for r in min_r..=max_r {
                active_cells.insert((c, r));
            }
        }
    }

    if let Some(current) = &state.current_stroke {
        let (min_c, max_c, min_r, max_r) = get_cells_for_bbox(&current.bbox);
        for c in min_c..=max_c {
            for r in min_r..=max_r {
                active_cells.insert((c, r));
            }
        }
    }

    if active_cells.is_empty() {
        println!("ℹ️ No geometry to save in grids.");
        return;
    }

    let is_transparent = state.config.transparent_background;
    let [bg_r, bg_g, bg_b, bg_a] = state.config.background_color;

    for (c, r) in active_cells {
        let cell_x = start_x + c as f64 * cell_w;
        let cell_y = start_y + r as f64 * cell_h;

        let filename = format!("sketchlayer_{}_cell_{}_{}.svg", timestamp, c, r);
        let full_path = save_dir.join(&filename);

        let path_str = match full_path.to_str() {
            Some(s) => s,
            None => continue,
        };

        match gtk::cairo::SvgSurface::new(cell_w, cell_h, Some(path_str)) {
            Ok(surface) => {
                let cr =
                    gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");

                // Shift view to center on this cell coordinate
                cr.translate(-cell_x, -cell_y);

                if !is_transparent {
                    cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
                    cr.paint().expect("Failed to paint background");
                    cr.set_operator(gtk::cairo::Operator::Over);
                }

                // Render intersecting strokes
                for stroke in &state.strokes {
                    let (min_c, max_c, min_r, max_r) = get_cells_for_bbox(&stroke.bbox);
                    if c >= min_c && c <= max_c && r >= min_r && r <= max_r {
                        render_stroke(&cr, stroke.as_ref(), is_transparent, &state.config);
                    }
                }

                if let Some(current) = &state.current_stroke {
                    let (min_c, max_c, min_r, max_r) = get_cells_for_bbox(&current.bbox);
                    if c >= min_c && c <= max_c && r >= min_r && r <= max_r {
                        render_stroke(&cr, current, is_transparent, &state.config);
                    }
                }

                surface.finish();
                println!(
                    "✅ Grid cell ({}, {}) saved to {}",
                    c,
                    r,
                    full_path.display()
                );
            }
            Err(e) => eprintln!("❌ Failed to save grid cell SVG: {:?}", e),
        }
    }
}

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

pub fn copy_to_clipboard(window: &ApplicationWindow, state: &AppState) {
    let width = window.width();
    let height = window.height();

    let is_transparent = state.config.transparent_background;
    let [bg_r, bg_g, bg_b, bg_a] = state.config.background_color;

    let surface = match gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
    {
        Ok(surf) => surf,
        Err(e) => {
            eprintln!("❌ Failed to create surface for clipboard: {:?}", e);
            return;
        }
    };

    let cr = gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");

    if is_transparent {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
    } else {
        cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
        cr.set_operator(gtk::cairo::Operator::Source);
    }
    cr.paint().expect("Failed to paint background");
    cr.set_operator(gtk::cairo::Operator::Over);

    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &state.config);
    }

    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &state.config);
    }

    surface.flush();

    let mut png_data = Vec::new();
    if let Err(e) = surface.write_to_png(&mut png_data) {
        eprintln!("❌ Failed to encode surface to PNG: {:?}", e);
        return;
    }

    let bytes = gtk::glib::Bytes::from(&png_data);
    let provider = gtk::gdk::ContentProvider::for_bytes("image/png", &bytes);

    if let Err(e) = window.clipboard().set_content(Some(&provider)) {
        eprintln!("❌ Failed to set clipboard content: {:?}", e);
    } else {
        println!("✅ Sketch copied to clipboard as PNG");
    }
}

pub fn copy_main_grid_to_clipboard(window: &ApplicationWindow, state: &AppState) {
    let cell_w = state.config.grid_cell_width;
    let cell_h = state.config.grid_cell_height;

    if cell_w <= 0.0 || cell_h <= 0.0 {
        eprintln!("❌ Invalid grid dimensions");
        return;
    }

    // 1. Calculate grid alignment (mirroring your save_grids logic)
    let mut start_x = state.config.grid_offset_x % cell_w;
    if start_x < 0.0 {
        start_x += cell_w;
    }
    let mut start_y = state.config.grid_offset_y % cell_h;
    if start_y < 0.0 {
        start_y += cell_h;
    }

    // 2. Find the center of your screen/window
    let center_x = window.width() as f64 / 2.0;
    let center_y = window.height() as f64 / 2.0;

    // 3. Determine which grid column (c) and row (r) that center point falls into
    let c = ((center_x - start_x) / cell_w).floor() as i32;
    let r = ((center_y - start_y) / cell_h).floor() as i32;

    // 4. Calculate the top-left pixel coordinates of this specific cell
    let main_x = start_x + c as f64 * cell_w;
    let main_y = start_y + r as f64 * cell_h;

    let is_transparent = state.config.transparent_background;
    let [bg_r, bg_g, bg_b, bg_a] = state.config.background_color;

    // 5. Create a surface exactly the size of your grid cell
    let surface = match gtk::cairo::ImageSurface::create(
        gtk::cairo::Format::ARgb32,
        cell_w as i32,
        cell_h as i32,
    ) {
        Ok(surf) => surf,
        Err(e) => {
            eprintln!("❌ Failed to create surface for clipboard: {:?}", e);
            return;
        }
    };

    let cr = gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");

    // 6. Shift the canvas so the top-left of the main cell becomes (0,0)
    cr.translate(-main_x, -main_y);

    if is_transparent {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
        cr.paint().expect("Failed to paint background");
        cr.set_operator(gtk::cairo::Operator::Over);
    } else {
        cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
        cr.set_operator(gtk::cairo::Operator::Source);
        cr.paint().expect("Failed to paint background");
        cr.set_operator(gtk::cairo::Operator::Over);
    }

    // 7. Render strokes (Cairo automatically clips anything outside the bounds)
    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &state.config);
    }

    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &state.config);
    }

    surface.flush();

    let mut png_data = Vec::new();
    if let Err(e) = surface.write_to_png(&mut png_data) {
        eprintln!("❌ Failed to encode surface to PNG: {:?}", e);
        return;
    }

    let bytes = gtk::glib::Bytes::from(&png_data);
    let provider = gtk::gdk::ContentProvider::for_bytes("image/png", &bytes);

    if let Err(e) = window.clipboard().set_content(Some(&provider)) {
        eprintln!("❌ Failed to set clipboard content: {:?}", e);
    } else {
        println!("✅ Main Grid ({}, {}) copied to clipboard as PNG", c, r);
    }
}
