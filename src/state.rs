use std::cell::RefCell;
use std::rc::Rc;
use crate::config::Config;

#[derive(Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub pressure: f64,
}

pub struct Stroke {
    pub points: Vec<Point>,
    pub color: (f64, f64, f64), // (R, G, B)
}

pub struct AppState {
    pub strokes: Vec<Stroke>,
    pub undone_strokes: Vec<Stroke>,
    pub current_stroke: Option<Stroke>,
    pub is_erasing: bool,
    pub current_color: (f64, f64, f64),
    pub white_background: bool,
    pub config: Config,
}

impl AppState {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            strokes: Vec::new(),
            undone_strokes: Vec::new(),
            current_stroke: None,
            is_erasing: false,
            current_color: (0.0, 0.0, 0.0),
            white_background: false,
            config: Config::load(),
        }))
    }
    
    pub fn undo(&mut self) -> bool {
        if let Some(stroke) = self.strokes.pop() {
            self.undone_strokes.push(stroke);
            true // Indicate state changed
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(stroke) = self.undone_strokes.pop() {
            self.strokes.push(stroke);
            true // Indicate state changed
        } else {
            false
        }
    }

    pub fn erase_at(&mut self, x: f64, y: f64) -> bool {
        let erase_radius = 15.0;
        let initial_len = self.strokes.len();

        self.strokes.retain(|stroke| {
            if stroke.points.is_empty() {
                return false;
            }
            if stroke.points.len() == 1 {
                let p = &stroke.points[0];
                return ((p.x - x).powi(2) + (p.y - y).powi(2)).sqrt() > erase_radius;
            }

            for i in 0..(stroke.points.len() - 1) {
                let p1 = &stroke.points[i];
                let p2 = &stroke.points[i + 1];

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
                    return false;
                }
            }
            true
        });

        initial_len != self.strokes.len()
    }
}
