use crate::exit_error;
use crate::WatchDir;
use notify::{
    event::{Event, EventKind, ModifyKind},
    Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher,
};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use serde_json::Deserializer;

use hashbrown::{
    hash_map::{HashMap, IterMut as HashBrownIterMut},
    hash_set::HashSet,
};

use directories::BaseDirs;

use crate::CONFIG_DIR;

pub struct Config {
    config_path: PathBuf,
    watched_dirs: HashMap<PathBuf, WatchDir>,
    dirs_with_changes: HashSet<PathBuf>,
    dir_watcher: RecommendedWatcher,
    config_change_listener: Receiver<NotifyResult<Event>>,
    is_watching_changes: bool,
}

impl Config {
    pub fn new(should_watch_changes: bool) -> Result<Self, String> {
        let mut config_path = match *CONFIG_DIR {
            Some(ref dir) => dir.clone(),
            None => {
                return Err("Could not locate OS config directory".into());
            }
        };
        config_path.push("config.json");

        let watched_dirs = if config_path.exists() {
            match Self::load_config(&config_path) {
                Ok(dirs) => dirs,
                Err(err) => {
                    log::warn!("Failed to load config: {}", err);
                    HashMap::new()
                }
            }
        } else {
            fs::create_dir_all(
                config_path
                    .parent()
                    .ok_or("Could not get parent directory of config file".to_string())?,
            )
            .map_err(|err| err.to_string())?;
            HashMap::new()
        };

        let (tx, rx) = mpsc::channel();
        let mut dir_watcher: RecommendedWatcher =
            Watcher::new(tx, NotifyConfig::default()).map_err(|err| err.to_string())?;

        if let Some(ref config_dir_path) = *CONFIG_DIR {
            dir_watcher
                .watch(&config_dir_path, RecursiveMode::Recursive)
                .map_err(|err| err.to_string())?;
        } else {
            return Err("Could not get parent directory of config file".into());
        }

        if should_watch_changes {
            for (path, _) in watched_dirs.iter() {
                dir_watcher
                    .watch(&path, RecursiveMode::Recursive)
                    .map_err(|err| err.to_string())?;
            }
        }

        Ok(Self {
            config_path,
            watched_dirs,
            dir_watcher,
            config_change_listener: rx,
            is_watching_changes: should_watch_changes,
            dirs_with_changes: HashSet::new(),
        })
    }

    fn load_config<P: AsRef<Path>>(path: P) -> Result<HashMap<PathBuf, WatchDir>, String> {
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
        let path = watch_dir_conf.target_dir().to_owned();
        if self
            .watched_dirs
            .insert(path.to_owned(), watch_dir_conf)
            .is_none()
            && self.is_watching_changes
        {
            // Inserted for the first time, add to watch list
            match self.dir_watcher.watch(&path, RecursiveMode::Recursive) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Failed to register notify handler on dir {:?}: {e}", path);
                }
            }
        }
    }

    pub fn update_if_changed(&mut self) -> Result<(), String> {
        #[derive(serde::Deserialize)]
        struct JsonKeyValue {
            target_dir: String,
            watch_dir: WatchDir,
        }
        // TODO: Change the following line so we don't (semantically) recompute every function call
        let config_file_path: PathBuf = CONFIG_DIR
            .clone()
            .and_then(|f| Some(f.join("config.json")))
            .ok_or("Could not locate config directory")?;
        match self.config_change_listener.try_recv() {
            Ok(event_result) => {
                if let Ok(event) = event_result {
                    let path = event.paths.get(0).ok_or("Got notify event without path")?;

                    if let EventKind::Modify(modify_kind) = event.kind {
                        if let ModifyKind::Data(_) = modify_kind {
                            // TODO: Determine if the config.json file was changed or some other watched file
                            if path == &config_file_path {
                                // Config file was changed
                                let file = fs::File::open(&self.config_path)
                                    .map_err(|err| err.to_string())?;
                                let buf_reader = BufReader::new(file);
                                let serde_stream = Deserializer::from_reader(buf_reader)
                                    .into_iter::<serde_json::Value>();
                                for watch_dir_res in serde_stream {
                                    let serde_value =
                                        watch_dir_res.map_err(|err| err.to_string())?;
                                    if let serde_json::Value::Object(map) = serde_value {
                                        let (_, watch_dir_value) = map.into_iter().next().ok_or("Invalid config.json contents. Try clearing config with `timemctl clearconf`".to_string())?;
                                        let watch_dir: WatchDir =
                                            serde_json::from_value(watch_dir_value)
                                                .map_err(|err| err.to_string())?;
                                        let path = watch_dir.target_dir().to_owned();
                                        if self
                                            .watched_dirs
                                            .insert(path.clone(), watch_dir)
                                            .is_none()
                                        {
                                            // This is a new dir to watch!
                                            self.dir_watcher
                                                .watch(&path, RecursiveMode::Recursive)
                                                .map_err(|err| err.to_string())?;
                                        }
                                    }
                                }
                            } else {
                                // Some other watched directory file was changed. Add to the hash set
                                // TODO: COME BACK HERE COME BACK HERE COME BACK HERE COME BACK HERE.
                                self.dirs_with_changes.insert(path.to_owned());
                                log::trace!("Observed change in watched dir {:?}", path);
                            }
                        }
                    }
                }
            }
            Err(e) if matches!(e, TryRecvError::Empty) => {}
            Err(e) => {
                // TryRecvError::Disconnected
                exit_error!("Config file watching error: {e}");
            }
        }
        Ok(())
    }

    pub fn iter_changed_paths(&mut self) -> impl Iterator<Item = PathBuf> {
        std::mem::take(&mut self.dirs_with_changes).into_iter()
    }

    pub fn get_watched_dir_mut<P: AsRef<Path>>(&mut self, path: P) -> Option<&mut WatchDir> {
        let path = path.as_ref();
        dbg!(&self.watched_dirs);
        self.watched_dirs.get_mut(path)
    }
}
