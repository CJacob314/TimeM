use std::path::PathBuf;
use std::time::Duration;

use structopt::clap::AppSettings;
use structopt::StructOpt;

use serde::{Deserialize, Serialize};

use humantime::parse_duration;
use parse_size::parse_size;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "timemctl", about = "TimeM configuration tool",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp]
)]
pub struct Args {
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Adds a directory to the watch list
    Add(CLIAdd),
}

#[derive(Debug, StructOpt)]
pub struct CLIAdd {
    #[structopt()]
    /// The directory to add to the watch list
    dir: String,
    #[structopt()]
    /// How often to automatically take a snapshot of the directory
    /// (e.g., 1h30m, 1d, 5m30s, etc.)
    frequency: String,
    #[structopt(short, long)]
    /// Max file size to sync inside the folder (files above this size will not be snapshotted).
    /// The default is 0, meaning all files will be snapshotted (e.g., 0.2 MiB, 2G, 128kb, etc.)
    max_file_size: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct WatchDir {
    dir: PathBuf,
    frequency: Duration,
    max_file_size: u64,
}

impl From<CLIAdd> for Result<WatchDir, String> {
    fn from(value: CLIAdd) -> Self {
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

        Ok(WatchDir {
            dir,
            frequency,
            max_file_size,
        })
    }
}
