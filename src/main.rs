use std::process::ExitCode;

use anyhow::Result;
use rofi_rbw_rs::dispatch::{dispatch, Env, Mode};
use rofi_rbw_rs::RofiRbw;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log::error!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match dispatch(&args, &Env::detect()) {
        Mode::Pinentry => rofi_rbw_rs::pinentry::run(),
        Mode::PinentryClear => rofi_rbw_rs::pinentry::clear_stored(),
        Mode::Frontend => RofiRbw::new()?.run(),
    }
}
