use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Button, CssProvider, DrawingArea, EventControllerKey,
    GestureStylus, Orientation, Popover, gdk, glib,
};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

use crate::render::{render_stroke, render_active_stroke, save_sketch};
use crate::state::{AppState, Point, Stroke};

pub fn build_ui(app: &Application) {
    setup_css();

    let state = AppState::new();

    let drawing_area = DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);
    drawing_area.set_cursor_from_name(Some("none"));

    let popover = build_context_menu(&drawing_area, state.clone());

    setup_drawing_area(&drawing_area, state.clone());
    setup_stylus_events(&drawing_area, state.clone(), popover);

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

    setup_keyboard_events(&window, &drawing_area, state.clone());
    window.present();
}

fn setup_css() {
    let provider = CssProvider::new();
    provider.load_from_data("window { background-color: transparent; }");

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_context_menu(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) -> Popover {
    let popover = Popover::new();
    popover.set_parent(drawing_area);
    popover.set_has_arrow(true);

    let menu_box = gtk::Box::new(Orientation::Vertical, 4);
    menu_box.set_margin_top(4);
    menu_box.set_margin_bottom(4);
    menu_box.set_margin_start(4);
    menu_box.set_margin_end(4);
    popover.set_child(Some(&menu_box));

    // Color Palette
    let color_box = gtk::Box::new(Orientation::Horizontal, 4);
    color_box.set_halign(gtk::Align::Center);
    color_box.set_margin_bottom(8);

    let colors = [
        ("White", (1.0, 1.0, 1.0)),
        ("Red", (1.0, 0.2, 0.2)),
        ("Green", (0.2, 1.0, 0.2)),
        ("Blue", (0.2, 0.5, 1.0)),
        ("Yellow", (1.0, 1.0, 0.2)),
        ("Black", (0.0, 0.0, 0.0)),
    ];

    for (name, color_val) in colors {
        let btn = Button::builder().tooltip_text(name).build();

        let (r, g, b) = color_val;
        let rgba_string = format!(
            "rgba({}, {}, {}, 1.0)",
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32
        );
        let css = format!(
            "button {{ background: {}; min-width: 24px; min-height: 24px; border-radius: 12px; }}",
            rgba_string
        );

        let provider = CssProvider::new();
        provider.load_from_data(&css);
        btn.style_context()
            .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let state_color = state.clone();
        btn.connect_clicked(move |_| {
            state_color.borrow_mut().current_color = color_val;
        });
        color_box.append(&btn);
    }
    menu_box.append(&color_box);

    let btn_undo = Button::with_label("Undo");
    let btn_redo = Button::with_label("Redo");
    let btn_bg = Button::with_label("Toggle Background");

    let opacity_box = gtk::Box::new(Orientation::Horizontal, 8);
    opacity_box.set_margin_start(4);
    opacity_box.set_margin_end(4);
    let opacity_label = gtk::Label::new(Some("Opacity:"));
    let opacity_scale = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
    opacity_scale.set_value(100.0);
    opacity_scale.set_draw_value(true);
    opacity_scale.set_hexpand(true);
    opacity_box.append(&opacity_label);
    opacity_box.append(&opacity_scale);

    let btn_opt = Button::with_label("Toggle Optimization");
    let btn_save = Button::with_label("Save Sketch");
    let btn_clear = Button::with_label("Clear Canvas");
    let btn_hide = Button::with_label("Hide Overlay");
    let btn_quit = Button::with_label("Quit");

    menu_box.append(&btn_undo);
    menu_box.append(&btn_redo);
    menu_box.append(&btn_bg);
    menu_box.append(&opacity_box);
    menu_box.append(&btn_opt);
    menu_box.append(&btn_save);
    menu_box.append(&btn_clear);
    menu_box.append(&btn_hide);
    menu_box.append(&btn_quit);

    let da_undo = drawing_area.clone();
    let state_undo = state.clone();
    btn_undo.connect_clicked(move |_| {
        if state_undo.borrow_mut().undo() {
            da_undo.queue_draw();
        }
    });

    let da_redo = drawing_area.clone();
    let state_redo = state.clone();
    btn_redo.connect_clicked(move |_| {
        if state_redo.borrow_mut().redo() {
            da_redo.queue_draw();
        }
    });

    let da_bg = drawing_area.clone();
    let state_bg = state.clone();
    let pop_bg = popover.clone();
    btn_bg.connect_clicked(move |_| {
        {
            let mut s = state_bg.borrow_mut();
            s.white_background = !s.white_background;
            s.cache_valid = false; // Background changed, invalidate cache
        }
        da_bg.queue_draw();
        pop_bg.popdown();
    });

    let da_opacity = drawing_area.clone();
    opacity_scale.connect_value_changed(move |scale| {
        if let Some(window) = da_opacity.root().and_downcast_ref::<ApplicationWindow>() {
            window.set_opacity(scale.value() / 100.0);
        }
    });

    let da_opt = drawing_area.clone();
    let state_opt = state.clone();
    let pop_opt = popover.clone();
    btn_opt.connect_clicked(move |_| {
        {
            let mut s = state_opt.borrow_mut();
            s.optimized_rendering = !s.optimized_rendering;
        }
        da_opt.queue_draw();
        pop_opt.popdown();
    });

    let da_save = drawing_area.clone();
    let state_save_menu = state.clone();
    let pop_save = popover.clone();
    btn_save.connect_clicked(move |_| {
        if let Some(window) = da_save.root().and_downcast_ref::<ApplicationWindow>() {
            save_sketch(&window, &state_save_menu.borrow());
        }
        pop_save.popdown();
    });

    let da_clear = drawing_area.clone();
    let state_clear = state.clone();
    let pop_clear = popover.clone();
    btn_clear.connect_clicked(move |_| {
        let mut s = state_clear.borrow_mut();
        if !s.strokes.is_empty() {
            let strokes = std::mem::take(&mut s.strokes);
            s.history.push(crate::state::Action::Clear(strokes));
            s.redo_history.clear();
            s.cache_valid = false; // Cleared, invalidate cache
            da_clear.queue_draw();
        }
        pop_clear.popdown();
    });

    let da_hide = drawing_area.clone();
    let pop_hide = popover.clone();
    btn_hide.connect_clicked(move |_| {
        if let Some(window) = da_hide.root().and_downcast_ref::<ApplicationWindow>() {
            window.set_visible(false);
        }
        pop_hide.popdown();
    });

    let da_quit = drawing_area.clone();
    btn_quit.connect_clicked(move |_| {
        if let Some(window) = da_quit.root().and_downcast_ref::<ApplicationWindow>() {
            window.close();
        }
    });

    popover
}

fn setup_drawing_area(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) {
    drawing_area.set_draw_func(move |area, cr, width, height| {
        let mut state = state.borrow_mut();

        // 1. Unoptimized Path (The old way)
        if !state.optimized_rendering {
            if state.white_background {
                cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cr.set_operator(gtk::cairo::Operator::Source);
            } else {
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                cr.set_operator(gtk::cairo::Operator::Clear);
            }
            
            cr.paint().expect("Failed to paint background");
            cr.set_operator(gtk::cairo::Operator::Over);

            for stroke in &state.strokes {
                render_stroke(cr, stroke);
            }

            if let Some(current) = &state.current_stroke {
                render_stroke(cr, current); // Keeps the heavy render here for comparison
            }
            return;
        }

        // 2. Optimized Path (Using Cairo Cache Surface)
        let scale = area.scale_factor();
        let pixel_w = width * scale;
        let pixel_h = height * scale;

        // Check if we need to rebuild the cache (resize or cache invalidated)
        let rebuild = match &state.cache_surface {
            None => true,
            Some(surf) => surf.width() != pixel_w || surf.height() != pixel_h || !state.cache_valid,
        };

        if rebuild && pixel_w > 0 && pixel_h > 0 {
            let cache = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, pixel_w, pixel_h)
                .expect("Failed to create cache surface");
            cache.set_device_scale(scale as f64, scale as f64);

            let cache_cr = gtk::cairo::Context::new(&cache).expect("Failed to create cache context");

            if state.white_background {
                cache_cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cache_cr.set_operator(gtk::cairo::Operator::Source);
            } else {
                cache_cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                cache_cr.set_operator(gtk::cairo::Operator::Clear);
            }
            cache_cr.paint().unwrap();
            cache_cr.set_operator(gtk::cairo::Operator::Over);

            // Bake all finished strokes into the cache
            for stroke in &state.strokes {
                render_stroke(&cache_cr, stroke);
            }
            
            state.cache_surface = Some(cache);
            state.cache_valid = true;
        }

        // Blit the flattened cache directly to the screen (Extremely fast)
        if let Some(cache) = &state.cache_surface {
            cr.set_source_surface(cache, 0.0, 0.0).unwrap();
            cr.paint().unwrap();
        }

        // Draw ONLY the current active stroke on top using the FAST method
        if let Some(current) = &state.current_stroke {
            render_active_stroke(cr, current);
        }
    });
}

fn setup_stylus_events(drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>, popover: Popover) {
    let stylus = GestureStylus::new();
    stylus.set_button(0);

    let state_down = state.clone();
    let da_down = drawing_area.clone();
    let popover_down = popover.clone();
    stylus.connect_down(move |gesture, x, y| {
        let button = gesture.current_button();

        if button == 3 {
            popover_down.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover_down.popup();

            let mut state = state_down.borrow_mut();
            state.is_erasing = false;
            state.current_stroke = None;
            return;
        }

        let is_eraser = button != 1
            || gesture
                .device_tool()
                .map_or(false, |t| t.tool_type() == gtk::gdk::DeviceToolType::Eraser);

        let mut state = state_down.borrow_mut();

        if is_eraser {
            state.is_erasing = true;
            state.current_erased.clear();
            if state.erase_at(x, y) {
                da_down.queue_draw();
            }
        } else {
            let pressure = gesture.axis(gtk::gdk::AxisUse::Pressure).unwrap_or(1.0);
            let color = state.current_color;
            let id = state.next_stroke_id;
            state.next_stroke_id += 1;

            state.is_erasing = false;
            state.current_stroke = Some(Stroke {
                id,
                points: vec![Point { x, y, pressure }],
                color,
            });
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
        
        if state.is_erasing {
            state.is_erasing = false;
            if !state.current_erased.is_empty() {
                let erased = std::mem::take(&mut state.current_erased);
                state.history.push(crate::state::Action::Erase(erased));
                state.redo_history.clear();
                state.cache_valid = false;
            }
        } else if let Some(stroke) = state.current_stroke.take() {
            state.history.push(crate::state::Action::Draw(stroke.clone()));
            state.strokes.push(stroke);
            state.redo_history.clear();
            
            // The stroke is finished, so we invalidate the cache to bake it in
            state.cache_valid = false; 
            da_up.queue_draw();
        }
    });

    drawing_area.add_controller(stylus);
}

fn setup_keyboard_events(window: &ApplicationWindow, drawing_area: &DrawingArea, state: Rc<RefCell<AppState>>) {
    let key_controller = EventControllerKey::new();
    let window_clone = window.clone();
    let da_clone = drawing_area.clone();

    key_controller.connect_key_pressed(move |_ctrl, key, _keycode, modifier_state| {
        if key == gdk::Key::Escape {
            window_clone.set_visible(false);
            return glib::Propagation::Stop;
        }

        if (key == gdk::Key::q || key == gdk::Key::Q)
            && modifier_state.contains(gdk::ModifierType::CONTROL_MASK)
        {
            window_clone.close();
            return glib::Propagation::Stop;
        }

        if (key == gdk::Key::s || key == gdk::Key::S)
            && modifier_state.contains(gdk::ModifierType::CONTROL_MASK)
        {
            save_sketch(&window_clone, &state.borrow());
            return glib::Propagation::Stop;
        }

        if key == gdk::Key::z || key == gdk::Key::Z {
            if modifier_state.contains(gdk::ModifierType::CONTROL_MASK) {
                let mut state_mut = state.borrow_mut();
                
                if modifier_state.contains(gdk::ModifierType::SHIFT_MASK) {
                    if state_mut.redo() {
                        da_clone.queue_draw();
                    }
                } else {
                    if state_mut.undo() {
                        da_clone.queue_draw();
                    }
                }
                return glib::Propagation::Stop;
            }
        }

        glib::Propagation::Proceed
    });

    window.add_controller(key_controller);
}
