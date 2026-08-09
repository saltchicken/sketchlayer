use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use crate::config::Config;

#[derive(Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub pressure: f64,
}

#[derive(Clone)]
pub struct Stroke {
    pub id: u64, // Used to preserve z-order when undoing erasures
    pub points: Vec<Point>,
    pub color: (f64, f64, f64), // (R, G, B)
}

#[derive(Clone)]
pub enum Action {
    Draw(Stroke),
    Erase(Vec<Stroke>), // Can contain multiple strokes deleted in one swipe
    Clear(Vec<Stroke>), // Saves the entire canvas when cleared
}

pub struct AppState {
    pub next_stroke_id: u64,
    pub strokes: Vec<Stroke>,
    pub history: Vec<Action>,
    pub redo_history: Vec<Action>,
    
    pub current_stroke: Option<Stroke>,
    pub current_erased: Vec<Stroke>, // Accumulates erased strokes during a single swipe
    pub is_erasing: bool,
    
    pub current_color: (f64, f64, f64),
    pub white_background: bool,
    pub config: Config,
}

impl AppState {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            next_stroke_id: 0,
            strokes: Vec::new(),
            history: Vec::new(),
            redo_history: Vec::new(),
            
            current_stroke: None,
            current_erased: Vec::new(),
            is_erasing: false,
            
            current_color: (0.0, 0.0, 0.0),
            white_background: false,
            config: Config::load(),
        }))
    }

    pub fn undo(&mut self) -> bool {
        if let Some(action) = self.history.pop() {
            match action {
                Action::Draw(stroke) => {
                    self.strokes.pop(); // The drawn stroke is always at the end
                    self.redo_history.push(Action::Draw(stroke));
                }
                Action::Erase(erased_strokes) => {
                    // Put the erased strokes back and sort by ID to restore exact z-order
                    self.strokes.extend(erased_strokes.clone());
                    self.strokes.sort_by_key(|s| s.id);
                    self.redo_history.push(Action::Erase(erased_strokes));
                }
                Action::Clear(strokes) => {
                    self.strokes = strokes.clone();
                    self.redo_history.push(Action::Clear(strokes));
                }
            }
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(action) = self.redo_history.pop() {
            match action {
                Action::Draw(stroke) => {
                    self.strokes.push(stroke.clone());
                    self.history.push(Action::Draw(stroke));
                }
                Action::Erase(erased_strokes) => {
                    // Re-erase the strokes based on their unique IDs
                    let erased_ids: HashSet<u64> = erased_strokes.iter().map(|s| s.id).collect();
                    self.strokes.retain(|s| !erased_ids.contains(&s.id));
                    self.history.push(Action::Erase(erased_strokes));
                }
                Action::Clear(strokes) => {
                    self.strokes.clear();
                    self.history.push(Action::Clear(strokes));
                }
            }
            true
        } else {
            false
        }
    }

    pub fn erase_at(&mut self, x: f64, y: f64) -> bool {
        let erase_radius = 15.0;
        let mut erased_any = false;

        // Iterate backwards so removing items doesn't mess up our loop index
        for i in (0..self.strokes.len()).rev() {
            let stroke = &self.strokes[i];
            let mut hit = false;

            if stroke.points.is_empty() {
                continue;
            } else if stroke.points.len() == 1 {
                let p = &stroke.points[0];
                hit = ((p.x - x).powi(2) + (p.y - y).powi(2)).sqrt() <= erase_radius;
            } else {
                for j in 0..(stroke.points.len() - 1) {
                    let p1 = &stroke.points[j];
                    let p2 = &stroke.points[j + 1];

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
                        hit = true;
                        break;
                    }
                }
            }

            if hit {
                let removed_stroke = self.strokes.remove(i);
                self.current_erased.push(removed_stroke);
                erased_any = true;
            }
        }

        erased_any
    }
}
