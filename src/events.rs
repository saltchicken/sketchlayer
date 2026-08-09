use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{ApplicationWindow, DrawingArea, EventControllerKey, GestureStylus, Popover, gdk, glib};
use std::cell::RefCell;
use std::rc::Rc;

use crate::render::{save_sketch, save_grids};
use crate::state::AppState;

pub fn setup_stylus_events(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>, popover: Popover) {
    let stylus = GestureStylus::new();
    stylus.set_button(0);

    stylus.connect_down(glib::clone!(
        #[strong] state,
        #[weak] drawing_area,
        #[weak] popover,
        move |gesture, x, y| {
            let button = gesture.current_button();
            let modifiers = gesture
                .current_event()
                .map(|e| e.modifier_state())
                .unwrap_or(gtk::gdk::ModifierType::empty());

            if button == 3 || modifiers.contains(gtk::gdk::ModifierType::BUTTON3_MASK) {
                popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                popover.popup();

                let mut state = state.borrow_mut();
                state.is_erasing = false;
                state.current_stroke = None;
                return;
            }

            let is_eraser_tool = button != 1
                || modifiers.contains(gtk::gdk::ModifierType::BUTTON2_MASK)
                || modifiers.contains(gtk::gdk::ModifierType::BUTTON4_MASK)
                || modifiers.contains(gtk::gdk::ModifierType::BUTTON5_MASK)
                || gesture
                    .device_tool()
                    .map_or(false, |t| t.tool_type() == gtk::gdk::DeviceToolType::Eraser);

            let mut state = state.borrow_mut();
            let erase_mode = state.erase_mode;

            if is_eraser_tool && erase_mode == crate::state::EraseMode::Vector {
                state.is_erasing = true;
                state.current_erased.clear();
                if state.erase_at(x, y) {
                    drawing_area.queue_draw();
                }
            } else {
                let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
                state.start_stroke(x, y, pressure, is_eraser_tool);
            }
        }
    ));

    stylus.connect_motion(glib::clone!(
        #[strong] state,
        #[weak] drawing_area,
        move |gesture, x, y| {
            let mut s = state.borrow_mut();

            if s.is_erasing { 
                if s.erase_at(x, y) {
                    drawing_area.queue_draw();
                }
            } else if s.current_stroke.is_some() { 
                let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
                s.continue_stroke(x, y, pressure);
                drawing_area.queue_draw();
            }
        }
    ));

    stylus.connect_up(glib::clone!(
        #[strong] state,
        #[weak] drawing_area,
        move |_gesture, _x, _y| {
            let mut s = state.borrow_mut();
            
            if s.is_erasing { 
                s.is_erasing = false;
                if !s.current_erased.is_empty() {
                    let erased = std::mem::take(&mut s.current_erased);
                    s.history.push(crate::state::Action::Erase(erased));
                    s.redo_history.clear();
                }
            } else if s.current_stroke.is_some() { 
                s.end_stroke();
                drawing_area.queue_draw();
            }
        }
    ));

    drawing_area.add_controller(stylus);
}

pub fn setup_keyboard_events(window: &ApplicationWindow, drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) {
    let key_controller = EventControllerKey::new();

    key_controller.connect_key_pressed(glib::clone!(
        #[strong] window,
        #[strong] drawing_area,
        #[strong] state,
        move |_ctrl, key, _keycode, modifier_state| {
            if key == gdk::Key::Escape {
                window.set_visible(false);
                return glib::Propagation::Stop;
            }

            if (key == gdk::Key::q || key == gdk::Key::Q)
                && modifier_state.contains(gdk::ModifierType::CONTROL_MASK)
            {
                window.close();
                return glib::Propagation::Stop;
            }

            if (key == gdk::Key::s || key == gdk::Key::S)
                && modifier_state.contains(gdk::ModifierType::CONTROL_MASK)
            {
                if modifier_state.contains(gdk::ModifierType::SHIFT_MASK) {
                    save_grids(&state.borrow());
                } else {
                    save_sketch(&window, &state.borrow());
                }
                return glib::Propagation::Stop;
            }

            if key == gdk::Key::z || key == gdk::Key::Z {
                if modifier_state.contains(gdk::ModifierType::CONTROL_MASK) {
                    let mut state_mut = state.borrow_mut();
                    
                    if modifier_state.contains(gdk::ModifierType::SHIFT_MASK) {
                        if state_mut.redo() {
                            drawing_area.queue_draw();
                        }
                    } else {
                        if state_mut.undo() {
                            drawing_area.queue_draw();
                        }
                    }
                    return glib::Propagation::Stop;
                }
            }

            glib::Propagation::Proceed
        }
    ));

    window.add_controller(key_controller);
}
