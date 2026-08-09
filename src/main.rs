use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{gdk, glib, Application, ApplicationWindow, CssProvider, DrawingArea, GestureStylus, EventControllerKey};
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
    pressure: f64,
}

struct Stroke {
    points: Vec<Point>,
}

struct AppState {
    strokes: Vec<Stroke>,
    current_stroke: Option<Stroke>,
    is_erasing: bool,
}

impl AppState {
    // Helper to check collision and remove strokes that the eraser touches
    fn erase_at(&mut self, x: f64, y: f64) -> bool {
        let erase_radius = 15.0; // How close the stylus needs to be to delete a line
        let initial_len = self.strokes.len();

        self.strokes.retain(|stroke| {
            if stroke.points.is_empty() { return false; }
            if stroke.points.len() == 1 {
                let p = &stroke.points[0];
                return ((p.x - x).powi(2) + (p.y - y).powi(2)).sqrt() > erase_radius;
            }
            
            // Check distance from eraser to each line segment in the stroke
            for i in 0..(stroke.points.len() - 1) {
                let p1 = &stroke.points[i];
                let p2 = &stroke.points[i+1];
                
                let l2 = (p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2);
                let dist = if l2 == 0.0 {
                    ((x - p1.x).powi(2) + (y - p1.y).powi(2)).sqrt()
                } else {
                    let mut t = ((x - p1.x) * (p2.x - p1.x) + (y - p1.y) * (p2.y - p1.y)) / l2;
                    t = t.clamp(0.0, 1.0);
                    let proj_x = p1.x + t * (p2.x - p1.x);
                    let proj_y = p1.y + t * (p2.y - p1.y);
                    ((x - proj_x).powi(2) + (y - proj_y).powi(2)).sqrt()
                };

                if dist <= erase_radius {
                    return false; // Hit detected, remove stroke
                }
            }
            true // Keep stroke
        });

        // Return true if we actually deleted something so we can queue a redraw
        initial_len != self.strokes.len()
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.github.minimal_sketch")
        .build();

    app.connect_activate(|app| {
        let windows = app.windows();
        
        if let Some(window) = windows.first() {
            if window.is_visible() {
                window.set_visible(false);
            } else {
                window.set_visible(true);
                window.present();
            }
        } else {
            build_ui(app);
        }
    });

    app.run();
}

fn render_stroke(cr: &gtk::cairo::Context, stroke: &Stroke) {
    if stroke.points.len() < 2 { return; }

    if stroke.points.len() == 2 {
        let p1 = &stroke.points[0];
        let p2 = &stroke.points[1];
        
        cr.set_line_width(1.0 + (p1.pressure * 3.0)); 
        cr.set_source_rgba(1.0, 1.0, 1.0, p1.pressure.clamp(0.1, 1.0));
        
        cr.move_to(p1.x, p1.y);
        cr.line_to(p2.x, p2.y);
        cr.stroke().expect("Failed to stroke path");
        return;
    }

    let mut start_x = stroke.points[0].x;
    let mut start_y = stroke.points[0].y;

    for i in 1..(stroke.points.len() - 1) {
        let p_ctrl = &stroke.points[i];
        let p_next = &stroke.points[i + 1]; 

        let end_x = (p_ctrl.x + p_next.x) / 2.0;
        let end_y = (p_ctrl.y + p_next.y) / 2.0;

        let cp1_x = start_x + (2.0 / 3.0) * (p_ctrl.x - start_x);
        let cp1_y = start_y + (2.0 / 3.0) * (p_ctrl.y - start_y);
        let cp2_x = end_x + (2.0 / 3.0) * (p_ctrl.x - end_x);
        let cp2_y = end_y + (2.0 / 3.0) * (p_ctrl.y - end_y);

        cr.set_line_width(1.0 + (p_ctrl.pressure * 3.0));
        
        let alpha = p_ctrl.pressure.clamp(0.1, 1.0);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);

        cr.move_to(start_x, start_y);
        cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, end_x, end_y);
        cr.stroke().expect("Failed to stroke path");

        start_x = end_x;
        start_y = end_y;
    }

    let p_last = &stroke.points[stroke.points.len() - 1];
    cr.set_line_width(1.0 + (p_last.pressure * 3.0));
    cr.set_source_rgba(1.0, 1.0, 1.0, p_last.pressure.clamp(0.1, 1.0));
    cr.move_to(start_x, start_y);
    cr.line_to(p_last.x, p_last.y);
    cr.stroke().expect("Failed to stroke path");
}

fn build_ui(app: &Application) {
    let provider = CssProvider::new();
    provider.load_from_data("window { background-color: transparent; }");
    
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let state = Rc::new(RefCell::new(AppState {
        strokes: Vec::new(),
        current_stroke: None,
        is_erasing: false,
    }));

    let drawing_area = DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);
    drawing_area.set_cursor_from_name(Some("none"));

    let state_draw = state.clone();
    drawing_area.set_draw_func(move |_area, cr, _width, _height| {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(gtk::cairo::Operator::Clear);
        cr.paint().expect("Failed to clear background");
        cr.set_operator(gtk::cairo::Operator::Over);

        let state = state_draw.borrow();
        
        for stroke in &state.strokes {
            render_stroke(cr, stroke);
        }

        if let Some(current) = &state.current_stroke {
            render_stroke(cr, current);
        }
    });

    let stylus = GestureStylus::new();
    
    // IMPORTANT: Listen to all buttons, not just the primary pen tip
    stylus.set_button(0);
    
    let state_down = state.clone();
    let da_down = drawing_area.clone();
    stylus.connect_down(move |gesture, x, y| {
        let button = gesture.current_button();
        
        // Button 1 is the main tip. Anything else (or a dedicated eraser tool) gets mapped to erasing.
        let is_eraser = button != 1 || gesture.device_tool().map_or(false, |t| t.tool_type() == gtk::gdk::DeviceToolType::Eraser);
        
        let mut state = state_down.borrow_mut();
        
        if is_eraser {
            state.is_erasing = true;
            if state.erase_at(x, y) {
                da_down.queue_draw();
            }
        } else {
            let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
            state.is_erasing = false;
            state.current_stroke = Some(Stroke { points: vec![Point { x, y, pressure }] });
        }
    });

    let state_motion = state.clone();
    let da_motion = drawing_area.clone();
    stylus.connect_motion(move |gesture, x, y| {
        let mut state = state_motion.borrow_mut();
        
        if state.is_erasing {
            if state.erase_at(x, y) {
                da_motion.queue_draw(); 
            }
        } else if let Some(stroke) = &mut state.current_stroke {
            let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
            stroke.points.push(Point { x, y, pressure });
            da_motion.queue_draw(); 
        }
    });

    let state_up = state.clone();
    let da_up = drawing_area.clone();
    stylus.connect_up(move |_gesture, _x, _y| {
        let mut state = state_up.borrow_mut();
        state.is_erasing = false;
        
        if let Some(stroke) = state.current_stroke.take() {
            state.strokes.push(stroke);
            da_up.queue_draw();
        }
    });

    drawing_area.add_controller(stylus);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Minimal Sketch")
        .child(&drawing_area)
        .build();

    window.set_cursor_from_name(Some("none"));

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    let key_controller = EventControllerKey::new();
    let window_clone = window.clone();
    let state_save = state.clone();
    
    key_controller.connect_key_pressed(move |_ctrl, key, _keycode, modifier_state| {
        if key == gdk::Key::Escape {
            window_clone.set_visible(false); 
            return glib::Propagation::Stop;
        }

        if (key == gdk::Key::q || key == gdk::Key::Q) && modifier_state.contains(gdk::ModifierType::CONTROL_MASK) {
            window_clone.close();
            return glib::Propagation::Stop;
        }
        
        if (key == gdk::Key::s || key == gdk::Key::S) && modifier_state.contains(gdk::ModifierType::CONTROL_MASK) {
            let width = window_clone.width() as f64;
            let height = window_clone.height() as f64;
            
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let filename = format!("sketchlayer_{}.svg", timestamp);
            
            match gtk::cairo::SvgSurface::new(width, height, Some(&filename)) {
                Ok(surface) => {
                    let cr = gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");
                    let state = state_save.borrow();
                    
                    for stroke in &state.strokes {
                        render_stroke(&cr, stroke);
                    }
                    
                    if let Some(current) = &state.current_stroke {
                        render_stroke(&cr, current);
                    }
                    
                    surface.finish();
                    println!("✅ Sketch saved to {}", filename);
                }
                Err(e) => eprintln!("❌ Failed to save SVG: {:?}", e),
            }
            
            return glib::Propagation::Stop;
        }
        
        glib::Propagation::Proceed
    });
    
    window.add_controller(key_controller);
    window.present();
}
