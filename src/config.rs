use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub save_dir: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: Some("~/Pictures/Sketches".to_string()),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        // Resolve ~/.config/sketchlayer
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("sketchlayer");
            
        let config_file = config_dir.join("config.toml");

        if config_file.exists() {
            if let Ok(contents) = fs::read_to_string(&config_file) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                }
            }
        } else {
            // Create default config file if it doesn't exist
            let default_config = Self::default();
            if let Ok(toml_string) = toml::to_string(&default_config) {
                let _ = fs::create_dir_all(&config_dir);
                let _ = fs::write(&config_file, toml_string);
            }
        }
        
        Self::default()
    }

    /// Resolves the save directory, automatically expanding `~/` to the user's home directory.
    pub fn get_resolved_save_dir(&self) -> PathBuf {
        let dir_str = self.save_dir.as_deref().unwrap_or("~/Pictures/Sketches");
        
        if dir_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&dir_str[2..]);
            }
        }
        
        PathBuf::from(dir_str)
    }
}
