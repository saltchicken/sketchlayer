use crate::config::Config;
use gtk4 as gtk;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::fs::File;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use anyhow::Result;

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

pub struct AppState {
    pub next_stroke_id: u64,
    pub strokes: Vec<Rc<Stroke>>,
    pub history: Vec<Action>,
    pub redo_history: Vec<Action>,

    pub current_stroke: Option<Stroke>,
    pub current_erased: Vec<Rc<Stroke>>,
    pub is_erasing: bool,
    pub erase_mode: EraseMode,

    pub current_color: (f64, f64, f64),
    pub config: Config,

    pub current_file: Option<PathBuf>,

    // View Transformation
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,

    // Caching for Performance
    pub cached_surface: Option<gtk::cairo::ImageSurface>,
    pub rendered_strokes_count: usize,
    pub needs_full_redraw: bool,
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
            erase_mode: EraseMode::Vector,

            current_color: (0.0, 0.0, 0.0),
            config: Config::load(),

            current_file: None,

            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,

            cached_surface: None,
            rendered_strokes_count: 0,
            needs_full_redraw: true,
        }))
    }
    
    pub fn save_state(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, &self.strokes)?;
        Ok(())
    }

    pub fn load_state(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path)?;
        let strokes: Vec<Rc<Stroke>> = serde_json::from_reader(file)?;
        self.strokes = strokes;
        self.next_stroke_id = self.strokes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        self.history.clear();
        self.redo_history.clear();
        self.needs_full_redraw = true;
        
        self.current_file = Some(path.to_path_buf());
        
        Ok(())
    }

    pub fn screen_to_canvas(&self, x: f64, y: f64) -> (f64, f64) {
        ((x - self.offset_x) / self.zoom, (y - self.offset_y) / self.zoom)
    }

    pub fn set_zoom(&mut self, new_zoom: f64, focal_x: f64, focal_y: f64) {
        let (canvas_x, canvas_y) = self.screen_to_canvas(focal_x, focal_y);
        
        self.zoom = new_zoom.clamp(0.1, 10.0);
        
        self.offset_x = focal_x - canvas_x * self.zoom;
        self.offset_y = focal_y - canvas_y * self.zoom;
        
        self.needs_full_redraw = true;
    }

    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.offset_x = 0.0;
        self.offset_y = 0.0;
        self.needs_full_redraw = true;
    }

    pub fn start_stroke(&mut self, x: f64, y: f64, pressure: f64, is_eraser: bool) {
        self.is_erasing = false;
        let id = self.next_stroke_id;
        self.next_stroke_id += 1;

        self.current_stroke = Some(Stroke {
            id,
            points: vec![Point { x, y, pressure }],
            color: if is_eraser {
                (0.0, 0.0, 0.0)
            } else {
                self.current_color
            },
            is_eraser,
            bbox: BoundingBox::new(x, y),
        });
    }

    pub fn continue_stroke(&mut self, x: f64, y: f64, pressure: f64) {
        if let Some(stroke) = &mut self.current_stroke {
            stroke.points.push(Point { x, y, pressure });
            stroke.bbox.expand(x, y);
        }
    }

    pub fn end_stroke(&mut self) {
        if let Some(stroke) = self.current_stroke.take() {
            let rc_stroke = Rc::new(stroke);
            self.history.push(Action::Draw(Rc::clone(&rc_stroke)));
            self.strokes.push(rc_stroke);
            self.redo_history.clear();
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(action) = self.history.pop() {
            match action {
                Action::Draw(stroke) => {
                    self.strokes.pop();
                    self.redo_history.push(Action::Draw(stroke));
                }
                Action::Erase(erased_strokes) => {
                    self.strokes.extend(erased_strokes.clone());
                    self.strokes.sort_by_key(|s| s.id);
                    self.redo_history.push(Action::Erase(erased_strokes));
                }
                Action::Clear(strokes) => {
                    self.strokes = strokes.clone();
                    self.redo_history.push(Action::Clear(strokes));
                }
            }
            self.needs_full_redraw = true;
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
                    let erased_ids: HashSet<u64> = erased_strokes.iter().map(|s| s.id).collect();
                    self.strokes.retain(|s| !erased_ids.contains(&s.id));
                    self.history.push(Action::Erase(erased_strokes));
                }
                Action::Clear(strokes) => {
                    self.strokes.clear();
                    self.history.push(Action::Clear(strokes));
                }
            }
            self.needs_full_redraw = true;
            true
        } else {
            false
        }
    }

    pub fn erase_at(&mut self, x: f64, y: f64) -> bool {
        let erase_radius = self.config.base_eraser_width;
        let mut erased_any = false;
        let test_point = Point {
            x,
            y,
            pressure: 1.0,
        };

        for i in (0..self.strokes.len()).rev() {
            let stroke = &self.strokes[i];

            if x < stroke.bbox.min_x - erase_radius
                || x > stroke.bbox.max_x + erase_radius
                || y < stroke.bbox.min_y - erase_radius
                || y > stroke.bbox.max_y + erase_radius
            {
                continue;
            }

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

                    if test_point.distance_to_segment(p1, p2) <= erase_radius {
                        hit = true;
                        break;
                    }
                }
            }

            if hit {
                let removed_stroke = self.strokes.remove(i);
                self.current_erased.push(removed_stroke);
                erased_any = true;
                self.needs_full_redraw = true;
            }
        }

        erased_any
    }
}
