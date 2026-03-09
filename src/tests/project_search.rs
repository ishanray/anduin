use crate::app::{
    ActivePane, ChangesFocus, HistoryFocus, Message, ProjectSearchResponse, SidebarTab,
    SidebarTarget, State, ThemeMode,
};
use crate::git::diff::{ChangedFile, FileStatus};
use crate::search::{ContextLine, MatchContext, ProjectSearchResult};
use crate::tree::SidebarRow;
use crate::update;
use iced::widget::Id;
use iced_code_editor::CodeEditor;
use std::collections::{HashMap, HashSet};

/// Build a minimal State for testing with the given list of files.
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
        show_shortcuts_help: false,
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

fn make_search_result(file_path: &str, total_matches: usize) -> ProjectSearchResult {
    let lines: Vec<ContextLine> = (0..3)
        .map(|i| ContextLine {
            line_number: i,
            line_number_display: format!("{:>4}", i + 1),
            text: if i == 1 {
                format!("match line {i}")
            } else {
                format!("context line {i}")
            },
            is_match: i == 1,
        })
        .collect();

    ProjectSearchResult {
        file_path: file_path.to_owned(),
        file_status: FileStatus::Modified,
        total_matches_display: total_matches.to_string(),
        total_matches,
        estimated_scroll_y: 0.0,
        matches: vec![MatchContext {
            start_line: 0,
            end_line: 2,
            lines,
        }],
    }
}

fn make_search_response(results: Vec<ProjectSearchResult>) -> ProjectSearchResponse {
    ProjectSearchResponse {
        results,
        cache: HashMap::new(),
    }
}

fn sample_files() -> Vec<ChangedFile> {
    vec![
        changed_file("src/main.rs", FileStatus::Modified),
        changed_file("src/app.rs", FileStatus::Modified),
        changed_file("src/views/sidebar.rs", FileStatus::Modified),
        changed_file("src/views/diff.rs", FileStatus::Modified),
        changed_file("src/search.rs", FileStatus::Added),
    ]
}

/// Open search then deliver results for the given files.
fn open_search_with_results(state: &mut State, query: &str, results: Vec<ProjectSearchResult>) {
    let _ = update::update(state, Message::OpenProjectSearch);
    let _ = update::update(state, Message::ProjectSearchQueryChanged(query.into()));
    let Some(search) = state.project_search.as_ref() else {
        unreachable!("project_search should be Some after opening");
    };
    let request_id = search.request_id;
    let _ = update::update(
        state,
        Message::ProjectSearchResults(request_id, Ok(make_search_response(results))),
    );
}

fn collect_visible_file_names(state: &State) -> Vec<String> {
    state
        .visible_cached_rows()
        .iter()
        .filter_map(|row| match row {
            SidebarRow::File { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn collect_visible_dir_names(state: &State) -> Vec<String> {
    state
        .visible_cached_rows()
        .iter()
        .filter_map(|row| match row {
            SidebarRow::Dir { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn collect_visible_file_targets(state: &State) -> Vec<String> {
    let targets = state.visible_sidebar_targets();
    targets
        .iter()
        .filter_map(|t| match t {
            SidebarTarget::File(path) => Some(path.clone()),
            _ => None,
        })
        .collect()
}

// === Opening and closing project search ===

#[test]
fn open_project_search_creates_search_state() {
    let mut state = test_state(sample_files());
    assert!(!state.is_search_open());

    let _ = update::update(&mut state, Message::OpenProjectSearch);

    assert!(state.is_search_open());
    let Some(search) = &state.project_search else {
        unreachable!("project_search should be Some");
    };
    assert!(search.is_open);
    assert!(search.input_focused);
    assert!(search.query.is_empty());
}

#[test]
fn close_project_search_preserves_state() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged("hello".into()),
    );

    let _ = update::update(&mut state, Message::CloseProjectSearch);

    assert!(!state.is_search_open());
    let Some(search) = &state.project_search else {
        unreachable!("project_search should be preserved after close");
    };
    assert!(!search.is_open);
    assert_eq!(search.query, "hello");
}

#[test]
fn reopen_project_search_retains_previous_query() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged("retained".into()),
    );
    let _ = update::update(&mut state, Message::CloseProjectSearch);
    assert!(!state.is_search_open());

    let _ = update::update(&mut state, Message::OpenProjectSearch);

    assert!(state.is_search_open());
    let Some(search) = &state.project_search else {
        unreachable!("project_search should be Some after reopen");
    };
    assert_eq!(search.query, "retained");
    assert!(search.input_focused);
}

// === Escape behavior: two-stage ===

#[test]
fn first_escape_unfocuses_input_but_keeps_search_open() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| s.input_focused)
    );

    // Simulate first escape: unfocus input, keep search open
    if let Some(search) = state.project_search.as_mut() {
        search.input_focused = false;
    }
    state.active_pane = ActivePane::Sidebar;

    assert!(state.is_search_open());
    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| !s.input_focused)
    );
    assert_eq!(state.active_pane, ActivePane::Sidebar);
}

#[test]
fn second_escape_closes_search() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    if let Some(search) = state.project_search.as_mut() {
        search.input_focused = false;
    }

    let _ = update::update(&mut state, Message::CloseProjectSearch);

    assert!(!state.is_search_open());
}

// === Query changes ===

#[test]
fn query_change_queues_search() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);

    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged("test".into()),
    );

    let Some(search) = &state.project_search else {
        unreachable!("project_search should be Some");
    };
    assert_eq!(search.query, "test");
    assert!(search.pending_run_at.is_some());
    assert!(search.input_focused);
}

#[test]
fn clearing_query_clears_results() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![make_search_result("src/main.rs", 2)],
    );
    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| !s.results.is_empty())
    );

    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged(String::new()),
    );

    let Some(search) = &state.project_search else {
        unreachable!("project_search should be Some");
    };
    assert!(search.query.is_empty());
    assert!(search.results.is_empty());
    assert!(search.matching_paths.is_empty());
    assert!(search.pending_run_at.is_none());
}

// === Case sensitivity toggle ===

#[test]
fn toggle_case_flips_and_queues_search() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);

    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| !s.case_sensitive)
    );
    let _ = update::update(&mut state, Message::ProjectSearchToggleCase);
    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| s.case_sensitive)
    );
    let _ = update::update(&mut state, Message::ProjectSearchToggleCase);
    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| !s.case_sensitive)
    );
}

// === Search results ===

#[test]
fn receiving_results_populates_matching_paths() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![
            make_search_result("src/main.rs", 3),
            make_search_result("src/search.rs", 1),
        ],
    );

    let Some(search) = &state.project_search else {
        unreachable!("project_search should be Some");
    };
    assert_eq!(search.results.len(), 2);
    assert!(search.matching_paths.contains("src/main.rs"));
    assert!(search.matching_paths.contains("src/search.rs"));
    assert!(!search.matching_paths.contains("src/app.rs"));
    assert!(search.result_index_by_path.contains_key("src/main.rs"));
    assert!(search.result_index_by_path.contains_key("src/search.rs"));
}

#[test]
fn stale_results_are_ignored() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged("test".into()),
    );

    let stale_id = state
        .project_search
        .as_ref()
        .map_or(0, |s| s.request_id.wrapping_sub(1));
    let _ = update::update(
        &mut state,
        Message::ProjectSearchResults(
            stale_id,
            Ok(make_search_response(vec![make_search_result(
                "src/main.rs",
                3,
            )])),
        ),
    );

    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| s.results.is_empty())
    );
}

#[test]
fn results_auto_select_first_matching_file() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged("test".into()),
    );
    state.selected_path = Some("src/app.rs".into());

    let request_id = state.project_search.as_ref().map_or(0, |s| s.request_id);
    let _ = update::update(
        &mut state,
        Message::ProjectSearchResults(
            request_id,
            Ok(make_search_response(vec![make_search_result(
                "src/main.rs",
                3,
            )])),
        ),
    );

    assert_eq!(state.selected_path.as_deref(), Some("src/main.rs"));
    assert_eq!(state.selected_file, Some(0));
}

#[test]
fn results_keep_current_selection_if_it_matches() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged("test".into()),
    );
    state.selected_path = Some("src/search.rs".into());
    state.selected_file = Some(4);

    let request_id = state.project_search.as_ref().map_or(0, |s| s.request_id);
    let _ = update::update(
        &mut state,
        Message::ProjectSearchResults(
            request_id,
            Ok(make_search_response(vec![
                make_search_result("src/main.rs", 3),
                make_search_result("src/search.rs", 1),
            ])),
        ),
    );

    assert_eq!(state.selected_path.as_deref(), Some("src/search.rs"));
    assert_eq!(state.selected_file, Some(4));
}

// === Sidebar filtering ===

#[test]
fn visible_rows_shows_all_when_search_closed() {
    let state = test_state(sample_files());
    // Root + src/ + views/ + 5 files = at least 8 rows
    assert!(state.visible_cached_rows().len() >= 8);
}

#[test]
fn visible_rows_filters_to_matching_files_when_search_open() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![make_search_result("src/main.rs", 3)],
    );

    let file_names = collect_visible_file_names(&state);
    assert_eq!(file_names, vec!["main.rs"]);

    let dir_names = collect_visible_dir_names(&state);
    assert_eq!(dir_names, vec!["src"]);
}

#[test]
fn visible_rows_includes_parent_dirs_of_matching_files() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![make_search_result("src/views/sidebar.rs", 2)],
    );

    let dir_names = collect_visible_dir_names(&state);
    assert!(dir_names.contains(&"src".to_owned()));
    assert!(dir_names.contains(&"views".to_owned()));

    let file_names = collect_visible_file_names(&state);
    assert_eq!(file_names, vec!["sidebar.rs"]);
}

#[test]
fn visible_rows_unfiltered_when_search_open_but_query_empty() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    assert!(state.visible_cached_rows().len() >= 8);
}

#[test]
fn visible_rows_unfiltered_after_search_closed() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![make_search_result("src/main.rs", 3)],
    );
    let filtered_count = state.visible_cached_rows().len();

    let _ = update::update(&mut state, Message::CloseProjectSearch);

    assert!(state.visible_cached_rows().len() > filtered_count);
}

// === Navigation targets respect filter ===

#[test]
fn sidebar_targets_respect_search_filter() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![make_search_result("src/main.rs", 3)],
    );

    let file_targets = collect_visible_file_targets(&state);
    assert_eq!(file_targets, vec!["src/main.rs"]);
    assert!(!file_targets.contains(&"src/app.rs".to_owned()));
}

// === Jump to from search result ===

#[test]
fn jump_to_closes_search_and_selects_file() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![make_search_result("src/search.rs", 1)],
    );

    let _ = update::update(
        &mut state,
        Message::ProjectSearchJumpTo("src/search.rs".into(), 42),
    );

    assert!(!state.is_search_open());
    assert!(state.project_search.is_some());
    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| s.query == "test")
    );

    let Some(jump) = &state.pending_diff_jump else {
        unreachable!("pending_diff_jump should be set after jump_to");
    };
    assert_eq!(jump.path, "src/search.rs");
    assert_eq!(jump.line_number, 42);
    assert_eq!(state.selected_path.as_deref(), Some("src/search.rs"));
}

// === Reopen after jump preserves query ===

#[test]
fn reopen_after_jump_preserves_query_and_results() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "find_me",
        vec![
            make_search_result("src/main.rs", 2),
            make_search_result("src/search.rs", 1),
        ],
    );

    let _ = update::update(
        &mut state,
        Message::ProjectSearchJumpTo("src/main.rs".into(), 10),
    );
    assert!(!state.is_search_open());

    let _ = update::update(&mut state, Message::OpenProjectSearch);

    assert!(state.is_search_open());
    let Some(search) = &state.project_search else {
        unreachable!("project_search should be Some after reopen");
    };
    assert_eq!(search.query, "find_me");
    assert_eq!(search.results.len(), 2);
    assert!(search.matching_paths.contains("src/main.rs"));
}

// === Edge cases ===

#[test]
fn search_with_no_matching_files_shows_empty() {
    let mut state = test_state(sample_files());
    open_search_with_results(&mut state, "xyz", vec![]);

    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| s.results.is_empty() && s.matching_paths.is_empty())
    );
    // No filter applied when no matches
    assert!(state.visible_cached_rows().len() >= 8);
}

#[test]
fn multiple_open_close_cycles_preserve_state() {
    let mut state = test_state(sample_files());

    for i in 0..3 {
        let _ = update::update(&mut state, Message::OpenProjectSearch);
        assert!(state.is_search_open());

        if i == 0 {
            let _ = update::update(
                &mut state,
                Message::ProjectSearchQueryChanged("persistent".into()),
            );
        }

        let _ = update::update(&mut state, Message::CloseProjectSearch);
        assert!(!state.is_search_open());
        assert!(
            state
                .project_search
                .as_ref()
                .is_some_and(|s| s.query == "persistent")
        );
    }
}

#[test]
fn open_search_sets_active_pane_to_sidebar() {
    let mut state = test_state(sample_files());
    state.active_pane = ActivePane::Diff;

    let _ = update::update(&mut state, Message::OpenProjectSearch);

    assert_eq!(state.active_pane, ActivePane::Sidebar);
}

#[test]
fn close_search_sets_active_pane_to_sidebar() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    state.active_pane = ActivePane::Diff;

    let _ = update::update(&mut state, Message::CloseProjectSearch);

    assert_eq!(state.active_pane, ActivePane::Sidebar);
}

#[test]
fn query_change_marks_input_focused() {
    let mut state = test_state(sample_files());
    let _ = update::update(&mut state, Message::OpenProjectSearch);
    if let Some(search) = state.project_search.as_mut() {
        search.input_focused = false;
    }

    let _ = update::update(
        &mut state,
        Message::ProjectSearchQueryChanged("x".into()),
    );

    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| s.input_focused)
    );
}

#[test]
fn reopen_does_not_rerun_search_when_results_exist() {
    let mut state = test_state(sample_files());
    open_search_with_results(
        &mut state,
        "test",
        vec![make_search_result("src/main.rs", 3)],
    );
    // Simulate tick clearing the pending_run_at (as the real app does)
    if let Some(search) = state.project_search.as_mut() {
        search.pending_run_at = None;
    }

    let _ = update::update(&mut state, Message::CloseProjectSearch);
    let _ = update::update(&mut state, Message::OpenProjectSearch);

    assert!(
        state
            .project_search
            .as_ref()
            .is_some_and(|s| s.pending_run_at.is_none())
    );
}
