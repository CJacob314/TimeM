use anyhow::Error;
use std::path::PathBuf;
use std::result;
use timem::{exit_error, log, logger_init, Config};

type Result<T> = result::Result<T, Error>;

fn main() -> Result<()> {
    logger_init();
    log::info!("TimeM Service Started");
    let mut config = match Config::new(true) {
        Ok(config) => config,
        Err(err_str) => {
            exit_error!("Config error: {err_str}");
        }
    };
    loop {
        match config.update_if_changed() {
            Ok(_) => {}
            Err(err_str) => log::error!("Updating config: {err_str}"),
        }

        let mut snapshotted_paths = Vec::with_capacity(config.dirs_with_changes.len());
        for changed_path in config.dirs_with_changes.iter() {
            if handle_dir(&config, changed_path.clone())? {
                snapshotted_paths.push(changed_path.to_owned());
            }
        }

        snapshotted_paths.iter().for_each(|path| {
            config.dirs_with_changes.remove(path);
        });

        std::hint::spin_loop();
    }
}

fn handle_dir(config: &Config, path: PathBuf) -> Result<bool> {
    let watch_dir = match config.get_watched_dir(&path) {
        Some(wd) => wd,
        None => {
            panic!(
                "Directory marked as changed no longer in list of watched directories: {:?}",
                &path
            );
        }
    };

    watch_dir.snapshot(false)
}
