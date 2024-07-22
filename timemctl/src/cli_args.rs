use std::path::PathBuf;

use structopt::clap::AppSettings;
use structopt::StructOpt;

use humantime::parse_duration;
use parse_size::parse_size;

use crate::WatchDir;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "timemctl", about = "TimeM configuration tool",
    global_settings = &[AppSettings::ColoredHelp]
)]
pub struct Args {
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "watch")]
    /// Adds a directory to the watch list
    Watch(CLIWatch),
    #[structopt(name = "list")]
    /// Lists all watched directories
    List,
    #[structopt(name = "log")]
    /// List all snapshots for a directory
    Log(CLILog),
    #[structopt(name = "diff")]
    /// Show the differences between two snapshots (or one snapshot to the last)
    Diff(CLIDiff),
    #[structopt(name = "restore")]
    /// Restore a snapshot
    Restore(CLIRestore),
    /// Completely removes config file (*warning*, this unwatches all watched directories)
    ClearConf,
}

#[derive(Debug, StructOpt)]
pub struct CLIRestore {
    #[structopt()]
    /// The snapshotted directory
    pub dir: String,
    #[structopt()]
    /// The snapshot hash to restore
    pub hash: String,
    #[structopt(short, long)]
    /// If provided, the snapshot will be restored to this directory instead of the snapshot origin directory
    pub to: Option<String>,
    #[structopt(short, long)]
    /// If provided, the snapshot restore will overwrite any existing files in the target directory
    pub force: bool,
}

#[derive(Debug, StructOpt)]
pub struct CLIDiff {
    #[structopt(short, long, default_value = ".")]
    /// The directory to diff
    pub dir: String,
    #[structopt()]
    /// An initial snapshot hash to diff
    pub snapshot_hash_one: String,
    #[structopt()]
    /// A second snapshot hash to diff against (if not provided, the snapshot before the first provided hash will be used)
    pub snapshot_hash_two: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct CLIWatch {
    #[structopt()]
    /// The directory to add to the watch list
    dir: String,
    #[structopt()]
    /// How often (e.g., 1h30m, 1d, 5m30s, etc.) to automatically take a snapshot of the directory
    /// (snapshots are only take if files have changed)
    frequency: String,
    #[structopt(short, long)]
    /// Max file size to sync inside the folder (files above this size will not be snapshotted).
    /// The default is 0, meaning all files will be snapshotted (e.g., 0.2 MiB, 2G, 128kb, etc.)
    max_file_size: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct CLILog {
    #[structopt(short, long, default_value = ".")]
    /// The directory for which list snapshots
    pub dir: String,
}

impl From<CLIWatch> for Result<WatchDir, String> {
    fn from(value: CLIWatch) -> Self {
        let dir = PathBuf::from(value.dir);
        if !dir.exists() {
            return Err("Path doesn't exist".into());
        }
        if !dir.is_dir() {
            return Err("Path is not a directory".into());
        }

        let frequency = parse_duration(&value.frequency).map_err(|err| err.to_string())?;

        let max_file_size = parse_size(value.max_file_size.unwrap_or("0B".into()))
            .map_err(|err| err.to_string())?;

        Ok(WatchDir::new(dir, frequency, max_file_size).map_err(|err| err.to_string())?)
    }
}
