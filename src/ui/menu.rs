// src/ui/menu.rs
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, DrawingArea, Orientation, Popover, glib};
use gtk4 as gtk;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{error, info};

use crate::render::clipboard::{copy_active_frame_to_clipboard, copy_to_clipboard};
use crate::render::export::{save_frames, save_active_frame_png, save_sketch, save_sketch_png};
use crate::state::app_state::AppState;
use crate::state::geometry::{Action, EraseMode};

pub fn build_context_menu(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) -> Popover {
    let popover = Popover::new();
    popover.set_parent(drawing_area);
    popover.set_has_arrow(true);

    let menu_box = gtk::Box::new(Orientation::Vertical, 8);
    menu_box.set_margin_top(4);
    menu_box.set_margin_bottom(4);
    menu_box.set_margin_start(4);
    menu_box.set_margin_end(4);

    // Group 1: Tools & Brushes
    let tools_expander = gtk::Expander::new(Some("Tools & Brushes"));
    tools_expander.set_expanded(true);
    let tools_box = gtk::Box::new(Orientation::Vertical, 4);
    tools_box.set_margin_start(8);
    tools_box.set_margin_top(4);
    tools_expander.set_child(Some(&tools_box));

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
        
        // Wrap the chooser and a back button inside a vertical box
        let color_chooser_box = gtk::Box::new(Orientation::Vertical, 4);
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

        // Add a back button to exit the custom color editor
        let back_btn = gtk::Button::with_label("Back to Palette");
        back_btn.connect_clicked(glib::clone!(
            #[weak] color_chooser,
            move |_| {
                color_chooser.set_property("show-editor", false);
            }
        ));

        // Bind the visibility of the back button to whether the editor is shown
        color_chooser
            .bind_property("show-editor", &back_btn, "visible")
            .sync_create()
            .build();

        color_chooser_box.append(&back_btn);
        color_chooser_box.append(&color_chooser);

        color_popover.set_child(Some(&color_chooser_box));
        color_menu_btn.set_popover(Some(&color_popover));
        
        custom_color_box.append(&custom_color_label);
        custom_color_box.append(&color_menu_btn);
        tools_box.append(&custom_color_box);
    }

    let btn_erase_mode = Button::with_label("Erase Mode: Pixel");
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

    let pen_pressure_box = gtk::Box::new(Orientation::Horizontal, 8);
    pen_pressure_box.set_margin_start(4);
    pen_pressure_box.set_margin_end(4);
    let pen_pressure_label = gtk::Label::new(Some("Pen Pressure:"));
    let pen_pressure_scale = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 50.0, 0.1);
    pen_pressure_scale.set_digits(1);
    pen_pressure_scale.set_value(state.borrow().config.pen_pressure_mult);
    pen_pressure_scale.set_draw_value(true);
    pen_pressure_scale.set_hexpand(true);
    pen_pressure_box.append(&pen_pressure_label);
    pen_pressure_box.append(&pen_pressure_scale);

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

    let eraser_pressure_box = gtk::Box::new(Orientation::Horizontal, 8);
    eraser_pressure_box.set_margin_start(4);
    eraser_pressure_box.set_margin_end(4);
    let eraser_pressure_label = gtk::Label::new(Some("Eraser Pressure:"));
    let eraser_pressure_scale = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 0.1);
    eraser_pressure_scale.set_digits(1);
    eraser_pressure_scale.set_value(state.borrow().config.eraser_pressure_mult);
    eraser_pressure_scale.set_draw_value(true);
    eraser_pressure_scale.set_hexpand(true);
    eraser_pressure_box.append(&eraser_pressure_label);
    eraser_pressure_box.append(&eraser_pressure_scale);

    tools_box.append(&btn_erase_mode);
    tools_box.append(&pen_width_box);
    tools_box.append(&pen_pressure_box);
    tools_box.append(&eraser_width_box);
    tools_box.append(&eraser_pressure_box);


    // Group 2: View & Guides
    let view_expander = gtk::Expander::new(Some("View & Guides"));
    let view_box = gtk::Box::new(Orientation::Vertical, 4);
    view_box.set_margin_start(8);
    view_box.set_margin_top(4);
    view_expander.set_child(Some(&view_box));

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

    let btn_bg = Button::with_label("Toggle Background");
    let btn_frames = Button::with_label("Toggle Frame Guides");
    let btn_vp = Button::with_label("Toggle Vanishing Points");
    let btn_vp_lines = Button::with_label("Toggle Perspective Grid");
    
    let current_persp_mode = state.borrow().config.perspective_mode.clamp(1, 3);
    let btn_persp_mode = Button::with_label(&format!("Perspective: {}-Point", current_persp_mode));
    
    let btn_reset_view = Button::with_label("Reset View");

    let vp_angle_box = gtk::Box::new(Orientation::Horizontal, 8);
    vp_angle_box.set_margin_start(4);
    vp_angle_box.set_margin_end(4);
    let vp_angle_label = gtk::Label::new(Some("Grid Angle:"));
    let vp_angle_scale = gtk::Scale::with_range(Orientation::Horizontal, 1.0, 90.0, 1.0);
    vp_angle_scale.set_digits(1);
    vp_angle_scale.set_value(state.borrow().config.vp_line_angle_step);
    vp_angle_scale.set_draw_value(true);
    vp_angle_scale.set_hexpand(true);
    vp_angle_box.append(&vp_angle_label);
    vp_angle_box.append(&vp_angle_scale);

    view_box.append(&opacity_box);
    view_box.append(&btn_bg);
    view_box.append(&btn_frames);
    view_box.append(&btn_vp);
    view_box.append(&btn_vp_lines);
    view_box.append(&btn_persp_mode);
    view_box.append(&vp_angle_box);
    view_box.append(&btn_reset_view);


    // Group 3: Edit Canvas
    let edit_expander = gtk::Expander::new(Some("Edit Canvas"));
    let edit_box = gtk::Box::new(Orientation::Vertical, 4);
    edit_box.set_margin_start(8);
    edit_box.set_margin_top(4);
    edit_expander.set_child(Some(&edit_box));

    let btn_undo = Button::with_label("Undo");
    let btn_redo = Button::with_label("Redo");
    let btn_clear = Button::with_label("Clear Canvas");

    edit_box.append(&btn_undo);
    edit_box.append(&btn_redo);
    edit_box.append(&btn_clear);


    // Group 4: Save & Export
    let export_expander = gtk::Expander::new(Some("Save & Export"));
    let export_box = gtk::Box::new(Orientation::Vertical, 4);
    export_box.set_margin_start(8);
    export_box.set_margin_top(4);
    export_expander.set_child(Some(&export_box));

    let btn_save_state = Button::with_label("Save State (.sketchlayer)");
    let btn_save = Button::with_label("Save Full Sketch (SVG)");
    let btn_save_png = Button::with_label("Save Full Sketch (PNG)");
    let btn_save_frames = Button::with_label("Save All Frames (SVG)");
    let btn_save_active_frame_png = Button::with_label("Save Active Frame (PNG)");
    let btn_copy = Button::with_label("Copy Full Screen");
    let btn_copy_active_frame = Button::with_label("Copy Active Frame");

    export_box.append(&btn_save_state);
    export_box.append(&btn_save);
    export_box.append(&btn_save_png);
    export_box.append(&btn_save_frames);
    export_box.append(&btn_save_active_frame_png);
    export_box.append(&btn_copy);
    export_box.append(&btn_copy_active_frame);


    // Group 5: Application
    let app_expander = gtk::Expander::new(Some("Application"));
    let app_box = gtk::Box::new(Orientation::Vertical, 4);
    app_box.set_margin_start(8);
    app_box.set_margin_top(4);
    app_expander.set_child(Some(&app_box));

    let btn_hide = Button::with_label("Hide Overlay");
    let btn_quit = Button::with_label("Quit");

    app_box.append(&btn_hide);
    app_box.append(&btn_quit);


    // Append all groups to the main menu
    menu_box.append(&tools_expander);
    menu_box.append(&view_expander);
    menu_box.append(&edit_expander);
    menu_box.append(&export_expander);
    menu_box.append(&app_expander);

    // Event Connections
    btn_erase_mode.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |btn| {
            let mut s = state.borrow_mut();
            if s.erase_mode == EraseMode::Pixel {
                s.erase_mode = EraseMode::Vector;
                btn.set_label("Erase Mode: Vector");
            } else {
                s.erase_mode = EraseMode::Pixel;
                btn.set_label("Erase Mode: Pixel");
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

    btn_frames.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.config.show_frames = !s.config.show_frames;
            }
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    btn_vp.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.config.show_vanishing_points = !s.config.show_vanishing_points;
                if !s.config.show_vanishing_points {
                    s.hover_pos = None;
                }
            }
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    btn_vp_lines.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.config.show_vanishing_point_lines = !s.config.show_vanishing_point_lines;
            }
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    btn_persp_mode.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[strong] state,
        move |btn| {
            {
                let mut s = state.borrow_mut();
                let mut mode = s.config.perspective_mode + 1;
                if mode > 3 { mode = 1; }
                s.config.perspective_mode = mode;
                btn.set_label(&format!("Perspective: {}-Point", mode));
            }
            drawing_area.queue_draw();
        }
    ));

    vp_angle_scale.connect_value_changed(glib::clone!(
        #[weak]
        drawing_area,
        #[strong]
        state,
        move |scale| {
            {
                let mut s = state.borrow_mut();
                s.config.vp_line_angle_step = scale.value();
            }
            drawing_area.queue_draw();
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

    pen_pressure_scale.connect_value_changed(glib::clone!(
        #[strong]
        state,
        move |scale| {
            let mut s = state.borrow_mut();
            s.config.pen_pressure_mult = scale.value();
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

    eraser_pressure_scale.connect_value_changed(glib::clone!(
        #[strong]
        state,
        move |scale| {
            let mut s = state.borrow_mut();
            s.config.eraser_pressure_mult = scale.value();
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

    btn_save_frames.connect_clicked(glib::clone!(
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Err(e) = save_frames(&*state.borrow()) {
                error!("Failed to save frames: {:?}", e);
            }
            popover.popdown();
        }
    ));

    btn_save_active_frame_png.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                if let Err(e) = save_active_frame_png(&window, &*state.borrow()) {
                    error!("Failed to save active frame as PNG: {:?}", e);
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

    btn_copy_active_frame.connect_clicked(glib::clone!(
        #[weak]
        drawing_area,
        #[weak]
        popover,
        #[strong]
        state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast::<ApplicationWindow>() {
                if let Err(e) = copy_active_frame_to_clipboard(&window, &*state.borrow()) {
                    error!("Failed to copy active frame to clipboard: {:?}", e);
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
                    s.cap_history();
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
    scrolled_window.set_max_content_height(600); 
    scrolled_window.set_propagate_natural_height(true);

    // Force a wider minimum width (e.g., 350 pixels)
    scrolled_window.set_min_content_width(350);
    
    scrolled_window.set_child(Some(&menu_box));
    popover.set_child(Some(&scrolled_window));

    popover
}
