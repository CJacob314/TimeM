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
    /// Adds a directory to the watch list
    #[structopt(name = "watch")]
    Watch(CLIWatch),
    /// Completely removes config file (*warning*, this unwatches all watched directories)
    ClearConf,
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

        Ok(WatchDir::new(dir, frequency, max_file_size))
    }
}
