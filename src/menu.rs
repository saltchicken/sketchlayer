use gtk4::prelude::*;
use gtk4::{Popover, Box as GtkBox, Button, Orientation, ColorButton};
use std::rc::Rc;
use std::cell::RefCell;
use crate::state::{AppState, EraseMode};

pub fn show_context_menu(parent: &gtk4::DrawingArea, x: i32, y: i32, state: Rc<RefCell<AppState>>) {
    let popover = Popover::builder()
        .position(gtk4::PositionType::Bottom)
        .has_arrow(false)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 5);
    
    let btn_draw = Button::with_label("Draw");
    let btn_erase_vec = Button::with_label("Vector Erase");
    let btn_erase_pix = Button::with_label("Pixel Erase");
    let color_btn = ColorButton::new();
    
    let s_clone = state.clone();
    color_btn.connect_color_set(move |cb| {
        let rgba = cb.rgba();
        let mut s = s_clone.borrow_mut();
        s.current_color = (rgba.red() as f64, rgba.green() as f64, rgba.blue() as f64);
    });

    let s_clone = state.clone();
    btn_draw.connect_clicked(move |_| {
        let mut s = s_clone.borrow_mut();
        s.is_erasing = false;
    });

    let s_clone = state.clone();
    btn_erase_vec.connect_clicked(move |_| {
        let mut s = s_clone.borrow_mut();
        s.is_erasing = true;
        s.erase_mode = EraseMode::Vector;
    });

    let s_clone = state.clone();
    btn_erase_pix.connect_clicked(move |_| {
        let mut s = s_clone.borrow_mut();
        s.is_erasing = true;
        s.erase_mode = EraseMode::Pixel;
    });

    let btn_clear = Button::with_label("Clear Canvas");
    let s_clone = state.clone();
    let parent_clone = parent.clone();
    btn_clear.connect_clicked(move |_| {
        let mut s = s_clone.borrow_mut();
        let strokes = s.strokes.clone();
        s.undo_stack.push(crate::state::Action::Clear(strokes.into_iter().map(|s| (*s).clone().into()).collect()));
        s.strokes.clear();
        s.needs_full_redraw = true;
        parent_clone.queue_draw();
    });

    vbox.append(&btn_draw);
    vbox.append(&btn_erase_vec);
    vbox.append(&btn_erase_pix);
    vbox.append(&color_btn);
    vbox.append(&btn_clear);

    popover.set_child(Some(&vbox));
    popover.set_parent(parent);
    
    let rect = gdk4::Rectangle::new(x, y, 1, 1);
    popover.set_pointing_to(Some(&rect));
    
    popover.popup();
}
