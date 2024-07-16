mod config;
mod macros;
mod watchdir;
pub use crate::config::Config;
pub use crate::watchdir::WatchDir;

use std::fs;
use std::path::PathBuf;

use lazy_static::lazy_static;

use directories::BaseDirs;

use env_logger;

pub use log;

lazy_static! {
    pub static ref CONFIG_DIR: Option<PathBuf> = {
        let base_dirs = BaseDirs::new();
        if base_dirs.is_none() {
            return None;
        }
        let mut config_path = base_dirs.unwrap().config_dir().to_owned();
        config_path.push("timem/");

        if !config_path.exists() {
            match fs::create_dir_all(&config_path) {
                Ok(_) => {}
                Err(e) => {
                    exit_error!("Failed to create config directory: {e}");
                }
            }
        }

        let config_json_path = config_path.join("config.json");
        if !config_json_path.exists() {
            match fs::File::create(&config_json_path) {
                Ok(_) => {}
                Err(e) => {
                    exit_error!("Failed to create config file: {e}");
                }
            }
        }
        Some(config_path)
    };
    pub static ref DOTGIT_DIR_DIR: Option<PathBuf> = {
        CONFIG_DIR
            .clone()
            .and_then(|conf_dir| Some(conf_dir.join(".git_dirs")))
    };
    pub static ref ENV_LOGGER_INIT: () = env_logger::Builder::from_env("LOG_CONFIG").init();
}

pub fn logger_init() {
    let _ = std::hint::black_box(*ENV_LOGGER_INIT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
