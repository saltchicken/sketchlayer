use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, DrawingArea, Orientation, Popover, glib};
use gtk4 as gtk;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{error, info};

use crate::render::clipboard::{copy_main_grid_to_clipboard, copy_to_clipboard};
use crate::render::export::{save_grids, save_main_grid_png, save_sketch, save_sketch_png};
use crate::state::app_state::AppState;
use crate::state::geometry::{Action, EraseMode};

pub fn build_context_menu(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) -> Popover {
    let popover = Popover::new();
    popover.set_parent(drawing_area);
    popover.set_has_arrow(true);

    let menu_box = gtk::Box::new(Orientation::Vertical, 4);
    menu_box.set_margin_top(4);
    menu_box.set_margin_bottom(4);
    menu_box.set_margin_start(4);
    menu_box.set_margin_end(4);

    // Custom Color Picker Widget via Nested Popover
    #[allow(deprecated)]
    {
        let custom_color_box = gtk::Box::new(Orientation::Horizontal, 8);
        custom_color_box.set_margin_start(4);
        custom_color_box.set_margin_end(4);
        custom_color_box.set_margin_bottom(8);

        let custom_color_label = gtk::Label::new(Some("Custom Color:"));
        custom_color_label.set_hexpand(true);
        custom_color_label.set_halign(gtk::Align::Start);

        // A MenuButton naturally spawns a nested Popover attached to itself.
        // This stops the main menu from resizing and jumping out from under your cursor.
        let color_menu_btn = gtk::MenuButton::new();
        color_menu_btn.set_label("Pick...");
        color_menu_btn.set_always_show_arrow(true);

        let color_popover = gtk::Popover::new();
        let color_chooser = gtk::ColorChooserWidget::new();
        color_chooser.set_use_alpha(false);

        let current_color = state.borrow().current_color;
        color_chooser.set_rgba(&gtk::gdk::RGBA::new(
            current_color.0 as f32,
            current_color.1 as f32,
            current_color.2 as f32,
            1.0,
        ));

        color_chooser.connect_rgba_notify(glib::clone!(
            #[strong] state,
            move |chooser| {
                let rgba = chooser.rgba();
                state.borrow_mut().current_color = (rgba.red() as f64, rgba.green() as f64, rgba.blue() as f64);
            }
        ));

        color_popover.set_child(Some(&color_chooser));
        color_menu_btn.set_popover(Some(&color_popover));
        
        custom_color_box.append(&custom_color_label);
        custom_color_box.append(&color_menu_btn);
        menu_box.append(&custom_color_box);
    }

    let btn_erase_mode = Button::with_label("Erase Mode: Vector");
    let btn_undo = Button::with_label("Undo");
    let btn_redo = Button::with_label("Redo");
    let btn_bg = Button::with_label("Toggle Background");
    let btn_grid = Button::with_label("Toggle Grid");
    let btn_reset_view = Button::with_label("Reset View");

    let opacity_box = gtk::Box::new(Orientation::Horizontal, 8);
    opacity_box.set_margin_start(4);
    opacity_box.set_margin_end(4);
    let opacity_label = gtk::Label::new(Some("Opacity:"));
    let opacity_scale = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
    opacity_scale.set_value(100.0);
    opacity_scale.set_draw_value(true);
    opacity_scale.set_hexpand(true);
    opacity_box.append(&opacity_label);
    opacity_box.append(&opacity_scale);

    let pen_width_box = gtk::Box::new(Orientation::Horizontal, 8);
    pen_width_box.set_margin_start(4);
    pen_width_box.set_margin_end(4);
    let pen_width_label = gtk::Label::new(Some("Pen Width:"));
    let pen_width_scale = gtk::Scale::with_range(Orientation::Horizontal, 0.1, 50.0, 0.1);
    pen_width_scale.set_digits(1);
    pen_width_scale.set_value(state.borrow().config.base_pen_width);
    pen_width_scale.set_draw_value(true);
    pen_width_scale.set_hexpand(true);
    pen_width_box.append(&pen_width_label);
    pen_width_box.append(&pen_width_scale);

    let eraser_width_box = gtk::Box::new(Orientation::Horizontal, 8);
    eraser_width_box.set_margin_start(4);
    eraser_width_box.set_margin_end(4);
    let eraser_width_label = gtk::Label::new(Some("Eraser Width:"));
    let eraser_width_scale = gtk::Scale::with_range(Orientation::Horizontal, 0.1, 100.0, 0.1);
    eraser_width_scale.set_digits(1);
    eraser_width_scale.set_value(state.borrow().config.base_eraser_width);
    eraser_width_scale.set_draw_value(true);
    eraser_width_scale.set_hexpand(true);
    eraser_width_box.append(&eraser_width_label);
    eraser_width_box.append(&eraser_width_scale);

    let btn_save_state = Button::with_label("Save State (.sketchlayer)");
    let btn_save = Button::with_label("Save Full Sketch (SVG)");
    let btn_save_png = Button::with_label("Save Full Sketch (PNG)");
    let btn_save_grid = Button::with_label("Save Grid Cells (SVG)");
    let btn_save_main_png = Button::with_label("Save Main Grid (PNG)");
    let btn_copy = Button::with_label("Copy Full Screen");
    let btn_copy_main = Button::with_label("Copy Main Grid");
    let btn_clear = Button::with_label("Clear Canvas");
    let btn_hide = Button::with_label("Hide Overlay");
    let btn_quit = Button::with_label("Quit");

    menu_box.append(&btn_erase_mode);
    menu_box.append(&btn_undo);
    menu_box.append(&btn_redo);
    menu_box.append(&btn_bg);
    menu_box.append(&btn_grid);
    menu_box.append(&btn_reset_view);
    menu_box.append(&opacity_box);
    menu_box.append(&pen_width_box);
    menu_box.append(&eraser_width_box);
    menu_box.append(&btn_save_state);
    menu_box.append(&btn_save);
    menu_box.append(&btn_save_png);
    menu_box.append(&btn_save_grid);
    menu_box.append(&btn_save_main_png);
    menu_box.append(&btn_copy);
    menu_box.append(&btn_copy_main);
    menu_box.append(&btn_clear);
    menu_box.append(&btn_hide);
    menu_box.append(&btn_quit);

    btn_erase_mode.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |btn| {
            let mut s = state.borrow_mut();
            if s.erase_mode == EraseMode::Vector {
                s.erase_mode = EraseMode::Pixel;
                btn.set_label("Erase Mode: Pixel");
            } else {
                s.erase_mode = EraseMode::Vector;
                btn.set_label("Erase Mode: Vector");
            }
        }
    ));

    btn_undo.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[strong]
        state,
        move |_| {
            if state.borrow_mut().undo() {
                drawing_area.queue_draw();
            }
        }
    ));

    btn_redo.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[strong]
        state,
        move |_| {
            if state.borrow_mut().redo() {
                drawing_area.queue_draw();
            }
        }
    ));

    btn_bg.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.config.transparent_background = !s.config.transparent_background;
                s.needs_full_redraw = true;
            }
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    btn_grid.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.config.show_grid = !s.config.show_grid;
            }
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    btn_reset_view.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            state.borrow_mut().reset_view();
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    opacity_scale.connect_value_changed(glib::clone!(
        #[weak]
        drawing_area,
        move |scale| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                window.set_opacity(scale.value() / 100.0);
            }
        }
    ));

    pen_width_scale.connect_value_changed(glib::clone!(
        #[strong]
        state,
        move |scale| {
            let mut s = state.borrow_mut();
            s.config.base_pen_width = scale.value();
        }
    ));

    eraser_width_scale.connect_value_changed(glib::clone!(
        #[strong]
        state,
        move |scale| {
            let mut s = state.borrow_mut();
            s.config.base_eraser_width = scale.value();
        }
    ));

    btn_save_state.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            let full_path = {
                let s = state.borrow();
                
                if let Some(ref path) = s.current_file {
                    path.clone()
                } else {
                    let save_dir = s.config.get_resolved_save_dir().join("sketchlayers");
                    
                    if !save_dir.exists() {
                        if let Err(e) = std::fs::create_dir_all(&save_dir) {
                            error!("Failed to create save directory: {:?}", e);
                            popover.popdown();
                            return;
                        }
                    }

                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let filename = format!("sketchlayer_{}.sketchlayer", timestamp);
                    save_dir.join(&filename)
                }
            };

            if let Err(e) = state.borrow().save_state(&full_path) {
                error!("Failed to save state: {:?}", e);
            } else {
                info!("State saved to {}", full_path.display());
                state.borrow_mut().current_file = Some(full_path);
            }
            popover.popdown();
        }
    ));

    btn_save.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                if let Err(e) = save_sketch(&window, &*state.borrow()) {
                    error!("Failed to save sketch: {:?}", e);
                }
            }
            popover.popdown();
        }
    ));

    btn_save_png.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                if let Err(e) = save_sketch_png(&window, &*state.borrow()) {
                    error!("Failed to save sketch as PNG: {:?}", e);
                }
            }
            popover.popdown();
        }
    ));

    btn_save_grid.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Err(e) = save_grids(&*state.borrow()) {
                error!("Failed to save grids: {:?}", e);
            }
            popover.popdown();
        }
    ));

    btn_save_main_png.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                if let Err(e) = save_main_grid_png(&window, &*state.borrow()) {
                    error!("Failed to save main grid as PNG: {:?}", e);
                }
            }
            popover.popdown();
        }
    ));

    btn_copy.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                if let Err(e) = copy_to_clipboard(&window, &*state.borrow()) {
                    error!("Failed to copy sketch to clipboard: {:?}", e);
                }
            }
            popover.popdown();
        }
    ));

    btn_copy_main.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                if let Err(e) = copy_main_grid_to_clipboard(&window, &*state.borrow()) {
                    error!("Failed to copy main grid to clipboard: {:?}", e);
                }
            }
            popover.popdown();
        }
    ));

    btn_clear.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            let mut should_redraw = false;

            {
                let mut s = state.borrow_mut();
                if !s.strokes.is_empty() {
                    let strokes = std::mem::take(&mut s.strokes);
                    s.history.push(Action::Clear(strokes));
                    s.redo_history.clear();
                    s.needs_full_redraw = true; 
                    should_redraw = true;
                }
            } 

            if should_redraw {
                drawing_area.queue_draw();
            }
            
            popover.popdown();
        }
    ));

    btn_hide.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                window.set_visible(false);
            }
            popover.popdown();
        }
    ));

    btn_quit.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                window.close();
            }
        }
    ));

    popover.connect_closed(glib::clone!(
        #[strong]
        state,
        move |_| {
            state.borrow().config.save();
        }
    ));

    let scrolled_window = gtk::ScrolledWindow::new();
    scrolled_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    
    // We can safely increase max_height slightly to comfortably accommodate the native picker
    scrolled_window.set_max_content_height(500);
    scrolled_window.set_propagate_natural_height(true);
    
    scrolled_window.set_child(Some(&menu_box));
    popover.set_child(Some(&scrolled_window));

    popover
}
