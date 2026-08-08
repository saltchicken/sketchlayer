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
        // Clear background to black
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.paint().expect("Failed to clear background");

        // Set ink properties to white
        cr.set_source_rgb(1.0, 1.0, 1.0); 
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        cr.set_line_join(gtk::cairo::LineJoin::Round);

        let state = state_draw.borrow();
        
        let draw_stroke = |stroke: &Stroke| {
            if stroke.points.len() < 2 { return; }

            // Fallback for very short strokes (just 2 points)
            if stroke.points.len() == 2 {
                let p1 = &stroke.points[0];
                let p2 = &stroke.points[1];
                cr.set_line_width(2.0 + (p1.pressure * 6.0));
                cr.move_to(p1.x, p1.y);
                cr.line_to(p2.x, p2.y);
                cr.stroke().expect("Failed to stroke path");
                return;
            }

            // Start the curve at the very first point
            let mut start_x = stroke.points[0].x;
            let mut start_y = stroke.points[0].y;

            // Loop through all middle points
            for i in 1..(stroke.points.len() - 1) {
                let p_ctrl = &stroke.points[i];     // The raw point pulls the curve (Control Point)
                let p_next = &stroke.points[i + 1]; 

                // Calculate the midpoint between current and next raw point
                let end_x = (p_ctrl.x + p_next.x) / 2.0;
                let end_y = (p_ctrl.y + p_next.y) / 2.0;

                // Cairo only has Cubic Beziers (2 control points). 
                // We convert our Quadratic Bezier (1 control point) into a Cubic using this math formula:
                let cp1_x = start_x + (2.0 / 3.0) * (p_ctrl.x - start_x);
                let cp1_y = start_y + (2.0 / 3.0) * (p_ctrl.y - start_y);
                
                let cp2_x = end_x + (2.0 / 3.0) * (p_ctrl.x - end_x);
                let cp2_y = end_y + (2.0 / 3.0) * (p_ctrl.y - end_y);

                // Set dynamic thickness for this segment
                let thickness = 2.0 + (p_ctrl.pressure * 6.0);
                cr.set_line_width(thickness);

                // Draw the smooth curve
                cr.move_to(start_x, start_y);
                cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, end_x, end_y);
                cr.stroke().expect("Failed to stroke path");

                // The end of this curve becomes the start of the next one
                start_x = end_x;
                start_y = end_y;
            }

            // Draw a final straight line to the very last point
            let p_last = &stroke.points[stroke.points.len() - 1];
            let thickness = 2.0 + (p_last.pressure * 6.0);
            cr.set_line_width(thickness);
            cr.move_to(start_x, start_y);
            cr.line_to(p_last.x, p_last.y);
            cr.stroke().expect("Failed to stroke path");
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
