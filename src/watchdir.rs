use git2::{build::CheckoutBuilder, Commit, Oid, Repository, RepositoryInitOptions};
use std::cell::Cell;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::MAIN_SEPARATOR;
use std::path::{Path, PathBuf};
use std::time::{self, Duration, Instant, SystemTime};

use anyhow::Error;

use serde::{Deserialize, Deserializer, Serialize};

use bstr::ByteSlice;

use humantime::format_duration;

use crate::{exit_error, DOTGIT_DIR_DIR};

#[derive(Serialize)]
pub struct WatchDir {
    target_dir: PathBuf,
    dotgit_dir: PathBuf,
    #[serde(skip)]
    repo: Repository,
    frequency: Duration,
    #[serde(skip)]
    last_snapshot_time: Cell<Instant>,
    max_file_size: u64,
}

#[derive(Deserialize)]
struct WatchDirHelper {
    target_dir: PathBuf,
    dotgit_dir: PathBuf,
    frequency: Duration,
    #[serde(skip, default = "Instant::cell_default")]
    last_snapshot_time: Cell<Instant>,
    max_file_size: u64,
}

impl WatchDir {
    pub fn new(
        target_dir: PathBuf,
        frequency: Duration,
        max_file_size: u64,
    ) -> Result<Self, Error> {
        let dir_os_str = target_dir
            .as_os_str()
            .as_encoded_bytes()
            .replace(b"_", b"__")
            .replace(MAIN_SEPARATOR.to_string(), b"_d_");
        if let Some(mut dotgit_dir) = DOTGIT_DIR_DIR.clone() {
            dotgit_dir.push(unsafe { OsStr::from_encoded_bytes_unchecked(dir_os_str.as_slice()) });
            let max_file_size = if max_file_size == 0 {
                u64::MAX
            } else {
                max_file_size
            };

            // If we have not created our .git directory for this watched dir yet
            if !dotgit_dir.exists() {
                std::fs::create_dir_all(&dotgit_dir)?;
                let mut opts = RepositoryInitOptions::new();
                opts.external_template(true).bare(false);

                let repo = Repository::init_opts(&target_dir, &opts)?;

                let repo_git_dir = repo.path().to_path_buf();
                let target_git_dir = Path::new(&dotgit_dir);

                std::fs::rename(&repo_git_dir, &target_git_dir)?;

                repo.set_workdir(Path::new(&target_dir), true)?;
            }

            let repo = Repository::open(&dotgit_dir)?;
            repo.set_workdir(&target_dir, false)?;

            Ok(Self {
                target_dir,
                frequency,
                max_file_size,
                dotgit_dir,
                repo,
                last_snapshot_time: Cell::new(Instant::now()),
            })
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

        let mut index = self.repo.index()?;

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
        let tree = self.repo.find_tree(oid)?;

        let time = SystemTime::now().duration_since(time::UNIX_EPOCH)?;
        let signature = self.repo.signature()?;
        let parents = if let Ok(head) = self.repo.head() {
            vec![head.peel_to_commit()?]
        } else {
            vec![]
        };
        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("TimeM snapshot at {:?}", time),
            &tree,
            &parents.iter().collect::<Vec<_>>(),
        )?;

        self.last_snapshot_time.set(Instant::now());

        log::info!(
            "Snapshotted directory {:?} to {:?}",
            &self.target_dir,
            &self.dotgit_dir
        );

        Ok(true)
    }

    pub fn restore_snapshot(
        &self,
        oid: Oid,
        restore_to: Option<impl AsRef<Path>>,
        force: bool,
    ) -> Result<(), Error> {
        let restore_to = restore_to
            .as_ref()
            .map(|p| p.as_ref())
            .unwrap_or(&self.target_dir);
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let mut checkout_builder = CheckoutBuilder::new();
        checkout_builder.target_dir(restore_to);
        if force {
            checkout_builder.force();
        }

        self.repo
            .checkout_tree(tree.as_object(), Some(&mut checkout_builder))?;

        log::info!(
            "Restored snapshot {:?} of {:?} to {:?}",
            oid,
            self.target_dir,
            restore_to
        );

        Ok(())
    }

    pub fn iter_oids(&self) -> Result<Vec<Result<Oid, git2::Error>>, Error> {
        self.repo.set_workdir(&self.target_dir, false)?;
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        Ok(revwalk.collect())
    }

    pub fn iter_commits(&self) -> Result<Vec<Result<Commit, git2::Error>>, Error> {
        self.repo.set_workdir(&self.target_dir, false)?;
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        Ok(revwalk
            .map(|oid_res| oid_res.and_then(|oid| self.repo.find_commit(oid)))
            .collect())
    }

    pub fn get_commit(&self, commit_hash: &str) -> Result<Commit, Error> {
        let oid = Oid::from_str(commit_hash)?;
        self.repo.find_commit(oid).map_err(|err| err.into())
    }

    pub fn get_repo(&self) -> &Repository {
        &self.repo
    }
}

impl<'de> Deserialize<'de> for WatchDir {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = WatchDirHelper::deserialize(deserializer)?;
        let repo = Repository::open(&helper.dotgit_dir).map_err(serde::de::Error::custom)?;
        repo.set_workdir(&helper.target_dir, false)
            .map_err(serde::de::Error::custom)?;

        Ok(WatchDir {
            target_dir: helper.target_dir,
            dotgit_dir: helper.dotgit_dir,
            repo,
            frequency: helper.frequency,
            last_snapshot_time: Instant::cell_default(),
            max_file_size: helper.max_file_size,
        })
    }
}

impl Display for WatchDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{} checked every {}",
            self.target_dir.display(),
            format_duration(self.frequency)
        )
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
