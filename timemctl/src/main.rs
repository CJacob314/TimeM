mod cli_args;
use cli_args::{Args, Command as ArgCommand};
use std::fs;
use std::path::{Path, PathBuf};
use timem::{exit_error, logger_init, Config, WatchDir, CONFIG_DIR};

use structopt::StructOpt;

use git2::{self, DiffFormat, Oid};

use chrono::{DateTime, Local, TimeZone, Utc};

use anyhow::Error;

fn main() -> Result<(), Error> {
    logger_init();
    let args = Args::from_args();
    let mut config = match Config::new(false) {
        Ok(config) => config,
        Err(err_str) => {
            exit_error!("Config error: {err_str}");
        }
    };

    match args.cmd {
        ArgCommand::Watch(cli_add) => {
            let watch_dir: WatchDir = match cli_add.into() {
                Ok(wdir) => wdir,
                Err(err_str) => {
                    exit_error!("Input error: {err_str}");
                }
            };

            config.add_watched_dir(watch_dir);
            match config.flush_config() {
                Ok(_) => {}
                Err(err_str) => {
                    exit_error!("Config flush error: {err_str}");
                }
            }
        }
        ArgCommand::Log(log) => {
            let dir = Path::new(&log.dir)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&log.dir));
            let watch_dir = config.get_watched_dir(&dir).ok_or(Error::msg(format!(
                "Directory {:?} is not being watched",
                dir
            )))?;

            watch_dir
                .iter_commits()?
                .iter()
                .filter_map(|res| res.as_ref().ok())
                .for_each(|commit| {
                    println!(
                        "{} {:?}",
                        commit.id(),
                        format_git2_time(&commit.time()).expect("git2 gave invalid time")
                    )
                });
        }
        ArgCommand::ClearConf => {
            if let Some(ref config_dir) = *CONFIG_DIR {
                let config_file = config_dir.join("config.json");
                match fs::remove_file(config_file) {
                    Ok(_) => {}
                    Err(e) => {
                        exit_error!("Failed to remove config file: {e}");
                    }
                }
            } else {
                exit_error!("Failed to find config directory");
            }
        }
        ArgCommand::List => {
            config
                .iter_watched_dirs()
                .for_each(|watch_dir| println!("{}", watch_dir));
        }
        ArgCommand::Diff(diff) => {
            let dir = Path::new(&diff.dir)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&diff.dir));
            let watch_dir = config.get_watched_dir(dir).ok_or(Error::msg(format!(
                "Directory {:?} is not being watched",
                diff.dir
            )))?;

            let commit_one = watch_dir.get_commit(&diff.snapshot_hash_one)?;
            let commit_two = match diff.snapshot_hash_two {
                Some(hash) => watch_dir.get_commit(&hash)?,
                None => commit_one.parent(0)?,
            };

            let repo = watch_dir.get_repo();
            let diff =
                repo.diff_tree_to_tree(Some(&commit_two.tree()?), Some(&commit_one.tree()?), None)?;

            diff.print(DiffFormat::Patch, |_, _, line| {
                match line.origin() {
                    c if c == '+' || c == '>' => print!("\x1b[48;5;28m{c}"),
                    c if c == '-' || c == '<' => print!("\x1b[48;5;88m{c}"),
                    _ => {}
                }
                print!("{}\x1b[0m", String::from_utf8_lossy(line.content()));
                true
            })?;
        }
        ArgCommand::Restore(restore) => {
            let dir = Path::new(&restore.dir)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&restore.dir));
            let watch_dir = config.get_watched_dir(&dir).ok_or(Error::msg(format!(
                "The directory {:?} is not watched",
                &dir
            )))?;

            let oid = Oid::from_str(&restore.hash)?;
            watch_dir.restore_snapshot(oid, restore.to.as_deref(), restore.force)?;
        }
    }

    Ok(())
}

fn format_git2_time(time: &git2::Time) -> Result<String, Error> {
    // Convert the timestamp to NaiveDateTime
    let naive = DateTime::from_timestamp(time.seconds(), 0)
        .ok_or(Error::msg("git2 time out-of-range"))?
        .naive_utc();
    // Convert NaiveDateTime to DateTime<Utc>
    let datetime_utc = Utc.from_utc_datetime(&naive);
    // Convert to local time
    let datetime_local = datetime_utc.with_timezone(&Local);

    // Format the local time
    let formatted_time = datetime_local.format("%Y-%m-%d %H:%M:%S");

    // Calculate and format the timezone offset
    let offset_minutes = time.offset_minutes();
    let offset_hours = offset_minutes / 60;
    let offset_minutes = offset_minutes % 60;
    let sign = if offset_hours < 0 { '-' } else { '+' };

    Ok(format!(
        "{} {}{:02}{:02}",
        formatted_time,
        sign,
        offset_hours.abs(),
        offset_minutes
    ))
}
