use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

// --- NEW: Spatial Index implementation ---
#[derive(Clone, Default)]
pub struct SpatialIndex {
    pub cell_size: f64,
    pub cells: HashMap<(i32, i32), HashSet<u64>>,
}

impl SpatialIndex {
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    fn get_cells(&self, bbox: &BoundingBox) -> Vec<(i32, i32)> {
        let min_x = (bbox.min_x / self.cell_size).floor() as i32;
        let max_x = (bbox.max_x / self.cell_size).floor() as i32;
        let min_y = (bbox.min_y / self.cell_size).floor() as i32;
        let max_y = (bbox.max_y / self.cell_size).floor() as i32;

        let mut cells = Vec::new();
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                cells.push((x, y));
            }
        }
        cells
    }

    pub fn insert(&mut self, id: u64, bbox: &BoundingBox) {
        for cell in self.get_cells(bbox) {
            self.cells.entry(cell).or_default().insert(id);
        }
    }

    pub fn remove(&mut self, id: u64, bbox: &BoundingBox) {
        for cell in self.get_cells(bbox) {
            if let Some(set) = self.cells.get_mut(&cell) {
                set.remove(&id);
                if set.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }

    pub fn query(&self, bbox: &BoundingBox) -> HashSet<u64> {
        let mut result = HashSet::new();
        for cell in self.get_cells(bbox) {
            if let Some(set) = self.cells.get(&cell) {
                result.extend(set);
            }
        }
        result
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }
}
