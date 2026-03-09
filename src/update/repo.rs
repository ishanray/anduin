use super::update;
use crate::actions::{
    commit_staged_changes, load_changed_files, load_selected_diff, maybe_run_project_search,
    scroll_sidebar_to_selected, stage_all_files, stage_files, unstage_all_files, unstage_files,
};
use crate::app::{ActivePane, CommitComposer, Message, ProjectSearch, SidebarTarget, State, StatusTone};
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
use iced::widget::operation::{focus, move_cursor_to_end};
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

            if let Some(index) = next_selection {
                let diff_task = load_selected_diff(state, index);
                let scroll_task = scroll_sidebar_to_selected(state);
                return Task::batch([diff_task, scroll_task, refresh_task, search_task]);
            }

            state.selected_file = None;
            state.selected_path = None;
            state.current_diff = None;
            state.clear_explicit_selection();
            Task::batch([refresh_task, search_task])
        }
        Err(error) => {
            state.error = Some(error);
            state.finish_refresh()
        }
    }
}

pub(crate) fn handle_select_file(state: &mut State, index: usize) -> Task<Message> {
    state.active_pane = ActivePane::Sidebar;
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
    state.pending_diff_jump = None;
    state.sidebar_scroll_offset = 0.0;
    state.sidebar_viewport_height = 0.0;
    state.active_pane = ActivePane::Sidebar;
    state.diff_editor.lose_focus();

    state.persist_settings();

    state.refresh_in_flight = true;
    state.refresh_queued = false;
    Task::perform(async move { load_changed_files(repo_path) }, Message::FilesLoaded)
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

    // Handle `?` (Shift+/) to toggle shortcuts help, and swallow keys while it's open.
    if let keyboard::Event::KeyPressed { key, .. } = &event {
        if matches!(key.as_ref(), keyboard::Key::Character("?")) {
            return update(state, Message::ToggleShortcutsHelp);
        }
        if state.show_shortcuts_help {
            if matches!(key.as_ref(), keyboard::Key::Named(keyboard::key::Named::Escape)) {
                state.show_shortcuts_help = false;
            }
            return Task::none();
        }
    }
    if state.show_shortcuts_help {
        return Task::none();
    }

    if state.project_search.is_some() {
        return handle_project_search_keyboard_event(state, &event);
    }

    if state.commit_composer.is_some() {
        return handle_commit_keyboard_event(state, &event);
    }

    match shortcut_action_for_event(current_shortcut_platform(), &event) {
        Some(ShortcutAction::OpenProject) => update(state, Message::OpenProjectSearch),
        Some(ShortcutAction::OpenDiff) => {
            state.active_pane = ActivePane::Diff;
            state.diff_editor.request_focus();
            state
                .diff_editor
                .update(&EditorMessage::OpenSearch)
                .map(Message::DiffEditor)
        }
        Some(ShortcutAction::CloseActive) => {
            if state.active_pane == ActivePane::Diff {
                focus_sidebar(state)
            } else if state.has_explicit_selection() {
                state.clear_explicit_selection();
                Task::none()
            } else {
                Task::none()
            }
        }
        None => {
            if state.active_pane == ActivePane::Sidebar {
                handle_file_list_keyboard_event(state, &event)
            } else {
                Task::none()
            }
        }
    }
}

pub(crate) fn handle_open_project_search(state: &mut State) -> Task<Message> {
    state.active_pane = ActivePane::Sidebar;
    state.diff_editor.lose_focus();
    let mut search = state
        .project_search
        .take()
        .unwrap_or_else(ProjectSearch::new);
    search.pending_run_at = if search.query.is_empty() {
        None
    } else {
        Some(Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS))
    };
    let input_id = search.input_id.clone();
    state.project_search = Some(search);
    Task::batch([focus(input_id.clone()), move_cursor_to_end(input_id)])
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
        async move { commit_staged_changes(repo_path, summary.clone()).map(|sha| format!("Committed {sha} — {summary}")) },
        Message::CommitFinished,
    )
}

pub(crate) fn handle_git_operation_finished(
    state: &mut State,
    result: Result<String, String>,
) -> Task<Message> {
    match result {
        Ok(message) => {
            state.set_status_message(message, StatusTone::Success);
            state.queue_refresh()
        }
        Err(error) => {
            state.set_status_message(error, StatusTone::Error);
            Task::none()
        }
    }
}

pub(crate) fn handle_commit_finished(
    state: &mut State,
    result: Result<String, String>,
) -> Task<Message> {
    match result {
        Ok(message) => {
            state.commit_composer = None;
            state.set_status_message(message, StatusTone::Success);
            state.queue_refresh()
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

fn handle_project_search_keyboard_event(
    state: &mut State,
    event: &keyboard::Event,
) -> Task<Message> {
    match shortcut_action_for_event(current_shortcut_platform(), event) {
        Some(ShortcutAction::CloseActive) => update(state, Message::CloseProjectSearch),
        _ => Task::none(),
    }
}

fn handle_commit_keyboard_event(state: &mut State, event: &keyboard::Event) -> Task<Message> {
    match shortcut_action_for_event(current_shortcut_platform(), event) {
        Some(ShortcutAction::CloseActive) => update(state, Message::CloseCommitComposer),
        Some(ShortcutAction::OpenProject | ShortcutAction::OpenDiff) => Task::none(),
        None => {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return Task::none();
            };

            if is_primary_modifier_pressed(current_shortcut_platform(), *modifiers)
                && matches!(key.as_ref(), keyboard::Key::Named(keyboard::key::Named::Enter))
            {
                update(state, Message::SubmitCommit)
            } else {
                Task::none()
            }
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
        keyboard::Key::Named(keyboard::key::Named::ArrowLeft)
            if modifiers_alt_only(*modifiers) =>
        {
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
        keyboard::Key::Named(keyboard::key::Named::Space) if modifiers_without_shift(*modifiers) => {
            toggle_stage_for_targeted_files(state)
        }
        keyboard::Key::Character(c) if no_shortcut_modifiers(*modifiers) && c.eq_ignore_ascii_case("a") => {
            toggle_stage_all(state)
        }
        keyboard::Key::Character(c) if no_shortcut_modifiers(*modifiers) && c.eq_ignore_ascii_case("u") => {
            unstage_all(state)
        }
        keyboard::Key::Character(c) if no_shortcut_modifiers(*modifiers) && c.eq_ignore_ascii_case("c") => {
            update(state, Message::OpenCommitComposer)
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
    let next_pos = current_pos.saturating_add_signed(delta).min(visible_targets.len() - 1);
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

        let Some(anchor_pos) = visible_targets.iter().position(|target| *target == anchor_target)
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
        SidebarTarget::Root | SidebarTarget::Dir(_) => scroll_sidebar_to_selected(state),
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
                state.expanded_dirs.extend(state.descendant_dir_paths(&path));
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
                unstage_files(repo_path, paths).map(|()| {
                    format!(
                        "Unstaged {count} file{}",
                        if count == 1 { "" } else { "s" }
                    )
                })
            },
            Message::GitOperationFinished,
        )
    } else {
        Task::perform(
            async move {
                stage_files(repo_path, paths).map(|()| {
                    format!("Staged {count} file{}", if count == 1 { "" } else { "s" })
                })
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

fn modifiers_without_shift(modifiers: keyboard::Modifiers) -> bool {
    !modifiers.control() && !modifiers.alt() && !modifiers.logo()
}

fn modifiers_alt_only(modifiers: keyboard::Modifiers) -> bool {
    modifiers.alt() && !modifiers.shift() && !modifiers.control() && !modifiers.logo()
}

fn no_shortcut_modifiers(modifiers: keyboard::Modifiers) -> bool {
    !modifiers.shift() && !modifiers.control() && !modifiers.alt() && !modifiers.logo()
}
