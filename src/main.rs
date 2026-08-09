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
}

fn main() {
    let app = Application::builder()
        .application_id("com.github.minimal_sketch")
        .build();

    app.connect_activate(|app| {
        let windows = app.windows();
        
        if let Some(window) = windows.first() {
            // The app is already running, toggle visibility
            if window.is_visible() {
                window.set_visible(false);
            } else {
                window.set_visible(true);
                window.present();
            }
        } else {
            // First time launch, build the UI
            build_ui(app);
        }
    });

    app.run();
}

// 1. Extracted Drawing Logic
// We moved this out of the closure so both the screen and the SVG exporter can use it.
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
    }));

    let drawing_area = DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);

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
    
    let state_down = state.clone();
    stylus.connect_down(move |gesture, x, y| {
        let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
        let mut state = state_down.borrow_mut();
        state.current_stroke = Some(Stroke { points: vec![Point { x, y, pressure }] });
    });

    let state_motion = state.clone();
    let da_motion = drawing_area.clone();
    stylus.connect_motion(move |gesture, x, y| {
        let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
        let mut state = state_motion.borrow_mut();
        if let Some(stroke) = &mut state.current_stroke {
            stroke.points.push(Point { x, y, pressure });
            da_motion.queue_draw(); 
        }
    });

    let state_up = state.clone();
    let da_up = drawing_area.clone();
    stylus.connect_up(move |_gesture, _x, _y| {
        let mut state = state_up.borrow_mut();
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

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    // 2. Add Key Controller for SVG Saving, Hiding, and Quitting
    let key_controller = EventControllerKey::new();
    let window_clone = window.clone();
    let state_save = state.clone();
    
    key_controller.connect_key_pressed(move |_ctrl, key, _keycode, modifier_state| {
        // ESCAPE: Hide the window (keeps state in memory)
        if key == gdk::Key::Escape {
            window_clone.set_visible(false); 
            return glib::Propagation::Stop;
        }

        // CTRL + Q: Actually close the app completely and drop state
        if (key == gdk::Key::q || key == gdk::Key::Q) && modifier_state.contains(gdk::ModifierType::CONTROL_MASK) {
            window_clone.close();
            return glib::Propagation::Stop;
        }
        
        // Listen for Ctrl + S
        if (key == gdk::Key::s || key == gdk::Key::S) && modifier_state.contains(gdk::ModifierType::CONTROL_MASK) {
            let width = window_clone.width() as f64;
            let height = window_clone.height() as f64;
            
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let filename = format!("sketchlayer_{}.svg", timestamp);
            
            // Create a virtual Cairo surface that writes directly to an SVG file
            match gtk::cairo::SvgSurface::new(width, height, Some(&filename)) {
                Ok(surface) => {
                    let cr = gtk::cairo::Context::new(&surface).expect("Failed to create cairo context");
                    let state = state_save.borrow();
                    
                    // Run the exact same drawing logic on the SVG surface
                    for stroke in &state.strokes {
                        render_stroke(&cr, stroke);
                    }
                    
                    if let Some(current) = &state.current_stroke {
                        render_stroke(&cr, current);
                    }
                    
                    surface.finish(); // Ensure the file buffer flushes
                    println!("✅ Sketch saved to {}", filename); // Prints to terminal / Hyprland logs
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
