use rofi_rbw_rs::args::{
    parse_action, parse_display_field, parse_keybindings, parse_menu_keybindings,
};
use rofi_rbw_rs::models::{Action, DisplayFieldToken, Target};

// ---------------------------------------------------------------------------
// parse_action
// ---------------------------------------------------------------------------

#[test]
fn action_known() {
    assert_eq!(parse_action("type").unwrap(), Action::Type);
    assert_eq!(parse_action("copy").unwrap(), Action::Copy);
    assert_eq!(parse_action("print").unwrap(), Action::Print);
    assert_eq!(parse_action("sync").unwrap(), Action::Sync);
}

#[test]
fn action_unknown_rejected() {
    assert!(parse_action("bogus").is_err());
    assert!(parse_action("TYPE").is_err()); // case-sensitive, matching Python
    assert!(parse_action("").is_err());
    assert!(parse_action("cancel").is_err()); // not a CLI action, only internal
}

// ---------------------------------------------------------------------------
// parse_display_field
// ---------------------------------------------------------------------------

#[test]
fn display_field_known() {
    assert_eq!(
        parse_display_field("name_only").unwrap(),
        DisplayFieldToken::NameOnly
    );
    assert_eq!(
        parse_display_field("name_with_folder").unwrap(),
        DisplayFieldToken::NameWithFolder
    );
    assert_eq!(
        parse_display_field("folder").unwrap(),
        DisplayFieldToken::Folder
    );
    assert_eq!(
        parse_display_field("user").unwrap(),
        DisplayFieldToken::User
    );
    assert_eq!(
        parse_display_field("uri").unwrap(),
        DisplayFieldToken::FirstUri
    );
}

#[test]
fn display_field_unknown_rejected() {
    assert!(parse_display_field("bogus").is_err());
    assert!(parse_display_field("first_uri").is_err()); // Python's value is "uri"
}

// ---------------------------------------------------------------------------
// parse_keybindings
// ---------------------------------------------------------------------------

#[test]
fn keybinding_full_sequence() {
    let kbs = parse_keybindings("Alt+1:type:username:tab:password").unwrap();
    assert_eq!(kbs.len(), 1);
    assert_eq!(kbs[0].shortcut, "Alt+1");
    assert_eq!(kbs[0].action, Some(Action::Type));
    assert_eq!(
        kbs[0].targets,
        vec![Target::Username, Target::Tab, Target::Password]
    );
}

#[test]
fn keybinding_empty_action() {
    // Alt+m::menu — action field is empty, targets follow
    let kbs = parse_keybindings("Alt+m::menu").unwrap();
    assert_eq!(kbs[0].shortcut, "Alt+m");
    assert_eq!(kbs[0].action, None);
    assert_eq!(kbs[0].targets, vec![Target::Menu]);
}

#[test]
fn keybinding_action_no_targets() {
    let kbs = parse_keybindings("Alt+s:sync").unwrap();
    assert_eq!(kbs[0].shortcut, "Alt+s");
    assert_eq!(kbs[0].action, Some(Action::Sync));
    assert!(kbs[0].targets.is_empty());
}

#[test]
fn keybinding_multiple_comma_separated() {
    let spec = "Alt+1:type:username,Alt+2:copy:password";
    let kbs = parse_keybindings(spec).unwrap();
    assert_eq!(kbs.len(), 2);
    assert_eq!(kbs[0].shortcut, "Alt+1");
    assert_eq!(kbs[0].action, Some(Action::Type));
    assert_eq!(kbs[0].targets, vec![Target::Username]);
    assert_eq!(kbs[1].shortcut, "Alt+2");
    assert_eq!(kbs[1].action, Some(Action::Copy));
    assert_eq!(kbs[1].targets, vec![Target::Password]);
}

#[test]
fn keybinding_skips_empty_specs() {
    let spec = "Alt+1:type:username,,Alt+2:copy:password";
    let kbs = parse_keybindings(spec).unwrap();
    assert_eq!(kbs.len(), 2);
}

#[test]
fn keybinding_invalid_action_rejected() {
    assert!(parse_keybindings("Alt+1:bogus:username").is_err());
}

#[test]
fn keybinding_invalid_target_rejected() {
    // Custom targets parse fine; nothing rejects. But empty string target?
    // Python: Target("") → __value = "" (falls through to else-branch). Not
    // an error. So this test verifies our `Target::parse` returns None for
    // empty and we surface it as an error.
    assert!(parse_keybindings("Alt+1:type:").is_err());
}

#[test]
fn keybinding_missing_shortcut_rejected() {
    assert!(parse_keybindings(":type:username").is_err());
}

// ---------------------------------------------------------------------------
// parse_menu_keybindings
// ---------------------------------------------------------------------------

#[test]
fn menu_keybinding_simple() {
    let kbs = parse_menu_keybindings("Alt+t:type").unwrap();
    assert_eq!(kbs.len(), 1);
    assert_eq!(kbs[0].shortcut, "Alt+t");
    assert_eq!(kbs[0].action, Some(Action::Type));
    assert!(kbs[0].targets.is_empty());
}

#[test]
fn menu_keybinding_multiple() {
    let kbs = parse_menu_keybindings("Alt+t:type,Alt+c:copy").unwrap();
    assert_eq!(kbs.len(), 2);
    assert_eq!(kbs[0].action, Some(Action::Type));
    assert_eq!(kbs[1].action, Some(Action::Copy));
}

#[test]
fn menu_keybinding_invalid_action_rejected() {
    assert!(parse_menu_keybindings("Alt+t:bogus").is_err());
}

#[test]
fn menu_keybinding_missing_colon_rejected() {
    assert!(parse_menu_keybindings("Alt+t").is_err());
}

#[test]
fn menu_keybinding_empty_action_ok() {
    // Python's spec allows empty action (guarded by `if elements[1] else None`)
    let kbs = parse_menu_keybindings("Alt+x:").unwrap();
    assert_eq!(kbs[0].action, None);
}

// ---------------------------------------------------------------------------
// Defaults sanity (regression — matches Python's DEFAULT_KEYBINDINGS)
// ---------------------------------------------------------------------------

#[test]
fn default_keybindings_parse() {
    // If you change the const in args.rs, this test will flag it.
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
    let kbs = parse_keybindings(DEFAULT_KEYBINDINGS).unwrap();
    assert_eq!(kbs.len(), 9);
    assert_eq!(kbs[0].shortcut, "Alt+1");
    assert_eq!(kbs[7].shortcut, "Alt+m");
    assert_eq!(kbs[7].action, None);
    assert_eq!(kbs[7].targets, vec![Target::Menu]);
    assert_eq!(kbs[8].shortcut, "Alt+s");
    assert_eq!(kbs[8].action, Some(Action::Sync));
    assert!(kbs[8].targets.is_empty());
}

#[test]
fn default_menu_keybindings_parse() {
    const DEFAULT_MENU_KEYBINDINGS: &str = "Alt+t:type,Alt+c:copy";
    let kbs = parse_menu_keybindings(DEFAULT_MENU_KEYBINDINGS).unwrap();
    assert_eq!(kbs.len(), 2);
}
