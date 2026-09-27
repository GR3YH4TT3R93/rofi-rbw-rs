use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};

use crate::{
    config_file::build_config_argv,
    models::{Action, DisplayFieldToken, Keybinding, Target},
};

const DEFAULT_KEYBINDINGS: &str = concat!(
    "Alt+1:type:username:tab:password,",
    "Alt+2:type:username,",
    "Alt+3:type:password,",
    "Alt+4:type:totp,",
    "Alt+c:copy:password,",
    "Alt+u:copy:username,",
    "Alt+t:copy:totp,",
    "Alt+m::menu,",
    "Alt+s:sync",
);

const DEFAULT_MENU_KEYBINDINGS: &str = "Alt+t:type,Alt+c:copy";

#[derive(Parser, Debug)]
#[command(
    name = "rofi-rbw-rs",
    version,
    about = "Insert or copy passwords and usernames from Bitwarden using rofi or rofi-likes."
)]
struct Cli {
    #[arg(short = 'a', long = "action", default_value = "type", value_parser = parse_action)]
    action: Action,

    #[arg(short = 't', long = "target", action = ArgAction::Append)]
    targets: Vec<String>,

    #[arg(short = 'r', long = "prompt", default_value = "Choose entry")]
    prompt: String,

    #[arg(long = "selector-args", default_value = "")]
    selector_args: String,

    #[arg(long = "selector", value_parser = ["rofi", "wofi", "fuzzel", "bemenu"])]
    selector: Option<String>,

    #[arg(long = "clipboarder", value_parser = ["xsel", "xclip", "wl-copy"])]
    clipboarder: Option<String>,

    #[arg(long = "typer", value_parser = ["xdotool", "wtype", "ydotool", "dotool"])]
    typer: Option<String>,

    #[arg(long = "clear-after", default_value_t = 0)]
    clear: u64,

    #[arg(long = "no-help", action = ArgAction::SetTrue)]
    no_help: bool,

    #[arg(long = "no-folder", action = ArgAction::SetTrue, hide = true)]
    no_folder: bool,

    #[arg(long = "display-fields", num_args = 1.., value_parser = parse_display_field)]
    display_fields: Option<Vec<DisplayFieldToken>>,

    #[arg(long = "no-cache", action = ArgAction::SetTrue)]
    no_cache: bool,

    #[arg(long = "use-notify-send", action = ArgAction::SetTrue)]
    use_notify_send: bool,

    #[arg(long = "keybindings", default_value = DEFAULT_KEYBINDINGS)]
    keybindings: String,

    #[arg(long = "menu-keybindings", default_value = DEFAULT_MENU_KEYBINDINGS)]
    menu_keybindings: String,

    #[arg(long = "typing-key-delay", default_value_t = 0)]
    key_delay: u64,

    #[arg(long = "typing-start-delay", default_value_t = 0)]
    start_delay: u64,

    #[arg(long = "action-sequence-delay", default_value_t = 1000)]
    action_sequence_delay: u64,
}

#[derive(Debug, Clone)]
pub struct Args {
    pub action: Action,
    pub targets: Option<Vec<Target>>,
    pub prompt: String,
    pub selector_args: Vec<String>,
    pub selector: Option<String>,
    pub clipboarder: Option<String>,
    pub typer: Option<String>,
    pub clear: u64,
    pub show_help: bool,
    pub display_fields: Vec<DisplayFieldToken>,
    pub use_cache: bool,
    pub use_notify_send: bool,
    pub parsed_keybindings: Vec<Keybinding>,
    pub parsed_menu_keybindings: Vec<Keybinding>,
    pub key_delay: u64,
    pub start_delay: u64,
    pub action_sequence_delay: u64,
}

pub fn parse_arguments() -> Result<Args> {
    // Real argv, with the program name split off.
    let mut args: Vec<String> = std::env::args().collect();
    let prog = if args.is_empty() {
        "rofi-rbw-rs".to_string()
    } else {
        args.remove(0)
    };

    // Emit config-file contents as argv prefix. clap's last-wins and
    // Append accumulation give the right precedence: CLI > config.
    let config_argv = build_config_argv();

    let mut merged = Vec::with_capacity(1 + config_argv.len() + args.len());
    merged.push(prog);
    merged.extend(config_argv);
    merged.extend(args);

    let cli = Cli::parse_from(merged);
    convert(cli)
}

fn convert(cli: Cli) -> Result<Args> {
    let targets = if cli.targets.is_empty() {
        None
    } else {
        let parsed = cli
            .targets
            .iter()
            .map(|s| Target::parse(s).ok_or_else(|| anyhow!("invalid target '{s}'")))
            .collect::<Result<Vec<_>>>()?;
        Some(parsed)
    };

    let selector_args = shlex::split(&cli.selector_args)
        .ok_or_else(|| anyhow!("could not parse --selector-args (unbalanced quotes?)"))?;

    let display_fields = match cli.display_fields {
        Some(fields) => fields,
        None if cli.no_folder => {
            eprintln!(
                "warning: --no-folder is deprecated and will be removed in a future release. \
                 Use --display-fields name_only user instead."
            );
            vec![DisplayFieldToken::NameOnly, DisplayFieldToken::User]
        }
        None => vec![DisplayFieldToken::NameWithFolder, DisplayFieldToken::User],
    };

    Ok(Args {
        action: cli.action,
        targets,
        prompt: cli.prompt,
        selector_args,
        selector: cli.selector,
        clipboarder: cli.clipboarder,
        typer: cli.typer,
        clear: cli.clear,
        show_help: !cli.no_help,
        display_fields,
        use_cache: !cli.no_cache,
        use_notify_send: cli.use_notify_send,
        parsed_keybindings: parse_keybindings(&cli.keybindings)?,
        parsed_menu_keybindings: parse_menu_keybindings(&cli.menu_keybindings)?,
        key_delay: cli.key_delay,
        start_delay: cli.start_delay,
        action_sequence_delay: cli.action_sequence_delay,
    })
}

// ---- clap value parsers ---------------------------------------------------

fn parse_action(s: &str) -> Result<Action, String> {
    match s {
        "type" => Ok(Action::Type),
        "copy" => Ok(Action::Copy),
        "print" => Ok(Action::Print),
        "sync" => Ok(Action::Sync),
        _ => Err(format!(
            "invalid action '{s}', expected one of: type, copy, print, sync"
        )),
    }
}

fn parse_display_field(s: &str) -> Result<DisplayFieldToken, String> {
    DisplayFieldToken::from_raw(s).ok_or_else(|| format!("invalid display field '{s}'"))
}

// ---- keybinding parsers ---------------------------------------------------

fn parse_keybindings(spec: &str) -> Result<Vec<Keybinding>> {
    spec.split(',')
        .filter(|s| !s.is_empty())
        .map(|kb| {
            let parts: Vec<&str> = kb.split(':').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(anyhow!("invalid keybinding '{kb}': missing shortcut"));
            }
            let shortcut = parts[0].to_string();

            let action = match parts.get(1).copied() {
                Some("") | None => None,
                Some(a) => Some(parse_action(a).map_err(|e| anyhow!("{e}"))?),
            };

            let mut targets = Vec::new();
            for s in parts.iter().skip(2) {
                let t = Target::parse(s).ok_or_else(|| anyhow!("invalid target '{s}'"))?;
                targets.push(t);
            }

            Ok(Keybinding {
                shortcut,
                action,
                targets,
            })
        })
        .collect()
}

fn parse_menu_keybindings(spec: &str) -> Result<Vec<Keybinding>> {
    spec.split(',')
        .filter(|s| !s.is_empty())
        .map(|kb| {
            let parts: Vec<&str> = kb.split(':').collect();
            if parts.len() < 2 || parts[0].is_empty() {
                return Err(anyhow!(
                    "invalid menu keybinding '{kb}': expected <shortcut>:<action>"
                ));
            }
            let shortcut = parts[0].to_string();
            let action = if parts[1].is_empty() {
                None
            } else {
                Some(parse_action(parts[1]).map_err(|e| anyhow!("{e}"))?)
            };
            Ok(Keybinding {
                shortcut,
                action,
                targets: Vec::new(),
            })
        })
        .collect()
}
