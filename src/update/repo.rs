use super::update;
use crate::actions::{
    commit_staged_changes, create_branch, discard_files, fetch_current_branch, list_branches,
    load_changed_files, load_selected_diff, load_selected_diff_without_focus_change,
    maybe_run_project_search, scroll_commit_list_to_selected, scroll_sidebar_to_selected,
    stage_all_files, stage_files, switch_branch, unstage_all_files, unstage_files,
};
use crate::actions_ui::{
    ActionsSurfaceCommand, actions_panel_command_for_key, is_actions_panel_command_key,
};
use crate::app::{
    ActivePane, BranchPicker, ChangesFocus, CommitComposer, DiscardButton, DiscardConfirm,
    HistoryFocus, Message, ProjectPicker, ProjectSearch, SidebarTab, SidebarTarget, State,
    StatusTone,
};
use crate::git;
use crate::search::SEARCH_DEBOUNCE_MS;
use crate::shortcuts::{
    ShortcutAction, current_shortcut_platform, event_modifiers, is_primary_modifier_pressed,
    shortcut_action_for_event,
};
use crate::tree::SidebarRow;
use crate::watch;
use iced::Task;
use iced::keyboard;
use iced::widget::Id;
use iced::widget::operation::{focus, move_cursor_to_end, select_all};
use iced_code_editor::{Message as EditorMessage, theme as editor_theme};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) fn handle_files_loaded(
    state: &mut State,
    result: Result<Vec<git::diff::ChangedFile>, String>,
) -> Task<Message> {
    match result {
        Ok(files) => {
            state.files = files;
            state.diff_search_cache.clear();
            state.error = None;
            state.sync_tree_state();
            state.ensure_rows_cached();
            state.retain_sidebar_selection();
            state.ensure_sidebar_focus();

            if state.project_search.is_some() {
                state.queue_project_search();
            }

            let selected_path = state.selected_path.clone();
            let next_selection = selected_path
                .as_ref()
                .and_then(|path| state.files.iter().position(|file| file.path == *path))
                .or_else(|| state.selected_file.filter(|&idx| idx < state.files.len()))
                .or_else(|| (!state.files.is_empty()).then_some(0));

            let refresh_task = state.finish_refresh();
            let search_task = maybe_run_project_search(state);
            let branch_task = {
                let repo = state.repo_path.clone();
                Task::perform(
                    async move { fetch_current_branch(repo) },
                    Message::CurrentBranchFetched,
                )
            };

            if let Some(index) = next_selection {
                let diff_task = load_selected_diff(state, index);
                let scroll_task = scroll_sidebar_to_selected(state);
                return Task::batch([
                    diff_task,
                    scroll_task,
                    refresh_task,
                    search_task,
                    branch_task,
                ]);
            }

            state.selected_file = None;
            state.selected_path = None;
            state.current_diff = None;
            state.clear_explicit_selection();
            Task::batch([refresh_task, search_task, branch_task])
        }
        Err(error) => {
            state.error = Some(error);
            state.finish_refresh()
        }
    }
}

pub(crate) fn handle_select_file(state: &mut State, index: usize) -> Task<Message> {
    state.active_pane = ActivePane::Sidebar;
    state.changes_focus = ChangesFocus::FileList;
    state.diff_editor.lose_focus();
    state.clear_explicit_selection();
    let diff_task = load_selected_diff(state, index);
    let scroll_task = scroll_sidebar_to_selected(state);
    Task::batch([diff_task, scroll_task])
}

pub(crate) fn handle_open_repo() -> Task<Message> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .set_title("Open Repository")
                .pick_folder()
                .await
                .map(|handle| handle.path().to_path_buf())
        },
        Message::RepoOpened,
    )
}

pub(crate) fn handle_repo_opened(state: &mut State, path: Option<PathBuf>) -> Task<Message> {
    let Some(path) = path else {
        return Task::none();
    };

    let repo_path = git::diff::find_repo_root(&path).unwrap_or(path);
    eprintln!("[anduin] switching to repo: {}", repo_path.display());

    state.repo_path = repo_path.clone();
    state.files.clear();
    state.selected_file = None;
    state.selected_path = None;
    state.focused_sidebar_target = None;
    state.selected_sidebar_targets.clear();
    state.selection_anchor_sidebar_target = None;
    state.current_diff = None;
    state.diff_search_cache.clear();
    state.expanded_dirs.clear();
    state.tree_root_expanded = true;
    state.initialized_tree = false;
    state.tree_dirty = true;
    state.error = None;
    state.status_message = None;
    state.commit_composer = None;
    state.project_search = None;
    state.branch_picker = None;
    state.project_picker = None;
    state.pending_diff_jump = None;
    state.show_actions_panel = false;
    state.discard_confirm = None;
    state.sidebar_scroll_offset = 0.0;
    state.sidebar_viewport_height = 0.0;
    state.active_pane = ActivePane::Sidebar;
    state.diff_editor.lose_focus();

    // Reset history state
    state.sidebar_tab = SidebarTab::Changes;
    state.commits.clear();
    state.selected_commit = None;
    state.commit_files.clear();
    state.commits_loading = false;
    state.commits_exhausted = false;
    state.history_selected_file = None;
    state.history_selected_path = None;
    state.history_diff = None;
    state.history_commit_header = None;
    state.history_focus = HistoryFocus::CommitList;
    state.changes_focus = ChangesFocus::FileList;

    let repo_str = repo_path.to_string_lossy().into_owned();
    state.recent_repos.retain(|p| p != &repo_str);
    state.recent_repos.insert(0, repo_str);
    state.recent_repos.truncate(20);

    state.persist_settings();

    state.refresh_in_flight = true;
    state.refresh_queued = false;
    Task::perform(
        async move { load_changed_files(repo_path) },
        Message::FilesLoaded,
    )
}

pub(crate) fn handle_toggle_theme(state: &mut State) -> Task<Message> {
    state.theme_mode.toggle();
    state.cached_theme = state.theme_mode.app_theme();
    state
        .diff_editor
        .set_theme(editor_theme::from_iced_theme(&state.cached_theme));
    state.persist_settings();
    Task::none()
}

pub(crate) fn handle_watch_event(state: &mut State, event: watch::Event) -> Task<Message> {
    match event {
        watch::Event::Changed => state.queue_refresh(),
        watch::Event::Error(error) => {
            state.error = Some(error);
            Task::none()
        }
    }
}

pub(crate) fn handle_sidebar_scrolled(
    state: &mut State,
    offset: f32,
    height: f32,
) -> Task<Message> {
    state.sidebar_scroll_offset = offset;
    state.sidebar_viewport_height = height;
    Task::none()
}

pub(crate) fn handle_keyboard_event(state: &mut State, event: keyboard::Event) -> Task<Message> {
    let modifiers = event_modifiers(&event);
    let alt = modifiers.alt();
    if alt != state.alt_pressed {
        state.alt_pressed = alt;
    }

    if state.show_actions_panel
        && let Some(task) = handle_actions_panel_key_event(state, &event)
    {
        return task;
    }

    // Close context menu on any keypress
    if state.sidebar_context_menu.is_some() && matches!(event, keyboard::Event::KeyPressed { .. }) {
        state.sidebar_context_menu = None;
        return Task::none();
    }

    if state.is_discard_confirm_open() {
        return handle_discard_confirm_key_event(state, &event);
    }

    if state.is_project_picker_open() {
        return handle_project_picker_key_event(state, event);
    }

    if state.is_branch_picker_open() {
        return handle_branch_picker_key_event(state, event);
    }

    if state.is_search_open() {
        return handle_project_search_keyboard_event(state, &event);
    }

    if state.commit_composer.is_some() {
        return handle_commit_keyboard_event(state, &event);
    }

    match shortcut_action_for_event(current_shortcut_platform(), &event) {
        Some(ShortcutAction::OpenProject) => update(state, Message::OpenProjectSearch),
        Some(ShortcutAction::OpenDiff) => open_diff_search(state),
        Some(ShortcutAction::OpenBranchPicker) => update(state, Message::OpenBranchPicker),
        Some(ShortcutAction::OpenProjectPicker) => update(state, Message::OpenProjectPicker),
        Some(ShortcutAction::ToggleActionsPanel) => update(state, Message::ToggleActionsPanel),
        Some(ShortcutAction::PreviousTab) => {
            let target = match state.sidebar_tab {
                SidebarTab::Changes => SidebarTab::History,
                SidebarTab::History => SidebarTab::Changes,
            };
            update(state, Message::SwitchSidebarTab(target))
        }
        Some(ShortcutAction::NextTab) => {
            let target = match state.sidebar_tab {
                SidebarTab::Changes => SidebarTab::History,
                SidebarTab::History => SidebarTab::Changes,
            };
            update(state, Message::SwitchSidebarTab(target))
        }
        Some(ShortcutAction::CloseActive) => {
            if state.sidebar_tab == SidebarTab::History {
                match state.history_focus {
                    HistoryFocus::DiffView => {
                        if state.diff_editor.is_search_open() {
                            state
                                .diff_editor
                                .update(&EditorMessage::CloseSearch)
                                .map(Message::DiffEditor)
                        } else {
                            state.history_focus = HistoryFocus::FileList;
                            state.diff_editor.lose_focus();
                            Task::none()
                        }
                    }
                    HistoryFocus::FileList => {
                        state.history_focus = HistoryFocus::CommitList;
                        Task::none()
                    }
                    HistoryFocus::CommitList => Task::none(),
                }
            } else {
                // Changes tab
                match state.changes_focus {
                    ChangesFocus::DiffView => {
                        if state.diff_editor.is_search_open() {
                            state
                                .diff_editor
                                .update(&EditorMessage::CloseSearch)
                                .map(Message::DiffEditor)
                        } else {
                            state.changes_focus = ChangesFocus::FileList;
                            focus_sidebar(state)
                        }
                    }
                    ChangesFocus::FileList => {
                        if state.active_pane == ActivePane::Diff
                            && state.diff_editor.is_search_open()
                        {
                            state
                                .diff_editor
                                .update(&EditorMessage::CloseSearch)
                                .map(Message::DiffEditor)
                        } else if state.has_explicit_selection() {
                            state.clear_explicit_selection();
                            Task::none()
                        } else {
                            Task::none()
                        }
                    }
                }
            }
        }
        None => {
            if state.sidebar_tab == SidebarTab::History {
                handle_history_keyboard_event(state, &event)
            } else if state.active_pane == ActivePane::Sidebar {
                handle_file_list_keyboard_event(state, &event)
            } else {
                Task::none()
            }
        }
    }
}

pub(crate) fn handle_open_project_search(state: &mut State) -> Task<Message> {
    state.active_pane = ActivePane::Sidebar;

    // Carry over the diff editor's search query if project search is empty
    let editor_query = state.diff_editor.search_query().to_owned();
    state.diff_editor.lose_focus();

    let mut search = state
        .project_search
        .take()
        .unwrap_or_else(ProjectSearch::new);
    search.is_open = true;
    search.input_focused = true;

    // Only seed from editor and trigger a new search if the search had no query
    let seeded = if search.query.is_empty() && !editor_query.is_empty() {
        search.query = editor_query;
        search.update_query_lower();
        true
    } else {
        false
    };

    // Only re-run search if we just seeded a new query; reopening with
    // existing results should show them instantly without re-searching.
    if seeded {
        search.rebuild_cached_summaries();
        search.pending_run_at = Some(Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
    }

    let input_id = search.input_id.clone();
    state.project_search = Some(search);

    // Select all text so the user can immediately type to replace
    Task::batch([focus(input_id.clone()), select_all(input_id)])
}

pub(crate) fn handle_open_commit_composer(state: &mut State) -> Task<Message> {
    state.active_pane = ActivePane::Sidebar;
    state.diff_editor.lose_focus();
    let mut composer = state
        .commit_composer
        .take()
        .unwrap_or_else(CommitComposer::new);
    composer.error = None;
    let input_id = composer.input_id.clone();
    state.commit_composer = Some(composer);
    Task::batch([focus(input_id.clone()), move_cursor_to_end(input_id)])
}

pub(crate) fn handle_close_commit_composer(state: &mut State) -> Task<Message> {
    state.commit_composer = None;
    focus_sidebar(state)
}

pub(crate) fn handle_commit_summary_changed(state: &mut State, summary: String) -> Task<Message> {
    if let Some(composer) = state.commit_composer.as_mut() {
        composer.summary = summary;
        composer.error = None;
    }
    Task::none()
}

pub(crate) fn handle_submit_commit(state: &mut State) -> Task<Message> {
    let staged_count = state.staged_file_count();
    let Some(composer) = state.commit_composer.as_mut() else {
        return Task::none();
    };

    if composer.summary.trim().is_empty() {
        composer.error = Some("Enter a commit summary".to_owned());
        return Task::none();
    }

    if staged_count == 0 {
        composer.error = Some("No staged changes to commit".to_owned());
        return Task::none();
    }

    if composer.submitting {
        return Task::none();
    }

    composer.submitting = true;
    composer.error = None;

    let repo_path = state.repo_path.clone();
    let summary = composer.summary.trim().to_owned();
    Task::perform(
        async move {
            commit_staged_changes(repo_path, summary.clone())
                .map(|sha| format!("Committed {sha} — {summary}"))
        },
        Message::CommitFinished,
    )
}

pub(crate) fn handle_git_operation_finished(
    state: &mut State,
    result: Result<String, String>,
) -> Task<Message> {
    match result {
        Ok(message) => {
            let clear = state.set_status_message(message, StatusTone::Success);
            Task::batch([state.queue_refresh(), clear])
        }
        Err(error) => state.set_status_message(error, StatusTone::Error),
    }
}

pub(crate) fn handle_commit_finished(
    state: &mut State,
    result: Result<String, String>,
) -> Task<Message> {
    match result {
        Ok(message) => {
            state.commit_composer = None;
            let clear = state.set_status_message(message, StatusTone::Success);
            Task::batch([state.queue_refresh(), clear])
        }
        Err(error) => {
            if let Some(composer) = state.commit_composer.as_mut() {
                composer.submitting = false;
                composer.error = Some(error);
            }
            Task::none()
        }
    }
}

pub(crate) fn handle_open_branch_picker(state: &mut State) -> Task<Message> {
    if state.branch_picker.is_some() {
        return handle_close_branch_picker(state);
    }

    let repo_path = state.repo_path.clone();
    Task::perform(
        async move { list_branches(repo_path) },
        Message::BranchesFetched,
    )
}

pub(crate) fn handle_close_branch_picker(state: &mut State) -> Task<Message> {
    state.branch_picker = None;
    Task::none()
}

pub(crate) fn handle_branches_fetched(
    state: &mut State,
    result: Result<(Vec<String>, String), String>,
) -> Task<Message> {
    match result {
        Ok((branches, current)) => {
            state.current_branch = Some(current.clone());
            let picker = BranchPicker::new(branches, current);
            let input_id = picker.input_id.clone();
            state.branch_picker = Some(picker);
            focus(input_id)
        }
        Err(error) => state.set_status_message(error, StatusTone::Error),
    }
}

pub(crate) fn handle_branch_picker_filter_changed(
    state: &mut State,
    filter: String,
) -> Task<Message> {
    if let Some(picker) = state.branch_picker.as_mut() {
        picker.filter = filter;
        picker.selected_index = 0;
        picker.error = None;
    }
    Task::none()
}

pub(crate) fn handle_branch_picker_key_event(
    state: &mut State,
    event: keyboard::Event,
) -> Task<Message> {
    let keyboard::Event::KeyPressed { key, .. } = &event else {
        return Task::none();
    };

    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
            if let Some(picker) = state.branch_picker.as_mut() {
                let count = picker.total_items();
                if count > 0 {
                    picker.selected_index = (picker.selected_index + 1).min(count - 1);
                }
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
            if let Some(picker) = state.branch_picker.as_mut() {
                picker.selected_index = picker.selected_index.saturating_sub(1);
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::Enter) => {
            if let Some(picker) = state.branch_picker.as_ref() {
                let show_create = picker.should_show_create();
                let idx = picker.selected_index;

                if show_create && idx == 0 {
                    // Create item is selected (first item)
                    let name = picker.filter.clone();
                    update(state, Message::CreateBranch(name))
                } else {
                    let offset = if show_create { 1 } else { 0 };
                    let filtered = picker.filtered_branches();
                    if let Some(branch) = filtered.get(idx - offset).map(|s| s.to_string()) {
                        update(state, Message::SwitchBranch(branch))
                    } else {
                        Task::none()
                    }
                }
            } else {
                Task::none()
            }
        }
        keyboard::Key::Named(keyboard::key::Named::Escape) => handle_close_branch_picker(state),
        _ => Task::none(),
    }
}

pub(crate) fn handle_switch_branch(state: &mut State, branch: String) -> Task<Message> {
    if state
        .branch_picker
        .as_ref()
        .is_some_and(|p| p.current == branch)
    {
        return handle_close_branch_picker(state);
    }

    let repo_path = state.repo_path.clone();
    let branch_clone = branch.clone();
    Task::perform(
        async move { switch_branch(repo_path, branch_clone) },
        Message::BranchSwitched,
    )
}

pub(crate) fn handle_branch_switched(
    state: &mut State,
    result: Result<(), String>,
) -> Task<Message> {
    match result {
        Ok(()) => {
            let branch_name = state
                .branch_picker
                .as_ref()
                .and_then(|p| {
                    let filtered = p.filtered_branches();
                    filtered.get(p.selected_index).map(|s| s.to_string())
                })
                .unwrap_or_default();

            state.branch_picker = None;
            state.current_branch = Some(branch_name.clone());
            let clear = state.set_status_message(format!("Switched to {branch_name}"), StatusTone::Success);

            state.files.clear();
            state.selected_file = None;
            state.selected_path = None;
            state.current_diff = None;
            state.diff_search_cache.clear();
            state.initialized_tree = false;
            state.tree_dirty = true;

            // Reset history (commits may differ on new branch)
            state.commits.clear();
            state.selected_commit = None;
            state.commit_files.clear();
            state.commits_loading = false;
            state.commits_exhausted = false;
            state.history_selected_file = None;
            state.history_selected_path = None;
            state.history_diff = None;
            state.history_commit_header = None;
            state.history_focus = HistoryFocus::CommitList;

            Task::batch([state.queue_refresh(), clear])
        }
        Err(error) => {
            if let Some(picker) = state.branch_picker.as_mut() {
                picker.error = Some(error);
            }
            Task::none()
        }
    }
}

pub(crate) fn handle_create_branch(state: &mut State, branch: String) -> Task<Message> {
    let repo_path = state.repo_path.clone();
    let branch_clone = branch.clone();
    Task::perform(
        async move { create_branch(repo_path, branch_clone) },
        Message::BranchCreated,
    )
}

pub(crate) fn handle_branch_created(
    state: &mut State,
    result: Result<(), String>,
) -> Task<Message> {
    match result {
        Ok(()) => {
            let branch_name = state
                .branch_picker
                .as_ref()
                .map(|p| p.filter.clone())
                .unwrap_or_default();

            state.branch_picker = None;
            state.current_branch = Some(branch_name.clone());
            let clear = state.set_status_message(
                format!("Created and switched to {branch_name}"),
                StatusTone::Success,
            );

            state.files.clear();
            state.selected_file = None;
            state.selected_path = None;
            state.current_diff = None;
            state.diff_search_cache.clear();
            state.initialized_tree = false;
            state.tree_dirty = true;

            state.commits.clear();
            state.selected_commit = None;
            state.commit_files.clear();
            state.commits_loading = false;
            state.commits_exhausted = false;
            state.history_selected_file = None;
            state.history_selected_path = None;
            state.history_diff = None;
            state.history_commit_header = None;
            state.history_focus = HistoryFocus::CommitList;

            Task::batch([state.queue_refresh(), clear])
        }
        Err(error) => {
            if let Some(picker) = state.branch_picker.as_mut() {
                picker.error = Some(error);
            }
            Task::none()
        }
    }
}

pub(crate) fn handle_current_branch_fetched(
    state: &mut State,
    result: Result<String, String>,
) -> Task<Message> {
    if let Ok(branch) = result {
        state.current_branch = Some(branch);
    }
    Task::none()
}

pub(crate) fn handle_open_project_picker(state: &mut State) -> Task<Message> {
    if state.project_picker.is_some() {
        return handle_close_project_picker(state);
    }

    let repos = state.recent_repos.clone();
    let current = state.repo_path.to_string_lossy().into_owned();
    let picker = ProjectPicker::new(repos, current);
    let input_id = picker.input_id.clone();
    state.project_picker = Some(picker);
    focus(input_id)
}

pub(crate) fn handle_close_project_picker(state: &mut State) -> Task<Message> {
    state.project_picker = None;
    Task::none()
}

pub(crate) fn handle_project_picker_filter_changed(
    state: &mut State,
    filter: String,
) -> Task<Message> {
    if let Some(picker) = state.project_picker.as_mut() {
        picker.filter = filter;
        picker.selected_index = 0;
    }
    Task::none()
}

pub(crate) fn handle_project_picker_key_event(
    state: &mut State,
    event: keyboard::Event,
) -> Task<Message> {
    let keyboard::Event::KeyPressed { key, .. } = &event else {
        return Task::none();
    };

    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
            if let Some(picker) = state.project_picker.as_mut() {
                let count = picker.filtered_repos().len();
                if count > 0 {
                    picker.selected_index = (picker.selected_index + 1).min(count - 1);
                }
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
            if let Some(picker) = state.project_picker.as_mut() {
                picker.selected_index = picker.selected_index.saturating_sub(1);
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::Enter) => {
            let repo = state.project_picker.as_ref().and_then(|picker| {
                let filtered = picker.filtered_repos();
                filtered.get(picker.selected_index).map(|s| s.to_string())
            });
            if let Some(repo) = repo {
                state.project_picker = None;
                update(state, Message::RepoOpened(Some(PathBuf::from(repo))))
            } else {
                Task::none()
            }
        }
        keyboard::Key::Named(keyboard::key::Named::Escape) => handle_close_project_picker(state),
        _ => Task::none(),
    }
}

pub(crate) fn handle_switch_project(state: &mut State, repo: String) -> Task<Message> {
    state.project_picker = None;
    update(state, Message::RepoOpened(Some(PathBuf::from(repo))))
}

fn handle_project_search_keyboard_event(
    state: &mut State,
    event: &keyboard::Event,
) -> Task<Message> {
    match shortcut_action_for_event(current_shortcut_platform(), event) {
        Some(ShortcutAction::CloseActive) => {
            let input_focused = state
                .project_search
                .as_ref()
                .is_some_and(|s| s.input_focused);
            if input_focused {
                // First Escape: unfocus input, enable sidebar keyboard nav
                if let Some(search) = state.project_search.as_mut() {
                    search.input_focused = false;
                }
                state.active_pane = ActivePane::Sidebar;
                // Unfocus the text input by focusing a dummy widget
                focus(Id::new("__unfocus_dummy__"))
            } else {
                // Second Escape: close search entirely
                update(state, Message::CloseProjectSearch)
            }
        }
        Some(ShortcutAction::OpenProject) => {
            // Cmd+Shift+F while search is open: refocus the input
            if let Some(search) = state.project_search.as_mut() {
                search.input_focused = true;
                let input_id = search.input_id.clone();
                Task::batch([focus(input_id.clone()), select_all(input_id)])
            } else {
                Task::none()
            }
        }
        None => {
            // When input is not focused, allow sidebar keyboard navigation
            let input_focused = state
                .project_search
                .as_ref()
                .is_some_and(|s| s.input_focused);
            if !input_focused && state.active_pane == ActivePane::Sidebar {
                handle_file_list_keyboard_event(state, event)
            } else {
                Task::none()
            }
        }
        _ => Task::none(),
    }
}

fn handle_commit_keyboard_event(state: &mut State, event: &keyboard::Event) -> Task<Message> {
    match shortcut_action_for_event(current_shortcut_platform(), event) {
        Some(ShortcutAction::CloseActive) => update(state, Message::CloseCommitComposer),
        Some(
            ShortcutAction::OpenProject
            | ShortcutAction::OpenDiff
            | ShortcutAction::OpenBranchPicker
            | ShortcutAction::OpenProjectPicker
            | ShortcutAction::ToggleActionsPanel
            | ShortcutAction::PreviousTab
            | ShortcutAction::NextTab,
        ) => Task::none(),
        None => {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return Task::none();
            };

            if is_primary_modifier_pressed(current_shortcut_platform(), *modifiers)
                && matches!(
                    key.as_ref(),
                    keyboard::Key::Named(keyboard::key::Named::Enter)
                )
            {
                update(state, Message::SubmitCommit)
            } else {
                Task::none()
            }
        }
    }
}

fn handle_history_keyboard_event(state: &mut State, event: &keyboard::Event) -> Task<Message> {
    let keyboard::Event::KeyPressed { key, .. } = event else {
        return Task::none();
    };

    match state.history_focus {
        HistoryFocus::CommitList => match key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                let count = state.commits.len();
                if count == 0 {
                    return Task::none();
                }
                let current = state.selected_commit.unwrap_or(0);
                let next = (current + 1).min(count - 1);
                if next != current {
                    let select_task = update(state, Message::SelectCommit(next));
                    let scroll_task = scroll_commit_list_to_selected(state);
                    Task::batch([select_task, scroll_task])
                } else {
                    Task::none()
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                let current = state.selected_commit.unwrap_or(0);
                let next = current.saturating_sub(1);
                if next != current {
                    let select_task = update(state, Message::SelectCommit(next));
                    let scroll_task = scroll_commit_list_to_selected(state);
                    Task::batch([select_task, scroll_task])
                } else {
                    Task::none()
                }
            }
            keyboard::Key::Named(keyboard::key::Named::Enter) => {
                if state.selected_commit.is_some() && !state.commit_files.is_empty() {
                    state.history_focus = HistoryFocus::FileList;
                }
                Task::none()
            }
            _ => Task::none(),
        },
        HistoryFocus::FileList => match key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                let count = state.commit_files.len();
                if count == 0 {
                    return Task::none();
                }
                let current = state.history_selected_file.unwrap_or(0);
                let next = (current + 1).min(count - 1);
                if next != current {
                    update(state, Message::SelectHistoryFile(next))
                } else {
                    Task::none()
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                let current = state.history_selected_file.unwrap_or(0);
                let next = current.saturating_sub(1);
                if next != current {
                    update(state, Message::SelectHistoryFile(next))
                } else {
                    Task::none()
                }
            }
            keyboard::Key::Named(keyboard::key::Named::Enter) => {
                if state.history_diff.is_some() {
                    state.history_focus = HistoryFocus::DiffView;
                    state.diff_editor.gain_focus();
                }
                Task::none()
            }
            _ => Task::none(),
        },
        HistoryFocus::DiffView => {
            // Diff editor handles its own keys (scrolling etc.)
            Task::none()
        }
    }
}

fn handle_file_list_keyboard_event(state: &mut State, event: &keyboard::Event) -> Task<Message> {
    let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
        return Task::none();
    };

    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::ArrowUp)
            if modifiers_without_shift(*modifiers) =>
        {
            navigate_visible_rows(state, -1, modifiers.shift())
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowDown)
            if modifiers_without_shift(*modifiers) =>
        {
            navigate_visible_rows(state, 1, modifiers.shift())
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowLeft) if modifiers_alt_only(*modifiers) => {
            collapse_focused_row(state, true)
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowRight)
            if modifiers_alt_only(*modifiers) =>
        {
            expand_focused_row(state, true)
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowLeft)
            if modifiers_without_shift(*modifiers) =>
        {
            collapse_focused_row(state, false)
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowRight)
            if modifiers_without_shift(*modifiers) =>
        {
            expand_focused_row(state, false)
        }
        keyboard::Key::Named(keyboard::key::Named::Space)
            if modifiers_without_shift(*modifiers) =>
        {
            toggle_stage_for_targeted_files(state)
        }
        keyboard::Key::Character(c)
            if no_shortcut_modifiers(*modifiers) && c.eq_ignore_ascii_case("a") =>
        {
            toggle_stage_all(state)
        }
        keyboard::Key::Character(c)
            if no_shortcut_modifiers(*modifiers) && c.eq_ignore_ascii_case("u") =>
        {
            unstage_all(state)
        }
        keyboard::Key::Character(c)
            if no_shortcut_modifiers(*modifiers) && c.eq_ignore_ascii_case("c") =>
        {
            update(state, Message::OpenCommitComposer)
        }
        keyboard::Key::Named(keyboard::key::Named::Enter) if no_shortcut_modifiers(*modifiers) => {
            if state.current_diff.is_some() {
                state.changes_focus = ChangesFocus::DiffView;
                state.active_pane = ActivePane::Diff;
                state.diff_editor.gain_focus();
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::Backspace | keyboard::key::Named::Delete)
            if no_shortcut_modifiers(*modifiers) =>
        {
            update(state, Message::RequestDiscard)
        }
        keyboard::Key::Named(keyboard::key::Named::Escape) => {
            if state.has_explicit_selection() {
                state.clear_explicit_selection();
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

fn navigate_visible_rows(state: &mut State, delta: isize, extend: bool) -> Task<Message> {
    state.ensure_rows_cached();
    state.retain_sidebar_selection();
    state.ensure_sidebar_focus();

    let visible_targets = state.visible_sidebar_targets();
    if visible_targets.is_empty() {
        return Task::none();
    }

    let current_pos = state.focused_sidebar_row_index().unwrap_or(0);
    let next_pos = current_pos
        .saturating_add_signed(delta)
        .min(visible_targets.len() - 1);
    if next_pos == current_pos {
        return Task::none();
    }

    let next_target = visible_targets[next_pos].clone();

    if extend {
        let anchor_target = state
            .selection_anchor_sidebar_target
            .clone()
            .or_else(|| state.focused_sidebar_target.clone())
            .unwrap_or_else(|| visible_targets[current_pos].clone());

        let Some(anchor_pos) = visible_targets
            .iter()
            .position(|target| *target == anchor_target)
        else {
            return Task::none();
        };

        let range = if anchor_pos <= next_pos {
            &visible_targets[anchor_pos..=next_pos]
        } else {
            &visible_targets[next_pos..=anchor_pos]
        };

        state.selection_anchor_sidebar_target = Some(anchor_target);
        state.selected_sidebar_targets = range.iter().cloned().collect();
    } else {
        state.clear_explicit_selection();
    }

    focus_sidebar_target(state, next_target)
}

fn focus_sidebar_target(state: &mut State, target: SidebarTarget) -> Task<Message> {
    state.active_pane = ActivePane::Sidebar;
    state.diff_editor.lose_focus();
    state.focused_sidebar_target = Some(target.clone());

    match target {
        SidebarTarget::File(path) => {
            let Some(index) = state.files.iter().position(|file| file.path == path) else {
                return scroll_sidebar_to_selected(state);
            };
            let diff_task = load_selected_diff(state, index);
            let scroll_task = scroll_sidebar_to_selected(state);
            Task::batch([diff_task, scroll_task])
        }
        SidebarTarget::Root => {
            let diff_task = if let Some(file_idx) = state.first_file_index_for_root() {
                load_selected_diff_without_focus_change(state, file_idx)
            } else {
                Task::none()
            };
            let scroll_task = scroll_sidebar_to_selected(state);
            Task::batch([diff_task, scroll_task])
        }
        SidebarTarget::Dir(ref path) => {
            let diff_task = if let Some(file_idx) = state.first_file_index_for_dir(path) {
                load_selected_diff_without_focus_change(state, file_idx)
            } else {
                Task::none()
            };
            let scroll_task = scroll_sidebar_to_selected(state);
            Task::batch([diff_task, scroll_task])
        }
    }
}

fn expand_focused_row(state: &mut State, recursive: bool) -> Task<Message> {
    let Some(target) = state.focused_sidebar_target.clone() else {
        return Task::none();
    };

    match target {
        SidebarTarget::Root => {
            if recursive {
                state.tree_root_expanded = true;
                state.expanded_dirs.extend(state.all_dir_paths());
                state.tree_dirty = true;
                state.ensure_rows_cached();
                state.retain_sidebar_selection();
                return scroll_sidebar_to_selected(state);
            }

            if !state.tree_root_expanded {
                state.toggle_root(false);
                state.ensure_rows_cached();
                state.retain_sidebar_selection();
                return scroll_sidebar_to_selected(state);
            }

            focus_first_visible_child(state)
        }
        SidebarTarget::Dir(path) => {
            if recursive {
                state.expanded_dirs.insert(path.clone());
                state
                    .expanded_dirs
                    .extend(state.descendant_dir_paths(&path));
                state.tree_dirty = true;
                state.ensure_rows_cached();
                state.retain_sidebar_selection();
                return scroll_sidebar_to_selected(state);
            }

            if !state.expanded_dirs.contains(&path) {
                state.toggle_dir(&path, false);
                state.ensure_rows_cached();
                state.retain_sidebar_selection();
                return scroll_sidebar_to_selected(state);
            }

            focus_first_visible_child(state)
        }
        SidebarTarget::File(_) => Task::none(),
    }
}

fn collapse_focused_row(state: &mut State, recursive: bool) -> Task<Message> {
    let Some(target) = state.focused_sidebar_target.clone() else {
        return Task::none();
    };

    match target {
        SidebarTarget::Root => {
            if state.tree_root_expanded {
                state.toggle_root(recursive);
                state.ensure_rows_cached();
                state.retain_sidebar_selection();
            }
            scroll_sidebar_to_selected(state)
        }
        SidebarTarget::Dir(path) => {
            if state.expanded_dirs.contains(&path) {
                state.toggle_dir(&path, recursive);
                state.ensure_rows_cached();
                state.retain_sidebar_selection();
                scroll_sidebar_to_selected(state)
            } else if let Some(parent) = parent_dir_target(&path) {
                focus_sidebar_target(state, parent)
            } else {
                focus_sidebar_target(state, SidebarTarget::Root)
            }
        }
        SidebarTarget::File(path) => {
            if let Some(parent) = file_parent_target(&path) {
                focus_sidebar_target(state, parent)
            } else {
                focus_sidebar_target(state, SidebarTarget::Root)
            }
        }
    }
}

fn toggle_stage_for_targeted_files(state: &mut State) -> Task<Message> {
    let paths = state.targeted_file_paths();
    if paths.is_empty() {
        return Task::none();
    }

    let all_staged = state.are_all_paths_staged(&paths);
    let count = paths.len();
    let repo_path = state.repo_path.clone();

    if all_staged {
        Task::perform(
            async move {
                unstage_files(repo_path, paths)
                    .map(|()| format!("Unstaged {count} file{}", if count == 1 { "" } else { "s" }))
            },
            Message::GitOperationFinished,
        )
    } else {
        Task::perform(
            async move {
                stage_files(repo_path, paths)
                    .map(|()| format!("Staged {count} file{}", if count == 1 { "" } else { "s" }))
            },
            Message::GitOperationFinished,
        )
    }
}

fn toggle_stage_all(state: &mut State) -> Task<Message> {
    let count = state.files.len();
    if count == 0 {
        return Task::none();
    }

    if state.files.iter().all(|file| file.is_staged()) {
        return unstage_all(state);
    }

    let repo_path = state.repo_path.clone();
    Task::perform(
        async move {
            stage_all_files(repo_path)
                .map(|()| format!("Staged {count} file{}", if count == 1 { "" } else { "s" }))
        },
        Message::GitOperationFinished,
    )
}

fn unstage_all(state: &mut State) -> Task<Message> {
    let count = state.staged_file_count();
    if count == 0 {
        return Task::none();
    }

    let repo_path = state.repo_path.clone();
    Task::perform(
        async move {
            unstage_all_files(repo_path)
                .map(|()| format!("Unstaged {count} file{}", if count == 1 { "" } else { "s" }))
        },
        Message::GitOperationFinished,
    )
}

fn focus_sidebar(state: &mut State) -> Task<Message> {
    state.active_pane = ActivePane::Sidebar;
    state
        .diff_editor
        .update(&EditorMessage::CanvasFocusLost)
        .map(Message::DiffEditor)
}

fn focus_first_visible_child(state: &mut State) -> Task<Message> {
    let Some(current_index) = state.focused_sidebar_row_index() else {
        return Task::none();
    };
    let Some(current_row) = state.cached_rows.get(current_index) else {
        return Task::none();
    };
    let current_depth = sidebar_row_depth(current_row);
    let Some(next_row) = state.cached_rows.get(current_index + 1) else {
        return Task::none();
    };
    if sidebar_row_depth(next_row) <= current_depth {
        return Task::none();
    }

    let next_target = state.sidebar_target_for_row(next_row);
    focus_sidebar_target(state, next_target)
}

fn sidebar_row_depth(row: &SidebarRow) -> usize {
    match row {
        SidebarRow::Root { .. } => 0,
        SidebarRow::Dir { depth, .. } | SidebarRow::File { depth, .. } => *depth,
    }
}

fn parent_dir_target(path: &str) -> Option<SidebarTarget> {
    path.rsplit_once('/')
        .map(|(parent, _)| SidebarTarget::Dir(parent.to_owned()))
}

fn file_parent_target(path: &str) -> Option<SidebarTarget> {
    path.rsplit_once('/')
        .map(|(parent, _)| SidebarTarget::Dir(parent.to_owned()))
}

fn handle_actions_panel_key_event(
    state: &mut State,
    event: &keyboard::Event,
) -> Option<Task<Message>> {
    let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
        return None;
    };

    if matches!(
        key.as_ref(),
        keyboard::Key::Named(keyboard::key::Named::Escape)
    ) {
        state.show_actions_panel = false;
        return Some(Task::none());
    }

    if !no_shortcut_modifiers(*modifiers) {
        return None;
    }

    let keyboard::Key::Character(key) = key.as_ref() else {
        return None;
    };

    let Some(command) = actions_panel_command_for_key(state, key) else {
        return is_actions_panel_command_key(key).then_some(Task::none());
    };

    state.show_actions_panel = false;

    let task = match command {
        ActionsSurfaceCommand::OpenRepo => update(state, Message::OpenRepo),
        ActionsSurfaceCommand::OpenBranchPicker => update(state, Message::OpenBranchPicker),
        ActionsSurfaceCommand::OpenProjectPicker => update(state, Message::OpenProjectPicker),
        ActionsSurfaceCommand::OpenProjectSearch => update(state, Message::OpenProjectSearch),
        ActionsSurfaceCommand::OpenCommitComposer => update(state, Message::OpenCommitComposer),
        ActionsSurfaceCommand::SearchDiff => open_diff_search(state),
        ActionsSurfaceCommand::SwitchToHistoryTab => {
            update(state, Message::SwitchSidebarTab(SidebarTab::History))
        }
        ActionsSurfaceCommand::SwitchToChangesTab => {
            update(state, Message::SwitchSidebarTab(SidebarTab::Changes))
        }
        ActionsSurfaceCommand::CopyCommitHash => {
            let Some(commit) = state.history_commit_header.as_ref() else {
                return Some(Task::none());
            };
            update(state, Message::CopyCommitHash(commit.hash.clone()))
        }
    };

    Some(task)
}

fn open_diff_search(state: &mut State) -> Task<Message> {
    let has_diff = if state.sidebar_tab == SidebarTab::History {
        state.history_diff.is_some()
    } else {
        state.current_diff.is_some()
    };

    if !has_diff {
        return Task::none();
    }

    state.active_pane = ActivePane::Diff;
    if state.sidebar_tab == SidebarTab::History {
        state.history_focus = HistoryFocus::DiffView;
    } else {
        state.changes_focus = ChangesFocus::DiffView;
    }
    state.diff_editor.gain_focus();
    state
        .diff_editor
        .update(&EditorMessage::OpenSearch)
        .map(Message::DiffEditor)
}

fn modifiers_without_shift(modifiers: keyboard::Modifiers) -> bool {
    !modifiers.control() && !modifiers.alt() && !modifiers.logo()
}

fn modifiers_alt_only(modifiers: keyboard::Modifiers) -> bool {
    modifiers.alt() && !modifiers.shift() && !modifiers.control() && !modifiers.logo()
}

fn no_shortcut_modifiers(modifiers: keyboard::Modifiers) -> bool {
    !modifiers.shift() && !modifiers.control() && !modifiers.alt() && !modifiers.logo()
}

// --- Discard ---

pub(crate) fn handle_request_discard(state: &mut State) -> Task<Message> {
    let paths = state.targeted_file_paths();
    if paths.is_empty() {
        return Task::none();
    }

    state.discard_confirm = Some(DiscardConfirm {
        paths,
        focused_button: DiscardButton::Cancel,
    });
    Task::none()
}

pub(crate) fn handle_confirm_discard(state: &mut State) -> Task<Message> {
    let Some(confirm) = state.discard_confirm.take() else {
        return Task::none();
    };

    let paths = confirm.paths;
    let repo_path = state.repo_path.clone();

    Task::perform(
        async move { discard_files(repo_path, paths) },
        Message::GitOperationFinished,
    )
}

pub(crate) fn handle_cancel_discard(state: &mut State) -> Task<Message> {
    state.discard_confirm = None;
    Task::none()
}

fn handle_discard_confirm_key_event(state: &mut State, event: &keyboard::Event) -> Task<Message> {
    let keyboard::Event::KeyPressed { key, .. } = event else {
        return Task::none();
    };

    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::Enter) => {
            let focused = state
                .discard_confirm
                .as_ref()
                .map(|c| c.focused_button)
                .unwrap_or(DiscardButton::Cancel);
            match focused {
                DiscardButton::Discard => update(state, Message::ConfirmDiscard),
                DiscardButton::Cancel => update(state, Message::CancelDiscard),
            }
        }
        keyboard::Key::Named(keyboard::key::Named::Tab) => {
            if let Some(confirm) = state.discard_confirm.as_mut() {
                confirm.focused_button = match confirm.focused_button {
                    DiscardButton::Cancel => DiscardButton::Discard,
                    DiscardButton::Discard => DiscardButton::Cancel,
                };
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
            if let Some(confirm) = state.discard_confirm.as_mut() {
                confirm.focused_button = DiscardButton::Cancel;
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
            if let Some(confirm) = state.discard_confirm.as_mut() {
                confirm.focused_button = DiscardButton::Discard;
            }
            Task::none()
        }
        keyboard::Key::Named(keyboard::key::Named::Escape) => update(state, Message::CancelDiscard),
        _ => Task::none(),
    }
}
