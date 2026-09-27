pub mod abstractionhelper;
pub mod args;
pub mod backend;
pub mod cache;
pub mod clipboarder;
pub mod config_file;
pub mod dispatch;
pub mod models;
pub mod notifier;
pub mod paths;
pub mod pinentry;
pub mod rofi_rbw;
pub mod selector;
pub mod typer;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use rofi_rbw::RofiRbw;
