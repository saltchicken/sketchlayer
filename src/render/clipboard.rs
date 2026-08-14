use anyhow::{anyhow, Context, Result};
use gtk::ApplicationWindow;
use gtk::prelude::*;
use gtk4 as gtk;
use tracing::info;

use crate::render::stroke::render_scene;
use crate::state::app_state::AppState;

pub fn copy_to_clipboard(window: &ApplicationWindow, state: &AppState) -> Result<()> {
    let width = window.width();
    let height = window.height();

    let mut export_config = state.config.clone();
    export_config.transparent_background = false;
    export_config.background_color = [1.0, 1.0, 1.0, 1.0];

    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
        .map_err(|e| anyhow!("Failed to create surface for clipboard: {:?}", e))?;

    let cr = gtk::cairo::Context::new(&surface).context("Failed to create cairo context")?;

    render_scene(&cr, state, &export_config, false)?;

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

pub fn copy_active_frame_to_clipboard(window: &ApplicationWindow, state: &AppState) -> Result<()> {
    let frame_w = state.config.frame_width;
    let frame_h = state.config.frame_height;

    if frame_w <= 0.0 || frame_h <= 0.0 {
        return Err(anyhow!("Invalid frame dimensions"));
    }

    let mut start_x = state.config.frame_offset_x % frame_w;
    if start_x < 0.0 {
        start_x += frame_w;
    }
    let mut start_y = state.config.frame_offset_y % frame_h;
    if start_y < 0.0 {
        start_y += frame_h;
    }

    let center_x = window.width() as f64 / 2.0;
    let center_y = window.height() as f64 / 2.0;

    let c = ((center_x - start_x) / frame_w).floor() as i32;
    let r = ((center_y - start_y) / frame_h).floor() as i32;

    let main_x = start_x + c as f64 * frame_w;
    let main_y = start_y + r as f64 * frame_h;

    let mut export_config = state.config.clone();
    export_config.transparent_background = false;
    export_config.background_color = [1.0, 1.0, 1.0, 1.0];

    let surface = gtk::cairo::ImageSurface::create(
        gtk::cairo::Format::ARgb32,
        frame_w as i32,
        frame_h as i32,
    ).map_err(|e| anyhow!("Failed to create surface for clipboard: {:?}", e))?;

    let cr = gtk::cairo::Context::new(&surface).context("Failed to create cairo context")?;

    cr.translate(-main_x, -main_y);

    render_scene(&cr, state, &export_config, false)?;

    surface.flush();

    let mut png_data = Vec::new();
    surface.write_to_png(&mut png_data)
        .map_err(|e| anyhow!("Failed to encode surface to PNG: {:?}", e))?;

    let bytes = gtk::glib::Bytes::from(&png_data);
    let provider = gtk::gdk::ContentProvider::for_bytes("image/png", &bytes);

    window.clipboard().set_content(Some(&provider))
        .map_err(|e| anyhow!("Failed to set clipboard content: {:?}", e))?;
        
    info!("Active Frame ({}, {}) copied to clipboard as PNG", c, r);
    Ok(())
}
