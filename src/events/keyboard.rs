use gtk::prelude::*;
use gtk::{ApplicationWindow, DrawingArea, EventControllerKey, gdk, glib};
use gtk4 as gtk;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::error;

use crate::render::clipboard::{copy_main_grid_to_clipboard, copy_to_clipboard};
use crate::render::export::{save_grids, save_sketch};
use crate::state::app_state::AppState;

pub fn setup_keyboard_events(
    window: &ApplicationWindow,
    drawing_area: &DrawingArea,
    state: Rc<RefCell<AppState>>,
) {
    let key_controller = EventControllerKey::new();

    key_controller.connect_key_pressed(glib::clone!(
        #[strong]
        window,
        #[strong]
        drawing_area,
        #[strong]
        state,
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
                    if let Err(e) = save_grids(&*state.borrow()) {
                        error!("Failed to save grids from shortcut: {:?}", e);
                    }
                } else {
                    if let Err(e) = save_sketch(&window, &*state.borrow()) {
                        error!("Failed to save sketch from shortcut: {:?}", e);
                    }
                }
                return glib::Propagation::Stop;
            }

            if (key == gdk::Key::c || key == gdk::Key::C)
                && modifier_state.contains(gdk::ModifierType::CONTROL_MASK)
            {
                if modifier_state.contains(gdk::ModifierType::SHIFT_MASK) {
                    if let Err(e) = copy_to_clipboard(&window, &*state.borrow()) {
                        error!("Failed to copy sketch to clipboard: {:?}", e);
                    }
                } else {
                    if let Err(e) = copy_main_grid_to_clipboard(&window, &*state.borrow()) {
                        error!("Failed to copy main grid to clipboard: {:?}", e);
                    }
                }
                return glib::Propagation::Stop;
            }

            if (key == gdk::Key::_0 || key == gdk::Key::KP_0)
                && modifier_state.contains(gdk::ModifierType::CONTROL_MASK)
            {
                state.borrow_mut().reset_view();
                drawing_area.queue_draw();
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
