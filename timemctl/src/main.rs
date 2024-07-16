mod cli_args;
use cli_args::{Args, Command as ArgCommand};
use std::fs;
use timem::{exit_error, logger_init, Config, WatchDir, CONFIG_DIR};

use structopt::StructOpt;

fn main() {
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
            let add_cmd: WatchDir = match cli_add.into() {
                Ok(wdir) => wdir,
                Err(err_str) => {
                    exit_error!("Input error: {err_str}");
                }
            };

            config.add_watched_dir(add_cmd);
            match config.flush_config() {
                Ok(_) => {}
                Err(err_str) => {
                    exit_error!("Config flush error: {err_str}");
                }
            }
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
    }
}
