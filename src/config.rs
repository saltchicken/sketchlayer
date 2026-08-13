use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub save_dir: String,
    pub load_file: Option<String>,
    pub base_pen_width: f64,
    pub pen_pressure_mult: f64,
    pub base_eraser_width: f64,
    pub eraser_pressure_mult: f64,
    pub transparent_background: bool,
    pub background_color: (f64, f64, f64, f64),
    pub grid_cell_width: f64,
    pub grid_cell_height: f64,
    pub grid_offset_x: f64,
    pub grid_offset_y: f64,
    pub show_grid: bool,
    pub target_monitor: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: "~/.local/share/sketchlayer".to_string(),
            load_file: None,
            base_pen_width: 2.0,
            pen_pressure_mult: 4.0,
            base_eraser_width: 20.0,
            eraser_pressure_mult: 0.0,
            transparent_background: true,
            background_color: (0.0, 0.0, 0.0, 0.0),
            grid_cell_width: 100.0,
            grid_cell_height: 100.0,
            grid_offset_x: 0.0,
            grid_offset_y: 0.0,
            show_grid: false,
            target_monitor: None,
        }
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(user_dirs) = directories::UserDirs::new() {
            let mut p = user_dirs.home_dir().to_path_buf();
            p.push(&path[2..]);
            return p;
        }
    }
    PathBuf::from(path)
}

impl Config {
    pub fn load() -> Self {
        let config_path = expand_tilde("~/.config/sketchlayer/config.toml");
        if config_path.exists() {
            if let Ok(contents) = fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                }
            }
        } else {
            let default_cfg = Config::default();
            if let Some(parent) = config_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(toml_str) = toml::to_string_pretty(&default_cfg) {
                let _ = fs::write(&config_path, toml_str);
            }
        }
        Config::default()
    }
}
