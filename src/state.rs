use std::rc::Rc;
use std::cell::RefCell;
use serde::{Deserialize, Serialize};
use cairo::ImageSurface;
use crate::config::Config;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub pressure: f64,
}

impl Point {
    pub fn distance_to_segment(&self, v: &Point, w: &Point) -> f64 {
        let l2 = (v.x - w.x).powi(2) + (v.y - w.y).powi(2);
        if l2 == 0.0 {
            return ((self.x - v.x).powi(2) + (self.y - v.y).powi(2)).sqrt();
        }
        let t = ((self.x - v.x) * (w.x - v.x) + (self.y - v.y) * (w.y - v.y)) / l2;
        let t = t.clamp(0.0, 1.0);
        let proj_x = v.x + t * (w.x - v.x);
        let proj_y = v.y + t * (w.y - v.y);
        ((self.x - proj_x).powi(2) + (self.y - proj_y).powi(2)).sqrt()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64) -> Self {
        Self { min_x: x, min_y: y, max_x: x, max_y: y }
    }
    
    pub fn expand(&mut self, x: f64, y: f64, padding: f64) {
        self.min_x = self.min_x.min(x - padding);
        self.min_y = self.min_y.min(y - padding);
        self.max_x = self.max_x.max(x + padding);
        self.max_y = self.max_y.max(y + padding);
    }
    
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min_x <= other.max_x && self.max_x >= other.min_x &&
        self.min_y <= other.max_y && self.max_y >= other.min_y
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stroke {
    pub id: u64,
    pub points: Vec<Point>,
    pub color: (f64, f64, f64),
    pub is_eraser: bool,
    pub bbox: Option<BoundingBox>,
}

#[derive(Clone)]
pub enum Action {
    Draw(Rc<Stroke>),
    Erase(Vec<Rc<Stroke>>),
    Clear(Vec<Rc<Stroke>>),
}

#[derive(Clone, Copy, PartialEq)]
pub enum EraseMode {
    Vector,
    Pixel,
}

pub struct AppState {
    pub config: Config,
    pub strokes: Vec<Rc<Stroke>>,
    pub active_stroke: Option<Stroke>,
    pub undo_stack: Vec<Action>,
    pub redo_stack: Vec<Action>,
    
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    
    pub current_color: (f64, f64, f64),
    pub is_erasing: bool,
    pub erase_mode: EraseMode,
    
    pub cached_surface: Option<ImageSurface>,
    pub rendered_strokes_count: usize,
    pub needs_full_redraw: bool,
    pub next_stroke_id: u64,
}

impl AppState {
    pub fn new(config: Config) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            config,
            strokes: Vec::new(),
            active_stroke: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            current_color: (1.0, 0.2, 0.2), 
            is_erasing: false,
            erase_mode: EraseMode::Vector,
            cached_surface: None,
            rendered_strokes_count: 0,
            needs_full_redraw: true,
            next_stroke_id: 1,
        }))
    }

    pub fn screen_to_canvas(&self, x: f64, y: f64) -> (f64, f64) {
        ((x - self.offset_x) / self.zoom, (y - self.offset_y) / self.zoom)
    }
}
