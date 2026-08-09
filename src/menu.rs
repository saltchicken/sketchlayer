use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Button, CssProvider, DrawingArea, Orientation, Popover, glib,
};
use gtk4 as gtk;
use std::cell::RefCell;
use std::rc::Rc;

use crate::render::save_sketch;
use crate::state::AppState;

fn create_color_button(name: &str, color_val: (f64, f64, f64), state: Rc<RefCell<AppState>>) -> Button {
    let btn = Button::builder().tooltip_text(name).build();
    let (r, g, b) = color_val;
    
    let css = format!(
        "button {{ background: rgba({}, {}, {}, 1.0); min-width: 24px; min-height: 24px; border-radius: 12px; }}",
        (r * 255.0) as i32, (g * 255.0) as i32, (b * 255.0) as i32
    );

    let provider = CssProvider::new();
    provider.load_from_data(&css);
    btn.style_context().add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    btn.connect_clicked(glib::clone!(
        #[strong] state,
        move |_| {
            state.borrow_mut().current_color = color_val;
        }
    ));
    
    btn
}

pub fn build_context_menu(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) -> Popover {
    let popover = Popover::new();
    popover.set_parent(drawing_area);
    popover.set_has_arrow(true);

    let menu_box = gtk::Box::new(Orientation::Vertical, 4);
    menu_box.set_margin_top(4);
    menu_box.set_margin_bottom(4);
    menu_box.set_margin_start(4);
    menu_box.set_margin_end(4);
    popover.set_child(Some(&menu_box));

    let color_box = gtk::Box::new(Orientation::Horizontal, 4);
    color_box.set_halign(gtk::Align::Center);
    color_box.set_margin_bottom(8);

    let colors = [
        ("White", (1.0, 1.0, 1.0)),
        ("Red", (1.0, 0.2, 0.2)),
        ("Green", (0.2, 1.0, 0.2)),
        ("Blue", (0.2, 0.5, 1.0)),
        ("Yellow", (1.0, 1.0, 0.2)),
        ("Black", (0.0, 0.0, 0.0)),
    ];

    for (name, color_val) in colors {
        color_box.append(&create_color_button(name, color_val, state.clone()));
    }
    menu_box.append(&color_box);

    let btn_erase_mode = Button::with_label("Erase Mode: Vector");
    let btn_undo = Button::with_label("Undo");
    let btn_redo = Button::with_label("Redo");
    let btn_bg = Button::with_label("Toggle Background");
    let btn_grid = Button::with_label("Toggle Grid");

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

    let btn_save = Button::with_label("Save Sketch");
    let btn_clear = Button::with_label("Clear Canvas");
    let btn_hide = Button::with_label("Hide Overlay");
    let btn_quit = Button::with_label("Quit");

    menu_box.append(&btn_erase_mode);
    menu_box.append(&btn_undo);
    menu_box.append(&btn_redo);
    menu_box.append(&btn_bg);
    menu_box.append(&btn_grid);
    menu_box.append(&opacity_box);
    menu_box.append(&btn_save);
    menu_box.append(&btn_clear);
    menu_box.append(&btn_hide);
    menu_box.append(&btn_quit);

    btn_erase_mode.connect_clicked(glib::clone!(
        #[strong] state,
        move |btn| {
            let mut s = state.borrow_mut();
            if s.erase_mode == crate::state::EraseMode::Vector {
                s.erase_mode = crate::state::EraseMode::Pixel;
                btn.set_label("Erase Mode: Pixel");
            } else {
                s.erase_mode = crate::state::EraseMode::Vector;
                btn.set_label("Erase Mode: Vector");
            }
        }
    ));

    btn_undo.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[strong] state,
        move |_| {
            if state.borrow_mut().undo() {
                drawing_area.queue_draw();
            }
        }
    ));

    btn_redo.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[strong] state,
        move |_| {
            if state.borrow_mut().redo() {
                drawing_area.queue_draw();
            }
        }
    ));

    btn_bg.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[weak] popover,
        #[strong] state,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.white_background = !s.white_background;
            }
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    btn_grid.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[weak] popover,
        #[strong] state,
        move |_| {
            {
                let mut s = state.borrow_mut();
                s.show_grid = !s.show_grid;
            }
            drawing_area.queue_draw();
            popover.popdown();
        }
    ));

    opacity_scale.connect_value_changed(glib::clone!(
        #[weak] drawing_area,
        move |scale| {
            if let Some(window) = drawing_area.root().and_downcast_ref::<ApplicationWindow>() {
                window.set_opacity(scale.value() / 100.0);
            }
        }
    ));

    btn_save.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[weak] popover,
        #[strong] state,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast_ref::<ApplicationWindow>() {
                save_sketch(&window, &state.borrow());
            }
            popover.popdown();
        }
    ));

    btn_clear.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[weak] popover,
        #[strong] state,
        move |_| {
            let mut s = state.borrow_mut();
            if !s.strokes.is_empty() {
                let strokes = std::mem::take(&mut s.strokes);
                s.history.push(crate::state::Action::Clear(strokes));
                s.redo_history.clear();
                drawing_area.queue_draw();
            }
            popover.popdown();
        }
    ));

    btn_hide.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        #[weak] popover,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast_ref::<ApplicationWindow>() {
                window.set_visible(false);
            }
            popover.popdown();
        }
    ));

    btn_quit.connect_clicked(glib::clone!(
        #[weak] drawing_area,
        move |_| {
            if let Some(window) = drawing_area.root().and_downcast_ref::<ApplicationWindow>() {
                window.close();
            }
        }
    ));

    popover
}
