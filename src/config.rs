use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{error, warn};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Config {
    pub save_dir: Option<String>,
    pub load_file: Option<String>,
    pub base_pen_width: f64,
    pub pen_pressure_mult: f64,
    pub base_eraser_width: f64,
    pub eraser_pressure_mult: f64,
    pub frame_width: f64,
    pub frame_height: f64,
    pub frame_offset_x: f64,
    pub frame_offset_y: f64,
    pub show_frames: bool,
    pub show_vanishing_points: bool,
    pub vp1: [f64; 2],
    pub vp2: [f64; 2],
    pub vp3: [f64; 2],
    pub transparent_background: bool,
    pub background_color: [f64; 4],
    pub target_monitor: Option<String>,
    pub max_undo_steps: usize,
}

impl Default for Config {
    fn default() -> Self {
        let default_save_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(".local/share"))
            .join("sketchlayer");

        Self {
            save_dir: Some(default_save_path.to_string_lossy().into_owned()),
            load_file: None,
            base_pen_width: 1.0,
            pen_pressure_mult: 3.0,
            base_eraser_width: 5.0,
            eraser_pressure_mult: 15.0,
            frame_width: 50.0,
            frame_height: 50.0,
            frame_offset_x: 0.0,
            frame_offset_y: 0.0,
            show_frames: false,
            show_vanishing_points: false,
            vp1: [0.0, 0.0],       // Top left of main frame
            vp2: [1920.0, 0.0],      // Top right of main frame
            vp3: [960.0, 1640.0],    // Below center of main frame
            transparent_background: true,
            background_color: [1.0, 1.0, 1.0, 1.0],
            target_monitor: None,
            max_undo_steps: 50,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("sketchlayer");

        let config_file = config_dir.join("config.toml");

        if config_file.exists() {
            if let Ok(contents) = fs::read_to_string(&config_file) {
                match toml::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => warn!(
                        "Failed to parse config.toml ({:?}). Falling back to defaults.",
                        e
                    ),
                }
            } else {
                warn!("Failed to read config.toml. Falling back to defaults.");
            }
        } else {
            let default_config = Self::default();
            default_config.save();
        }

        Self::default()
    }

    pub fn save(&self) {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("sketchlayer");

        let config_file = config_dir.join("config.toml");

        if let Ok(toml_string) = toml::to_string(self) {
            let _ = fs::create_dir_all(&config_dir);
            if let Err(e) = fs::write(&config_file, toml_string) {
                error!("Failed to save config.toml: {:?}", e);
            }
        }
    }

    pub fn get_resolved_save_dir(&self) -> PathBuf {
        let fallback = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(".local/share"))
            .join("sketchlayer");

        let dir_str = self.save_dir.as_deref().unwrap_or("");

        if dir_str.is_empty() {
            return fallback;
        }

        if dir_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&dir_str[2..]);
            }
        }

        PathBuf::from(dir_str)
    }

    pub fn get_resolved_load_file(&self) -> Option<PathBuf> {
        let file_str = self.load_file.as_deref()?;

        if file_str.is_empty() {
            return None;
        }

        if file_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return Some(home.join(&file_str[2..]));
            }
        }

        Some(PathBuf::from(file_str))
    }
}
