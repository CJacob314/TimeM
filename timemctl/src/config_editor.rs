use crate::WatchDir;
use std::fs;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

pub struct ConfigEditor {
    config_path: PathBuf,
    watched_dirs: Vec<WatchDir>,
}

impl ConfigEditor {
    pub fn new() -> Result<Self, String> {
        let base_dirs =
            BaseDirs::new().ok_or("Could not locate OS config directory".to_string())?;
        let mut config_path = base_dirs.config_dir().to_owned();
        config_path.push("timem/config.json");

        let watched_dirs = if config_path.exists() {
            match Self::load_config(&config_path) {
                Ok(dirs) => dirs,
                Err(err) => {
                    eprintln!("Failed to load config: {}", err);
                    Vec::new()
                }
            }
        } else {
            fs::create_dir_all(
                config_path
                    .parent()
                    .ok_or("Could not get parent directory of config file".to_string())?,
            )
            .map_err(|err| err.to_string())?;
            Vec::new()
        };

        Ok(Self {
            config_path,
            watched_dirs,
        })
    }

    fn load_config<P: AsRef<Path>>(path: P) -> Result<Vec<WatchDir>, String> {
        let config_content = fs::read_to_string(path).map_err(|err| err.to_string())?;
        let dirs = serde_json::from_str(&config_content).map_err(|err| err.to_string())?;
        Ok(dirs)
    }

    pub fn flush_config(&self) -> Result<(), String> {
        let content = serde_json::to_string(&self.watched_dirs).map_err(|err| err.to_string())?;
        fs::write(&self.config_path, content).map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn add_watched_dir(&mut self, watch_dir_conf: WatchDir) {
        self.watched_dirs.push(watch_dir_conf);
    }
}
