use crate::app::{
    ActivePane, ChangesFocus, Commit, HistoryFocus, Message, SidebarTab, State, ThemeMode,
};
use crate::git::diff::FileDiff;
use crate::update;
use iced::keyboard;
use iced::widget::Id;
use iced_code_editor::CodeEditor;
use std::collections::{HashMap, HashSet};
use std::env;

fn key_event(key: keyboard::Key) -> Message {
    Message::KeyboardEvent(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    })
}

fn sample_commit() -> Commit {
    Commit {
        hash: "abc1234deadbeef".to_owned(),
        short_hash: "abc1234".to_owned(),
        author: "Test User".to_owned(),
        date: "2026-03-12".to_owned(),
        message: "Test commit".to_owned(),
    }
}

fn test_state() -> State {
    let theme_mode = ThemeMode::Dark;
    let mut diff_editor = CodeEditor::new("", "diff");
    diff_editor.lose_focus();

    State {
        repo_path: "/tmp/test-repo".into(),
        files: Vec::new(),
        selected_file: None,
        selected_path: None,
        focused_sidebar_target: None,
        selected_sidebar_targets: HashSet::new(),
        selection_anchor_sidebar_target: None,
        current_diff: None,
        diff_editor,
        theme_mode,
        error: None,
        status_message: None,
        commit_composer: None,
        expanded_dirs: HashSet::new(),
        tree_root_expanded: true,
        alt_pressed: false,
        initialized_tree: false,
        cached_rows: Vec::new(),
        tree_dirty: true,
        refresh_in_flight: false,
        refresh_queued: false,
        active_diff_request: 0,
        diff_search_cache: HashMap::new(),
        scroll_positions: HashMap::new(),
        project_search: None,
        pending_diff_jump: None,
        sidebar_scroll_id: Id::unique(),
        sidebar_scroll_offset: 0.0,
        sidebar_viewport_height: 0.0,
        active_pane: ActivePane::Sidebar,
        cached_theme: theme_mode.app_theme(),
        show_actions_panel: false,
        current_branch: None,
        branch_picker: None,
        project_picker: None,
        recent_repos: Vec::new(),
        sidebar_tab: SidebarTab::Changes,
        commits: Vec::new(),
        selected_commit: None,
        commit_files: Vec::new(),
        commits_loading: false,
        commits_exhausted: false,
        history_selected_file: None,
        history_selected_path: None,
        history_diff: None,
        history_commit_header: None,
        history_focus: HistoryFocus::CommitList,
        changes_focus: ChangesFocus::FileList,
        commit_list_scroll_id: Id::unique(),
        commit_list_scroll_offset: 0.0,
        commit_list_viewport_height: 0.0,
        discard_confirm: None,
        sidebar_context_menu: None,
        window_size: None,
        pending_settings_save: None,
    }
}

#[test]
fn toggle_actions_panel_flips_open_state() {
    let mut state = test_state();

    let _ = update::update(&mut state, Message::ToggleActionsPanel);
    assert!(state.show_actions_panel);

    let _ = update::update(&mut state, Message::ToggleActionsPanel);
    assert!(!state.show_actions_panel);
}

#[test]
fn repo_opened_closes_actions_panel() {
    let mut state = test_state();
    state.show_actions_panel = true;

    let repo_path = match env::current_dir() {
        Ok(path) => path,
        Err(error) => panic!("current dir unavailable: {error}"),
    };
    let _ = update::update(&mut state, Message::RepoOpened(Some(repo_path)));

    assert!(!state.show_actions_panel);
}

#[test]
fn escape_closes_actions_panel_before_other_navigation() {
    let mut state = test_state();
    state.show_actions_panel = true;

    let _ = update::update(
        &mut state,
        Message::KeyboardEvent(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }),
    );

    assert!(!state.show_actions_panel);
}

#[test]
fn actions_panel_b_starts_opening_branch_picker() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;

    let task = update::update(&mut state, key_event(keyboard::Key::Character("b".into())));

    assert!(task.units() > 0);
}

#[test]
fn actions_panel_p_opens_project_picker() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("p".into())));

    assert!(state.project_picker.is_some());
}

#[test]
fn actions_panel_h_switches_to_history_tab() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("h".into())));

    assert_eq!(state.sidebar_tab, SidebarTab::History);
}

#[test]
fn actions_panel_y_copies_hash_from_history_context() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;
    state.sidebar_tab = SidebarTab::History;
    state.history_commit_header = Some(sample_commit());

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("y".into())));

    assert_eq!(
        state.status_message.as_ref().map(|message| message.text.as_str()),
        Some("Copied commit hash")
    );
}

#[test]
fn actions_panel_f_opens_project_search() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("f".into())));

    assert!(state.project_search.is_some());
}

#[test]
fn actions_panel_o_starts_open_repo_flow_without_repo() {
    let mut state = test_state();
    state.show_actions_panel = true;

    let task = update::update(&mut state, key_event(keyboard::Key::Character("o".into())));

    assert!(task.units() > 0);
}

#[test]
fn actions_panel_slash_opens_diff_search_in_changes() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;
    state.current_diff = Some(FileDiff {
        path: "src/main.rs".to_owned(),
        raw_patch: "@@ -1 +1 @@\n-old\n+new\n".to_owned(),
    });

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("/".into())));

    assert!(!state.show_actions_panel);
    assert_eq!(state.changes_focus, ChangesFocus::DiffView);
    assert!(state.diff_editor.is_search_open());
}

#[test]
fn actions_panel_slash_opens_diff_search_in_history() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;
    state.sidebar_tab = SidebarTab::History;
    state.history_focus = HistoryFocus::FileList;
    state.history_diff = Some(FileDiff {
        path: "src/main.rs".to_owned(),
        raw_patch: "@@ -1 +1 @@\n-old\n+new\n".to_owned(),
    });

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("/".into())));

    assert!(!state.show_actions_panel);
    assert_eq!(state.history_focus, HistoryFocus::DiffView);
    assert!(state.diff_editor.is_search_open());
}

#[test]
fn actions_panel_c_opens_commit_composer_and_closes_panel() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("c".into())));

    assert!(state.commit_composer.is_some());
    assert!(!state.show_actions_panel);
}

#[test]
fn commit_composer_t_does_not_switch_tabs_after_opening_from_actions() {
    let mut state = test_state();
    state.current_branch = Some("main".to_owned());
    state.show_actions_panel = true;

    let _ = update::update(&mut state, key_event(keyboard::Key::Character("c".into())));
    let _ = update::update(&mut state, key_event(keyboard::Key::Character("t".into())));

    assert_eq!(state.sidebar_tab, SidebarTab::Changes);
    assert!(state.commit_composer.is_some());
}
