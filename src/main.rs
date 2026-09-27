use std::process::ExitCode;

use anyhow::Result;
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
    RofiRbw::new()?.run()
}
