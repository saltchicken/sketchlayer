use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, DrawingArea, GestureStylus};
use std::cell::RefCell;
use std::rc::Rc;

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

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // 1. Initialize our shared application state
    let state = Rc::new(RefCell::new(AppState {
        strokes: Vec::new(),
        current_stroke: None,
    }));

    let drawing_area = DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);

    // 2. Set up the Cairo Rendering Loop
    let state_draw = state.clone();
    drawing_area.set_draw_func(move |_area, cr, _width, _height| {
        // Clear background to white
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.paint().expect("Failed to clear background");

        // Set ink properties
        cr.set_source_rgb(1.0, 1.0, 1.0); 
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        cr.set_line_join(gtk::cairo::LineJoin::Round);

        let state = state_draw.borrow();
        
        let draw_stroke = |stroke: &Stroke| {
            if stroke.points.len() < 2 { return; }
            
            // Draw lines between each recorded point
            for i in 0..(stroke.points.len() - 1) {
                let p1 = &stroke.points[i];
                let p2 = &stroke.points[i + 1];
                
                // Dynamically adjust stroke width based on tablet pressure
                let thickness = 2.0 + (p1.pressure * 6.0);
                cr.set_line_width(thickness);
                
                cr.move_to(p1.x, p1.y);
                cr.line_to(p2.x, p2.y);
                cr.stroke().expect("Failed to stroke path");
            }
        };

        // Render all finalized strokes
        for stroke in &state.strokes {
            draw_stroke(stroke);
        }

        // Render the stroke currently being drawn
        if let Some(current) = &state.current_stroke {
            draw_stroke(current);
        }
    });

    // 3. Set up Wacom Tablet / Stylus Event Handlers
    let stylus = GestureStylus::new();
    
    // Handle Pen Down
    let state_down = state.clone();
    stylus.connect_down(move |gesture, x, y| {
        let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
        let mut state = state_down.borrow_mut();
        
        state.current_stroke = Some(Stroke {
            points: vec![Point { x, y, pressure }],
        });
    });

    // Handle Pen Move
    let state_motion = state.clone();
    let da_motion = drawing_area.clone();
    stylus.connect_motion(move |gesture, x, y| {
        let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
        let mut state = state_motion.borrow_mut();
        
        if let Some(stroke) = &mut state.current_stroke {
            stroke.points.push(Point { x, y, pressure });
            // Request a redraw to update the screen
            da_motion.queue_draw(); 
        }
    });

    // Handle Pen Up
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

    // 4. Build the borderless overlay window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Minimal Sketch")
        .child(&drawing_area)
        .decorated(false) // Removes title bar and buttons for an overlay feel
        .default_width(1280)
        .default_height(720)
        .build();

    window.present();
}
