mod cli_args;
mod config_editor;
mod macros;
use cli_args::{Args, Command as ArgCommand, WatchDir};
use config_editor::ConfigEditor;

use structopt::StructOpt;

fn main() {
    let args = Args::from_args();
    let mut config = match ConfigEditor::new() {
        Ok(config) => config,
        Err(err_str) => {
            exit_error!("Config error: {err_str}");
        }
    };

    match args.cmd {
        ArgCommand::Add(cli_add) => {
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
    }
}
