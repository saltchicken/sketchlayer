use gtk::prelude::*;
use gtk::{DrawingArea, GestureDrag, GestureStylus, Popover, glib};
use gtk4 as gtk;
use std::cell::RefCell;
use std::rc::Rc;

use crate::state::app_state::AppState;
use crate::state::geometry::{Action, EraseMode};

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
            // Accurately extract the button that triggered the event,
            // falling back to the gesture's state if not a ButtonEvent.
            let mut button = gesture.current_button();
            if let Some(event) = gesture.current_event() {
                if let Some(btn_event) = event.downcast_ref::<gtk::gdk::ButtonEvent>() {
                    button = btn_event.button();
                }
            }

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
                state.active_button = 0;
                state.current_stroke = None;
                return;
            }

            let mut state = state.borrow_mut();

            // Prevent secondary inputs from interfering if we are already in an action
            if state.current_stroke.is_some() || state.is_erasing {
                return;
            }
            
            state.active_button = button;

            // Button 2 (Usually upper barrel / middle-click) -> Eraser
            let is_eraser_tool = button == 2
                || modifiers.contains(gtk::gdk::ModifierType::BUTTON2_MASK)
                || modifiers.contains(gtk::gdk::ModifierType::BUTTON5_MASK)
                || gesture
                    .device_tool()
                    .map_or(false, |t| t.tool_type() == gtk::gdk::DeviceToolType::Eraser);

            let erase_mode = state.erase_mode;
            let (cx, cy) = state.screen_to_canvas(x, y);
            let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);

            if is_eraser_tool && erase_mode == EraseMode::Vector {
                state.is_erasing = true;
                state.current_erased.clear();
                
                // Only immediately erase if making contact (pressure > 0.0)
                if pressure > 0.0 {
                    if state.erase_at(cx, cy) {
                        drawing_area.queue_draw();
                    }
                }
            } else {
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
            let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);

            if s.is_erasing {
                // Only erase if making contact (pressure > 0.0)
                if pressure > 0.0 {
                    if s.erase_at(cx, cy) {
                        drawing_area.queue_draw();
                    }
                }
            } else if s.current_stroke.is_some() {
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
        move |gesture, _x, _y| {
            let mut s = state.borrow_mut();
            
            // Bypass GestureSingle's tracker and inspect the raw event 
            // to accurately identify the physical button that was released.
            let mut released_button = gesture.current_button();
            if let Some(event) = gesture.current_event() {
                if let Some(btn_event) = event.downcast_ref::<gtk::gdk::ButtonEvent>() {
                    released_button = btn_event.button();
                }
            }

            // Ignore releases from explicitly different buttons, but allow `0`.
            // GTK frequently reports button 0 when the tip is lifted / sequence ends.
            if s.active_button != 0 && released_button != 0 && s.active_button != released_button {
                return;
            }
            
            s.active_button = 0;

            if s.is_erasing {
                s.is_erasing = false;
                if !s.current_erased.is_empty() {
                    let erased = std::mem::take(&mut s.current_erased);
                    s.history.push(Action::Erase(erased));
                    s.redo_history.clear();
                    s.cap_history();
                }
            } else if s.current_stroke.is_some() {
                s.end_stroke();
                drawing_area.queue_draw();
            }
        }
    ));

    drawing_area.add_controller(stylus);

    // Track Stylus and Pointer Hover Motion
    let hover_controller = gtk::EventControllerMotion::new();
    hover_controller.connect_motion(glib::clone!(
        #[strong] state,
        #[weak] drawing_area,
        move |_controller, x, y| {
            let mut s = state.borrow_mut();
            if s.config.show_vanishing_points {
                let (cx, cy) = s.screen_to_canvas(x, y);
                s.hover_pos = Some((cx, cy));
                drawing_area.queue_draw();
            }
        }
    ));

    hover_controller.connect_leave(glib::clone!(
        #[strong] state,
        #[weak] drawing_area,
        move |_controller| {
            let mut s = state.borrow_mut();
            if s.hover_pos.is_some() {
                s.hover_pos = None;
                drawing_area.queue_draw();
            }
        }
    ));

    drawing_area.add_controller(hover_controller);
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
