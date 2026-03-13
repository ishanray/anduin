use crate::app::{
    ActivePane, ChangesFocus, HistoryFocus, Message, SidebarTab, State, ThemeMode,
};
use crate::update;
use iced::widget::Id;
use iced_code_editor::CodeEditor;
use std::collections::{HashMap, HashSet};

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

    let repo_path = std::env::current_dir().expect("current dir unavailable");
    let _ = update::update(&mut state, Message::RepoOpened(Some(repo_path)));

    assert!(!state.show_actions_panel);
}
