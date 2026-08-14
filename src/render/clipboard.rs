use anyhow::{anyhow, Context, Result};
use gtk::ApplicationWindow;
use gtk::prelude::*;
use gtk4 as gtk;
use tracing::info;

use crate::render::stroke::render_stroke;
use crate::state::app_state::AppState;

pub fn copy_to_clipboard(window: &ApplicationWindow, state: &AppState) -> Result<()> {
    let width = window.width();
    let height = window.height();

    let mut export_config = state.config.clone();
    export_config.transparent_background = false;
    export_config.background_color = [1.0, 1.0, 1.0, 1.0];

    let is_transparent = false;
    let [bg_r, bg_g, bg_b, bg_a] = export_config.background_color;

    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
        .map_err(|e| anyhow!("Failed to create surface for clipboard: {:?}", e))?;

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

    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &export_config);
    }

    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &export_config);
    }

    surface.flush();

    let mut png_data = Vec::new();
    surface.write_to_png(&mut png_data)
        .map_err(|e| anyhow!("Failed to encode surface to PNG: {:?}", e))?;

    let bytes = gtk::glib::Bytes::from(&png_data);
    let provider = gtk::gdk::ContentProvider::for_bytes("image/png", &bytes);

    window.clipboard().set_content(Some(&provider))
        .map_err(|e| anyhow!("Failed to set clipboard content: {:?}", e))?;
        
    info!("Sketch copied to clipboard as PNG");
    
    Ok(())
}

pub fn copy_main_grid_to_clipboard(window: &ApplicationWindow, state: &AppState) -> Result<()> {
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

    let mut export_config = state.config.clone();
    export_config.transparent_background = false;
    export_config.background_color = [1.0, 1.0, 1.0, 1.0];

    let is_transparent = false;
    let [bg_r, bg_g, bg_b, bg_a] = export_config.background_color;

    let surface = gtk::cairo::ImageSurface::create(
        gtk::cairo::Format::ARgb32,
        cell_w as i32,
        cell_h as i32,
    ).map_err(|e| anyhow!("Failed to create surface for clipboard: {:?}", e))?;

    let cr = gtk::cairo::Context::new(&surface).context("Failed to create cairo context")?;

    cr.translate(-main_x, -main_y);

    if is_transparent {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
        cr.paint().context("Failed to paint background")?;
        cr.set_operator(gtk::cairo::Operator::Over);
    } else {
        cr.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
        cr.set_operator(gtk::cairo::Operator::Source);
        cr.paint().context("Failed to paint background")?;
        cr.set_operator(gtk::cairo::Operator::Over);
    }

    for stroke in &state.strokes {
        render_stroke(&cr, stroke.as_ref(), is_transparent, &export_config);
    }

    if let Some(current) = &state.current_stroke {
        render_stroke(&cr, current, is_transparent, &export_config);
    }

    surface.flush();

    let mut png_data = Vec::new();
    surface.write_to_png(&mut png_data)
        .map_err(|e| anyhow!("Failed to encode surface to PNG: {:?}", e))?;

    let bytes = gtk::glib::Bytes::from(&png_data);
    let provider = gtk::gdk::ContentProvider::for_bytes("image/png", &bytes);

    window.clipboard().set_content(Some(&provider))
        .map_err(|e| anyhow!("Failed to set clipboard content: {:?}", e))?;
        
    info!("Main Grid ({}, {}) copied to clipboard as PNG", c, r);
    Ok(())
}
