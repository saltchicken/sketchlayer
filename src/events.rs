use gtk::prelude::*;
use gtk::{ApplicationWindow, DrawingArea, EventControllerKey, GestureDrag, GestureStylus, Popover, gdk, glib};
use gtk4 as gtk;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::error;

use crate::render::{copy_main_grid_to_clipboard, copy_to_clipboard, save_grids, save_sketch};
use crate::state::AppState;

pub fn setup_stylus_events(
    drawing_area: &DrawingArea,
    state: Rc<RefCell<AppState>>,
    popover: Popover,
) {
    let stylus = GestureStylus::new();
    stylus.set_button(0);

    stylus.connect_down(glib::clone!(
        #[strong]
        state,
        #[weak]
        drawing_area,
        #[weak]
        popover,
        move |gesture, x, y| {
            let button = gesture.current_button();
            let modifiers = gesture
                .current_event()
                .map(|e| e.modifier_state())
                .unwrap_or(gtk::gdk::ModifierType::empty());

            // Button 3 (Usually lower barrel / right-click) -> Context Menu
            if button == 3 || modifiers.contains(gtk::gdk::ModifierType::BUTTON3_MASK) {
                popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                popover.popup();

                let mut state = state.borrow_mut();
                state.is_erasing = false;
                state.current_stroke = None;
                return;
            }

            // Button 2 (Usually upper barrel / middle-click) -> Eraser
            let is_eraser_tool = button == 2
                || modifiers.contains(gtk::gdk::ModifierType::BUTTON2_MASK)
                || modifiers.contains(gtk::gdk::ModifierType::BUTTON5_MASK)
                || gesture
                    .device_tool()
                    .map_or(false, |t| t.tool_type() == gtk::gdk::DeviceToolType::Eraser);

            let mut state = state.borrow_mut();
            let erase_mode = state.erase_mode;
            let (cx, cy) = state.screen_to_canvas(x, y);

            if is_eraser_tool && erase_mode == crate::state::EraseMode::Vector {
                state.is_erasing = true;
                state.current_erased.clear();
                if state.erase_at(cx, cy) {
                    drawing_area.queue_draw();
                }
            } else {
                let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
                state.start_stroke(cx, cy, pressure, is_eraser_tool);
            }
        }
    ));

    stylus.connect_motion(glib::clone!(
        #[strong]
        state,
        #[weak]
        drawing_area,
        move |gesture, x, y| {
            let mut s = state.borrow_mut();
            let (cx, cy) = s.screen_to_canvas(x, y);

            if s.is_erasing {
                if s.erase_at(cx, cy) {
                    drawing_area.queue_draw();
                }
            } else if s.current_stroke.is_some() {
                let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
                s.continue_stroke(cx, cy, pressure);
                drawing_area.queue_draw();
            }
        }
    ));

    stylus.connect_up(glib::clone!(
        #[strong]
        state,
        #[weak]
        drawing_area,
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

pub fn setup_view_events(
    drawing_area: &DrawingArea,
    state: Rc<RefCell<AppState>>,
) {
    // --- SCROLL WHEEL = ZOOM ---
    let scroll_controller = gtk::EventControllerScroll::new(
        gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::HORIZONTAL,
    );

    let weak_da = drawing_area.downgrade();

    scroll_controller.connect_scroll(glib::clone!(
        #[strong] state,
        move |controller, dx, dy| {
            let Some(drawing_area) = weak_da.upgrade() else {
                return glib::Propagation::Proceed;
            };

            let mut s = state.borrow_mut();
            
            let (focal_x, focal_y) = if let Some(event) = controller.current_event() {
                event.position().unwrap_or((drawing_area.width() as f64 / 2.0, drawing_area.height() as f64 / 2.0))
            } else {
                (drawing_area.width() as f64 / 2.0, drawing_area.height() as f64 / 2.0)
            };

            // Combine dx and dy so zoom works regardless of scroll wheel direction mapping
            let amount = dx + dy; 
            if amount != 0.0 {
                let zoom_factor = if amount > 0.0 { 0.9 } else { 1.1 };
                let new_zoom = s.zoom * zoom_factor;
                s.set_zoom(new_zoom, focal_x, focal_y);
            }

            drawing_area.queue_draw();
            glib::Propagation::Stop
        }
    ));

    drawing_area.add_controller(scroll_controller);

    // --- MIDDLE CLICK = PAN ---
    let drag_controller = GestureDrag::new();
    drag_controller.set_button(2); // Button 2 is the Middle Mouse Button

    let pan_start = Rc::new(RefCell::new((0.0, 0.0)));

    drag_controller.connect_drag_begin(glib::clone!(
        #[strong] state,
        #[strong] pan_start,
        move |gesture, _start_x, _start_y| {
            // Ignore panning requests if the device is a stylus (which frees button 2 for the eraser)
            if gesture.current_event().and_then(|e| e.device_tool()).is_some() {
                gesture.set_state(gtk::EventSequenceState::Denied);
                return;
            }

            let s = state.borrow();
            // Store the initial canvas offset when the drag starts
            *pan_start.borrow_mut() = (s.offset_x, s.offset_y);
        }
    ));

    let weak_da_drag = drawing_area.downgrade();
    drag_controller.connect_drag_update(glib::clone!(
        #[strong] state,
        #[strong] pan_start,
        move |gesture, offset_x, offset_y| {
            // Ignore updates if a stylus is being used
            if gesture.current_event().and_then(|e| e.device_tool()).is_some() {
                return;
            }

            let Some(drawing_area) = weak_da_drag.upgrade() else {
                return;
            };
            
            let mut s = state.borrow_mut();
            let start = *pan_start.borrow();
            
            // Add the drag offset to the original canvas offset
            s.offset_x = start.0 + offset_x;
            s.offset_y = start.1 + offset_y;
            s.needs_full_redraw = true;
            
            drawing_area.queue_draw();
        }
    ));

    drawing_area.add_controller(drag_controller);
}

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
                // SWAPPED: Shift + Ctrl + C now copies the full sketch, while Ctrl + C copies the main grid
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
