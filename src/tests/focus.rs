use crate::app::{
    ActivePane, ChangesFocus, Commit, HistoryFocus, Message, SidebarTab, State, ThemeMode,
};
use crate::git::diff::{ChangedFile, FileStatus};
use crate::update;
use iced::widget::Id;
use iced_code_editor::CodeEditor;
use std::collections::{HashMap, HashSet};
use std::env;

fn test_state(files: Vec<ChangedFile>) -> State {
    let theme_mode = ThemeMode::Dark;
    let mut diff_editor = CodeEditor::new("", "diff");
    diff_editor.lose_focus();

    let mut state = State {
        repo_path: "/tmp/test-repo".into(),
        files,
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
        status_message_id: 0,
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
        zoom_level: 1.0,
    };

    state.sync_tree_state();
    state.ensure_rows_cached();
    state
}

fn changed_file(path: &str, status: FileStatus) -> ChangedFile {
    ChangedFile {
        path: path.to_owned(),
        status,
        staged: false,
        unstaged: true,
    }
}

fn sample_commit() -> Commit {
    Commit {
        hash: "abc123".to_owned(),
        short_hash: "abc123".to_owned(),
        author: "Test User".to_owned(),
        date: "2026-03-10".to_owned(),
        message: "Test commit".to_owned(),
    }
}

#[test]
fn history_initial_load_batches_scroll_with_first_selection() {
    let mut state = test_state(Vec::new());
    state.sidebar_tab = SidebarTab::History;
    state.commits_loading = true;

    let task = update::update(
        &mut state,
        Message::CommitsLoaded(Ok(vec![sample_commit()])),
    );

    assert_eq!(state.selected_commit, Some(0));
    assert_eq!(task.units(), 2);
}

#[test]
fn repo_opened_resets_changes_to_file_list() {
    let mut state = test_state(Vec::new());
    state.active_pane = ActivePane::Diff;
    state.changes_focus = ChangesFocus::DiffView;
    state.sidebar_tab = SidebarTab::History;
    state.history_focus = HistoryFocus::DiffView;

    let repo_path = match env::current_dir() {
        Ok(path) => path,
        Err(error) => panic!("current dir unavailable: {error}"),
    };
    let _ = update::update(&mut state, Message::RepoOpened(Some(repo_path)));

    assert_eq!(state.sidebar_tab, SidebarTab::Changes);
    assert_eq!(state.active_pane, ActivePane::Sidebar);
    assert_eq!(state.changes_focus, ChangesFocus::FileList);
    assert_eq!(state.history_focus, HistoryFocus::CommitList);
    assert!(state.current_diff.is_none());
}

#[test]
fn switching_back_to_changes_resets_focus_to_file_list() {
    let mut state = test_state(vec![changed_file("src/main.rs", FileStatus::Modified)]);
    state.sidebar_tab = SidebarTab::History;
    state.active_pane = ActivePane::Diff;
    state.changes_focus = ChangesFocus::DiffView;
    state.history_focus = HistoryFocus::DiffView;
    state.commits = vec![sample_commit()];

    let _ = update::update(&mut state, Message::SwitchSidebarTab(SidebarTab::Changes));

    assert_eq!(state.sidebar_tab, SidebarTab::Changes);
    assert_eq!(state.active_pane, ActivePane::Sidebar);
    assert_eq!(state.changes_focus, ChangesFocus::FileList);
    assert!(state.history_diff.is_none());
    assert!(state.history_commit_header.is_none());
    assert!(state.focused_sidebar_target.is_some());
}
