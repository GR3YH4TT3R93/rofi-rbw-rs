use rofi_rbw_rs::config_file::parse_content;

fn parse(s: &str) -> Vec<String> {
    parse_content(s)
}

// ---------------------------------------------------------------------------
// Separators
// ---------------------------------------------------------------------------

#[test]
fn equals_separator() {
    assert_eq!(parse("selector = rofi"), vec!["--selector", "rofi"]);
}

#[test]
fn colon_separator() {
    assert_eq!(parse("selector: rofi"), vec!["--selector", "rofi"]);
}

#[test]
fn whitespace_separator() {
    assert_eq!(parse("selector rofi"), vec!["--selector", "rofi"]);
}

#[test]
fn equals_with_no_spaces() {
    assert_eq!(parse("selector=rofi"), vec!["--selector", "rofi"]);
}

#[test]
fn colon_with_no_spaces() {
    assert_eq!(parse("selector:rofi"), vec!["--selector", "rofi"]);
}

// ---------------------------------------------------------------------------
// Key prefixes
// ---------------------------------------------------------------------------

#[test]
fn bare_key_becomes_long_option() {
    assert_eq!(parse("selector = rofi"), vec!["--selector", "rofi"]);
}

#[test]
fn explicit_double_dash_key() {
    assert_eq!(parse("--selector = rofi"), vec!["--selector", "rofi"]);
}

#[test]
fn explicit_single_dash_key() {
    assert_eq!(parse("-a = copy"), vec!["-a", "copy"]);
}

// ---------------------------------------------------------------------------
// Comments, sections, blank lines
// ---------------------------------------------------------------------------

#[test]
fn hash_comment_ignored() {
    assert_eq!(
        parse("# comment\nselector = rofi"),
        vec!["--selector", "rofi"]
    );
}

#[test]
fn semicolon_comment_ignored() {
    assert_eq!(
        parse("; comment\nselector = rofi"),
        vec!["--selector", "rofi"]
    );
}

#[test]
fn section_header_ignored() {
    assert_eq!(
        parse("[general]\nselector = rofi"),
        vec!["--selector", "rofi"]
    );
}

#[test]
fn blank_lines_ignored() {
    assert_eq!(parse("\n\nselector = rofi\n\n"), vec!["--selector", "rofi"]);
}

// ---------------------------------------------------------------------------
// Booleans
// ---------------------------------------------------------------------------

#[test]
fn bare_boolean_key_is_on() {
    assert_eq!(parse("use-notify-send"), vec!["--use-notify-send"]);
}

#[test]
fn boolean_true_values() {
    for v in ["true", "yes", "1", "on", "TRUE", "Yes", "ON"] {
        assert_eq!(
            parse(&format!("use-notify-send = {v}")),
            vec!["--use-notify-send"],
            "failed for value {v}"
        );
    }
}

#[test]
fn boolean_false_values_emit_nothing() {
    for v in ["false", "no", "0", "off", "FALSE", "No", "OFF"] {
        assert_eq!(
            parse(&format!("use-notify-send = {v}")),
            Vec::<String>::new(),
            "failed for value {v}"
        );
    }
}

// ---------------------------------------------------------------------------
// Values: quoting, multi-value, continuation
// ---------------------------------------------------------------------------

#[test]
fn value_with_spaces_via_quotes() {
    assert_eq!(
        parse(r#"prompt = "Choose an entry""#),
        vec!["--prompt", "Choose an entry"]
    );
}

#[test]
fn value_with_commas_via_quotes() {
    assert_eq!(
        parse(r#"keybindings = "Alt+1:type:username, Alt+c:copy:password""#),
        vec!["--keybindings", "Alt+1:type:username, Alt+c:copy:password"]
    );
}

#[test]
fn multi_value_arg_split() {
    assert_eq!(
        parse("display-fields = name_with_folder user"),
        vec!["--display-fields", "name_with_folder", "user"]
    );
}

#[test]
fn numeric_value() {
    assert_eq!(parse("clear-after = 30"), vec!["--clear-after", "30"]);
}

#[test]
fn line_continuation_joins() {
    let input = "keybindings = Alt+1:type:username,\\\nAlt+c:copy:password";
    assert_eq!(
        parse(input),
        vec!["--keybindings", "Alt+1:type:username,Alt+c:copy:password"]
    );
}

// ---------------------------------------------------------------------------
// Realistic full config
// ---------------------------------------------------------------------------

#[test]
fn realistic_config() {
    let input = "\
# rofi-rbw config
[general]

selector = rofi
prompt   = \"Pick one\"
clear-after = 30
use-notify-send
no-cache = false
display-fields = name_with_folder user
";
    assert_eq!(
        parse(input),
        vec![
            "--selector",
            "rofi",
            "--prompt",
            "Pick one",
            "--clear-after",
            "30",
            "--use-notify-send",
            "--display-fields",
            "name_with_folder",
            "user",
        ]
    );
}
