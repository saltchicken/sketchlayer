use std::rc::Rc;
use std::cell::RefCell;
use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerMotion, GestureClick, EventControllerKey, EventControllerScroll, EventControllerScrollFlags};
use gdk4::Key;
use crate::state::{AppState, Stroke, Point, Action, EraseMode, BoundingBox};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn setup_events(area: &DrawingArea, state: Rc<RefCell<AppState>>, window: &gtk4::ApplicationWindow) {
    // Pen Motion Handling
    let motion = EventControllerMotion::new();
    let state_clone = state.clone();
    let area_clone = area.clone();
    motion.connect_motion(move |_, x, y| {
        let mut s = state_clone.borrow_mut();
        let pressure = 0.5; // Stub for extended GdkEvent pressure handling 
        let (cx, cy) = s.screen_to_canvas(x, y);

        // Pre-fetch config values to avoid simultaneous borrow conflicts
        let pen_width = s.config.base_pen_width;
        let eraser_width = s.config.base_eraser_width;

        if let Some(active) = s.active_stroke.as_mut() {
            active.points.push(Point { x: cx, y: cy, pressure });
            let padding = if active.is_eraser { eraser_width } else { pen_width };
            
            if let Some(bbox) = active.bbox.as_mut() {
                bbox.expand(cx, cy, padding);
            } else {
                active.bbox = Some(BoundingBox::new(cx, cy));
            }
            area_clone.queue_draw();
        }
    });
    area.add_controller(motion);

    // Click / Touch Start Handling
    let click = GestureClick::new();
    click.set_button(0); 
    let state_clone = state.clone();
    let area_clone = area.clone();
    
    click.connect_pressed(move |gesture, _, x, y| {
        let btn = gesture.current_button();
        let mut s = state_clone.borrow_mut();
        
        let is_eraser = s.is_erasing || btn == 2; 
        
        // Stylus Lower Barrel / Right Click Context Menu
        if btn == 3 {
            crate::menu::show_context_menu(&area_clone, x as i32, y as i32, state_clone.clone());
            return;
        }

        let (cx, cy) = s.screen_to_canvas(x, y);
        let padding = if is_eraser { s.config.base_eraser_width } else { s.config.base_pen_width };
        
        if is_eraser && s.erase_mode == EraseMode::Vector {
            let mut to_remove = Vec::new();
            let mut new_strokes = Vec::new();
            
            for stroke in &s.strokes {
                let mut hit = false;
                if let Some(bbox) = &stroke.bbox {
                    let e_bbox = BoundingBox {
                        min_x: cx - padding, max_x: cx + padding,
                        min_y: cy - padding, max_y: cy + padding,
                    };
                    if bbox.intersects(&e_bbox) {
                        for i in 0..stroke.points.len().saturating_sub(1) {
                            let p = Point { x: cx, y: cy, pressure: 1.0 };
                            let dist = p.distance_to_segment(&stroke.points[i], &stroke.points[i+1]);
                            if dist < padding {
                                hit = true;
                                break;
                            }
                        }
                    }
                }
                if hit { to_remove.push(stroke.clone()); } 
                else { new_strokes.push(stroke.clone()); }
            }
            
            if !to_remove.is_empty() {
                s.strokes = new_strokes;
                s.undo_stack.push(Action::Erase(to_remove));
                s.redo_stack.clear();
                s.needs_full_redraw = true;
                area_clone.queue_draw();
            }
        } else {
            let mut bbox = BoundingBox::new(cx, cy);
            bbox.expand(cx, cy, padding);
            
            s.active_stroke = Some(Stroke {
                id: s.next_stroke_id,
                points: vec![Point { x: cx, y: cy, pressure: 0.5 }],
                color: s.current_color,
                is_eraser,
                bbox: Some(bbox),
            });
            s.next_stroke_id += 1;
        }
    });

    let state_clone = state.clone();
    let area_clone = area.clone();
    click.connect_released(move |_, _, _, _| {
        let mut s = state_clone.borrow_mut();
        if let Some(stroke) = s.active_stroke.take() {
            let rc_stroke = Rc::new(stroke);
            s.strokes.push(rc_stroke.clone());
            s.undo_stack.push(Action::Draw(rc_stroke));
            s.redo_stack.clear();
            area_clone.queue_draw();
        }
    });
    area.add_controller(click);

    // Scroll / Pan
    let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::HORIZONTAL);
    let state_clone = state.clone();
    let area_clone = area.clone();
    scroll.connect_scroll(move |_, _dx, dy| {
        let mut s = state_clone.borrow_mut();
        let zoom_factor = if dy > 0.0 { 0.9 } else { 1.1 };
        s.zoom *= zoom_factor;
        s.needs_full_redraw = true;
        area_clone.queue_draw();
        glib::Propagation::Proceed
    });
    area.add_controller(scroll);

    // Keyboard Shortcuts
    let key = EventControllerKey::new();
    let state_clone = state.clone();
    let area_clone = area.clone();
    let window_clone = window.clone(); // Clone the GTK reference for the static closure
    
    key.connect_key_pressed(move |_, keyval, _, state_mask| {
        let mut s = state_clone.borrow_mut();
        let ctrl = state_mask.contains(gdk4::ModifierType::CONTROL_MASK);
        let shift = state_mask.contains(gdk4::ModifierType::SHIFT_MASK);

        match keyval {
            Key::Escape => { window_clone.hide(); }
            Key::q | Key::Q if ctrl => { std::process::exit(0); }
            Key::s | Key::S if ctrl && shift => {
                let p = crate::config::expand_tilde(&s.config.save_dir);
                crate::render::export_grids(&s, &p);
            }
            Key::s | Key::S if ctrl => {
                let p = crate::config::expand_tilde(&s.config.save_dir);
                std::fs::create_dir_all(&p).unwrap();
                let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let file = p.join(format!("sketch_{}.svg", time));
                crate::render::export_svg(&s, &file, area_clone.width() as f64, area_clone.height() as f64);
            }
            Key::z | Key::Z if ctrl && shift => {
                if let Some(action) = s.redo_stack.pop() {
                    match &action {
                        Action::Draw(stroke) => s.strokes.push(stroke.clone()),
                        Action::Erase(strokes) => s.strokes.retain(|st| !strokes.iter().any(|r| r.id == st.id)),
                        Action::Clear(_) => s.strokes.clear(),
                    }
                    s.undo_stack.push(action);
                    s.needs_full_redraw = true;
                    area_clone.queue_draw();
                }
            }
            Key::z | Key::Z if ctrl => {
                if let Some(action) = s.undo_stack.pop() {
                    match &action {
                        Action::Draw(stroke) => s.strokes.retain(|st| st.id != stroke.id),
                        Action::Erase(strokes) | Action::Clear(strokes) => s.strokes.extend(strokes.clone()),
                    }
                    s.redo_stack.push(action);
                    s.needs_full_redraw = true;
                    area_clone.queue_draw();
                }
            }
            Key::_0 if ctrl => {
                s.zoom = 1.0;
                s.offset_x = 0.0;
                s.offset_y = 0.0;
                s.needs_full_redraw = true;
                area_clone.queue_draw();
            }
            _ => {}
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key);
}
