use git2::{Repository, RepositoryInitOptions};
use std::cell::Cell;
use std::ffi::OsStr;
use std::fs;
use std::path::MAIN_SEPARATOR;
use std::path::{Path, PathBuf};
use std::time::{self, Duration, Instant, SystemTime};

use anyhow::Error;

use serde::{Deserialize, Serialize};

use bstr::ByteSlice;

use crate::{exit_error, DOTGIT_DIR_DIR};

#[derive(Debug, Serialize, Deserialize)]
pub struct WatchDir {
    target_dir: PathBuf,
    dotgit_dir: PathBuf,
    frequency: Duration,
    #[serde(skip, default = "Instant::cell_default")]
    last_snapshot_time: Cell<Instant>,
    max_file_size: u64,
}

impl WatchDir {
    pub fn new(target_dir: PathBuf, frequency: Duration, max_file_size: u64) -> Self {
        let dir_os_str = target_dir
            .as_os_str()
            .as_encoded_bytes()
            .replace(b"_", b"__")
            .replace(MAIN_SEPARATOR.to_string(), b"_d_");
        if let Some(mut dotgit_dir) = DOTGIT_DIR_DIR.clone() {
            dotgit_dir.push(unsafe { OsStr::from_encoded_bytes_unchecked(dir_os_str.as_slice()) });
            log::info!("dotgit_dir constructed as: {:?}", &dotgit_dir);
            let max_file_size = if max_file_size == 0 {
                u64::MAX
            } else {
                max_file_size
            };

            Self {
                target_dir,
                frequency,
                max_file_size,
                dotgit_dir,
                last_snapshot_time: Cell::new(Instant::now()),
            }
        } else {
            exit_error!("Could not locate OS config directory");
        }
    }

    pub fn target_dir(&self) -> &Path {
        self.target_dir.as_path()
    }

    pub fn snapshot(&self, force: bool) -> Result<bool, Error> {
        let last_snapshot_time = self.last_snapshot_time.get();
        if !force && Instant::now() < last_snapshot_time.checked_add(self.frequency).unwrap() {
            return Ok(false);
        }

        // If we have not created our .git directory for this watched dir yet
        if !self.dotgit_dir.exists() {
            std::fs::create_dir_all(&self.dotgit_dir)?;
            let mut opts = RepositoryInitOptions::new();
            opts.external_template(true).bare(false);

            let repo = Repository::init_opts(&self.target_dir, &opts)?;

            let repo_git_dir = repo.path().to_path_buf();
            let target_git_dir = Path::new(&self.dotgit_dir);

            std::fs::rename(&repo_git_dir, &target_git_dir)?;

            repo.set_workdir(Path::new(&self.target_dir), true)?;
        }

        log::info!("self.dotgit_dir: {:?}", &self.dotgit_dir);
        let repo = Repository::open(&self.dotgit_dir)?;
        repo.set_workdir(&self.target_dir, false)?;
        let mut index = repo.index()?;

        for entry in fs::read_dir(&self.target_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .metadata()
                    .map(|meta| meta.len() <= self.max_file_size)
                    .unwrap_or(false)
            })
        {
            let path = entry.path();
            let relative_path = path.strip_prefix(&self.target_dir)?;
            index.add_path(relative_path)?;
        }

        index.write()?;

        let oid = index.write_tree()?;
        let tree = repo.find_tree(oid)?;

        let time = SystemTime::now().duration_since(time::UNIX_EPOCH)?;
        let signature = repo.signature()?;
        let parents = if let Ok(head) = repo.head() {
            vec![head.peel_to_commit()?]
        } else {
            vec![]
        };
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("TimeM snapshot at {:?}", time),
            &tree,
            &parents.iter().collect::<Vec<_>>(),
        )?;

        self.last_snapshot_time.set(Instant::now());

        log::debug!(
            "Snapshotted directory {:?} to {:?}",
            &self.target_dir,
            &self.dotgit_dir
        );

        Ok(true)
    }
}

pub trait CellDefault {
    fn cell_default() -> Cell<Self>;
}

impl CellDefault for Instant {
    fn cell_default() -> Cell<Self> {
        Cell::new(Instant::now())
    }
}
