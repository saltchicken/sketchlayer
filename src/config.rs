use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Config {
    pub save_dir: Option<String>,
    pub base_pen_width: f64,
    pub pen_pressure_mult: f64,
    pub base_eraser_width: f64,
    pub eraser_pressure_mult: f64,
    pub grid_cell_width: f64,
    pub grid_cell_height: f64,
    pub grid_offset_x: f64,
    pub grid_offset_y: f64,
}

impl Default for Config {
    fn default() -> Self {
        let default_save_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(".local/share"))
            .join("sketchlayer");

        Self {
            save_dir: Some(default_save_path.to_string_lossy().into_owned()),
            base_pen_width: 1.0,
            pen_pressure_mult: 3.0,
            base_eraser_width: 5.0,
            eraser_pressure_mult: 15.0,
            grid_cell_width: 50.0,
            grid_cell_height: 50.0,
            grid_offset_x: 0.0,
            grid_offset_y: 0.0,
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
                    Err(e) => eprintln!("Warning: Failed to parse config.toml ({:?}). Falling back to defaults.", e),
                }
            } else {
                eprintln!("Warning: Failed to read config.toml. Falling back to defaults.");
            }
        } else {
            let default_config = Self::default();
            if let Ok(toml_string) = toml::to_string(&default_config) {
                let _ = fs::create_dir_all(&config_dir);
                let _ = fs::write(&config_file, toml_string);
            }
        }
        
        Self::default()
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
}
