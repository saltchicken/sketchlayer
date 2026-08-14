use anyhow::{anyhow, Context, Result};
use gtk::ApplicationWindow;
use gtk::prelude::*;
use gtk4 as gtk;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::render::stroke::render_stroke;
use crate::state::app_state::AppState;
use crate::state::geometry::BoundingBox;

pub fn save_sketch(window: &ApplicationWindow, state: &AppState) -> Result<()> {
    let width = window.width() as f64;
    let height = window.height() as f64;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let save_dir = state.config.get_resolved_save_dir().join("svgs");

    if !save_dir.exists() {
        fs::create_dir_all(&save_dir).context("Failed to create save directory")?;
    }

    let filename = format!("sketchlayer_{}.svg", timestamp);
    let full_path = save_dir.join(&filename);

    let path_str = full_path
        .to_str()
        .context("Failed to save SVG: Path contains invalid UTF-8")?;

    let is_transparent = state.config.transparent_background;

    let surface = gtk::cairo::SvgSurface::new(width, height, Some(path_str))
        .map_err(|e| anyhow!("Failed to create SvgSurface: {:?}", e))?;
        
    let cr = gtk::cairo::Context::new(&surface).context("Failed to create cairo context")?;

    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &state.config);
    }

    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &state.config);
    }

    surface.finish();
    info!("Sketch saved to {}", full_path.display());
    
    Ok(())
}

pub fn save_sketch_png(window: &ApplicationWindow, state: &AppState) -> Result<()> {
    let width = window.width();
    let height = window.height();

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let save_dir = state.config.get_resolved_save_dir().join("images");

    if !save_dir.exists() {
        fs::create_dir_all(&save_dir).context("Failed to create save directory")?;
    }

    let filename = format!("sketchlayer_{}.png", timestamp);
    let full_path = save_dir.join(&filename);

    let mut export_config = state.config.clone();
    export_config.transparent_background = false;
    export_config.background_color = [1.0, 1.0, 1.0, 1.0];

    let is_transparent = false;
    let [bg_r, bg_g, bg_b, bg_a] = export_config.background_color;

    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
        .map_err(|e| anyhow!("Failed to create surface for PNG: {:?}", e))?;

    let cr = gtk::cairo::Context::new(&surface).context("Failed to create cairo context")?;

    if is_transparent {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
    } else {
        cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
        cr.set_operator(gtk::cairo::Operator::Source);
    }
    cr.paint().context("Failed to paint background")?;
    cr.set_operator(gtk::cairo::Operator::Over);

    // Render all strokes
    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &export_config);
    }

    // Render active stroke if currently drawing
    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &export_config);
    }

    surface.flush();

    let mut file = fs::File::create(&full_path).context("Failed to create PNG file")?;

    surface
        .write_to_png(&mut file)
        .map_err(|e| anyhow!("Failed to encode surface to PNG: {:?}", e))?;
        
    info!("Sketch saved to {}", full_path.display());

    Ok(())
}

pub fn save_grids(state: &AppState) -> Result<()> {
    let cell_w = state.config.grid_cell_width;
    let cell_h = state.config.grid_cell_height;

    if cell_w <= 0.0 || cell_h <= 0.0 {
        return Err(anyhow!("Invalid grid dimensions"));
    }

    // Determine grid layout start position
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

    let save_dir = state.config.get_resolved_save_dir().join("svgs");
    if !save_dir.exists() {
        fs::create_dir_all(&save_dir).context("Failed to create save directory")?;
    }

    // Calculate maximum stroke padding so lines on borders are appropriately exported in adjacent cells
    let max_pen = state.config.base_pen_width + state.config.pen_pressure_mult;
    let max_eraser = state.config.base_eraser_width + state.config.eraser_pressure_mult;
    let padding = max_pen.max(max_eraser) / 2.0;

    let mut active_cells = std::collections::HashSet::new();

    let get_cells_for_bbox = |bbox: &BoundingBox| {
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
        info!("No geometry to save in grids.");
        return Ok(());
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

        let surface = gtk::cairo::SvgSurface::new(cell_w, cell_h, Some(path_str))
            .map_err(|e| anyhow!("Failed to create SvgSurface for grid cell: {:?}", e))?;
            
        let cr = gtk::cairo::Context::new(&surface).context("Failed to create cairo context")?;

        // Shift view to center on this cell coordinate
        cr.translate(-cell_x, -cell_y);

        if !is_transparent {
            cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
            cr.paint().context("Failed to paint background")?;
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
        info!("Grid cell ({}, {}) saved to {}", c, r, full_path.display());
    }
    
    Ok(())
}

pub fn save_main_grid_png(window: &ApplicationWindow, state: &AppState) -> Result<()> {
    let cell_w = state.config.grid_cell_width;
    let cell_h = state.config.grid_cell_height;

    if cell_w <= 0.0 || cell_h <= 0.0 {
        return Err(anyhow!("Invalid grid dimensions"));
    }

    let mut start_x = state.config.grid_offset_x % cell_w;
    if start_x < 0.0 {
        start_x += cell_w;
    }
    let mut start_y = state.config.grid_offset_y % cell_h;
    if start_y < 0.0 {
        start_y += cell_h;
    }

    let center_x = window.width() as f64 / 2.0;
    let center_y = window.height() as f64 / 2.0;

    let c = ((center_x - start_x) / cell_w).floor() as i32;
    let r = ((center_y - start_y) / cell_h).floor() as i32;

    let main_x = start_x + c as f64 * cell_w;
    let main_y = start_y + r as f64 * cell_h;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let save_dir = state.config.get_resolved_save_dir().join("images");
    if !save_dir.exists() {
        fs::create_dir_all(&save_dir).context("Failed to create save directory")?;
    }

    let filename = format!("sketchlayer_main_cell_{}_{}_{}.png", c, r, timestamp);
    let full_path = save_dir.join(&filename);

    let mut export_config = state.config.clone();
    export_config.transparent_background = false;
    export_config.background_color = [1.0, 1.0, 1.0, 1.0];

    let is_transparent = false;
    let [bg_r, bg_g, bg_b, bg_a] = export_config.background_color;

    let surface = gtk::cairo::ImageSurface::create(
        gtk::cairo::Format::ARgb32,
        cell_w as i32,
        cell_h as i32,
    ).map_err(|e| anyhow!("Failed to create surface for PNG: {:?}", e))?;

    let cr = gtk::cairo::Context::new(&surface).context("Failed to create cairo context")?;

    cr.translate(-main_x, -main_y);

    if is_transparent {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
    } else {
        cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
        cr.set_operator(gtk::cairo::Operator::Source);
    }
    cr.paint().context("Failed to paint background")?;
    cr.set_operator(gtk::cairo::Operator::Over);

    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &export_config);
    }

    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &export_config);
    }

    surface.flush();

    let mut file = fs::File::create(&full_path).context("Failed to create PNG file")?;

    surface
        .write_to_png(&mut file)
        .map_err(|e| anyhow!("Failed to encode surface to PNG: {:?}", e))?;
        
    info!("Main Grid ({}, {}) saved to {}", c, r, full_path.display());
    
    Ok(())
}
