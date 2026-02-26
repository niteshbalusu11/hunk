use hunk::config::{AppConfig, KeyboardShortcuts, ThemePreference};

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn app_config_defaults_include_existing_keyboard_shortcuts() {
    let config = AppConfig::default();

    assert_eq!(
        config.keyboard_shortcuts.select_next_line,
        strings(&["down"])
    );
    assert_eq!(
        config.keyboard_shortcuts.select_previous_line,
        strings(&["up"])
    );
    assert_eq!(
        config.keyboard_shortcuts.extend_selection_next_line,
        strings(&["shift-down"])
    );
    assert_eq!(
        config.keyboard_shortcuts.extend_selection_previous_line,
        strings(&["shift-up"])
    );
    assert_eq!(
        config.keyboard_shortcuts.copy_selection,
        strings(&["cmd-c", "ctrl-c"])
    );
    assert_eq!(
        config.keyboard_shortcuts.select_all_diff_rows,
        strings(&["cmd-a", "ctrl-a"])
    );
    assert_eq!(config.keyboard_shortcuts.next_hunk, strings(&["f7"]));
    assert_eq!(
        config.keyboard_shortcuts.previous_hunk,
        strings(&["shift-f7"])
    );
    assert_eq!(config.keyboard_shortcuts.next_file, strings(&["alt-down"]));
    assert_eq!(
        config.keyboard_shortcuts.previous_file,
        strings(&["alt-up"])
    );
    assert_eq!(
        config.keyboard_shortcuts.open_project,
        strings(&["cmd-shift-o", "ctrl-shift-o"])
    );
    assert_eq!(
        config.keyboard_shortcuts.save_current_file,
        strings(&["cmd-s", "ctrl-s"])
    );
    assert_eq!(config.keyboard_shortcuts.quit_app, strings(&["cmd-q"]));
}

#[test]
fn app_config_parses_without_keyboard_shortcuts_field() {
    let raw = r#"
theme = "dark"
"#;
    let config: AppConfig =
        toml::from_str(raw).expect("config without keyboard_shortcuts should parse");

    assert_eq!(config.theme, ThemePreference::Dark);
    assert_eq!(config.keyboard_shortcuts, KeyboardShortcuts::default());
}

#[test]
fn app_config_applies_partial_shortcut_overrides() {
    let raw = r#"
[keyboard_shortcuts]
open_project = ["cmd-o", "ctrl-o"]
next_hunk = ["f8"]
"#;
    let config: AppConfig = toml::from_str(raw).expect("partial keyboard_shortcuts should parse");

    assert_eq!(
        config.keyboard_shortcuts.open_project,
        strings(&["cmd-o", "ctrl-o"])
    );
    assert_eq!(config.keyboard_shortcuts.next_hunk, strings(&["f8"]));
    assert_eq!(
        config.keyboard_shortcuts.save_current_file,
        strings(&["cmd-s", "ctrl-s"])
    );
}

#[test]
fn app_config_allows_disabling_shortcuts_with_empty_list() {
    let raw = r#"
[keyboard_shortcuts]
quit_app = []
"#;
    let config: AppConfig = toml::from_str(raw).expect("empty shortcut list should parse");

    assert!(config.keyboard_shortcuts.quit_app.is_empty());
    assert_eq!(
        config.keyboard_shortcuts.open_project,
        strings(&["cmd-shift-o", "ctrl-shift-o"])
    );
}
