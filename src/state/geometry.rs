use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
pub enum EraseMode {
    Vector,
    Pixel,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub pressure: f64,
}

impl Point {
    pub fn distance_to_segment(&self, p1: &Point, p2: &Point) -> f64 {
        let l2 = (p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2);
        if l2 == 0.0 {
            return ((self.x - p1.x).powi(2) + (self.y - p1.y).powi(2)).sqrt();
        }

        let t = (((self.x - p1.x) * (p2.x - p1.x) + (self.y - p1.y) * (p2.y - p1.y)) / l2)
            .clamp(0.0, 1.0);
        let proj_x = p1.x + t * (p2.x - p1.x);
        let proj_y = p1.y + t * (p2.y - p1.y);

        ((self.x - proj_x).powi(2) + (self.y - proj_y).powi(2)).sqrt()
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    pub fn expand(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Stroke {
    pub id: u64,
    pub points: Vec<Point>,
    pub color: (f64, f64, f64),
    pub is_eraser: bool,
    pub bbox: BoundingBox,
}

#[derive(Clone)]
pub enum Action {
    Draw(Rc<Stroke>),
    Erase(Vec<Rc<Stroke>>),
    Clear(Vec<Rc<Stroke>>),
}
