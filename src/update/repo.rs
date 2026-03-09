use super::update;
use crate::actions::{
    commit_staged_changes, load_changed_files, load_selected_diff, maybe_run_project_search,
    scroll_sidebar_to_selected, stage_all_files, stage_files, unstage_all_files, unstage_files,
};
use crate::app::{CommitComposer, Message, ProjectSearch, State, StatusTone};
use crate::git;
use crate::search::SEARCH_DEBOUNCE_MS;
use crate::shortcuts::{
    ShortcutAction, current_shortcut_platform, event_modifiers, is_primary_modifier_pressed,
    shortcut_action_for_event,
};
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
            state.retain_file_selection();
            state.sync_tree_state();
            state.ensure_rows_cached();

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
    state.selected_paths.clear();
    state.selection_anchor_path = None;
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

pub(crate) fn handle_keyboard_event(state: &mut State, event: keyboard::Event) -> Task<Message> {
    let modifiers = event_modifiers(&event);
    let alt = modifiers.alt();
    if alt != state.alt_pressed {
        state.alt_pressed = alt;
    }

    if state.project_search.is_some() {
        return handle_project_search_keyboard_event(state, &event);
    }

    if state.commit_composer.is_some() {
        return handle_commit_keyboard_event(state, &event);
    }

    match shortcut_action_for_event(current_shortcut_platform(), &event) {
        Some(ShortcutAction::OpenProject) => update(state, Message::OpenProjectSearch),
        Some(ShortcutAction::OpenDiff) => state
            .diff_editor
            .update(&EditorMessage::OpenSearch)
            .map(Message::DiffEditor),
        Some(ShortcutAction::CloseActive) => {
            if state.has_explicit_selection() {
                state.clear_explicit_selection();
                Task::none()
            } else {
                state
                    .diff_editor
                    .update(&EditorMessage::CloseSearch)
                    .map(Message::DiffEditor)
            }
        }
        None => handle_file_list_keyboard_event(state, &event),
    }
}

pub(crate) fn handle_open_project_search(state: &mut State) -> Task<Message> {
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
    Task::none()
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
            navigate_visible_files(state, -1, modifiers.shift())
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowDown)
            if modifiers_without_shift(*modifiers) =>
        {
            navigate_visible_files(state, 1, modifiers.shift())
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

fn navigate_visible_files(state: &mut State, delta: isize, extend: bool) -> Task<Message> {
    state.ensure_rows_cached();
    let visible_files = state.visible_file_indices();
    if visible_files.is_empty() {
        return Task::none();
    }

    let current_pos = state
        .selected_file
        .and_then(|index| visible_files.iter().position(|visible| *visible == index))
        .unwrap_or(0);

    let next_pos = current_pos.saturating_add_signed(delta).min(visible_files.len() - 1);
    if next_pos == current_pos && state.selected_file.is_some() {
        return Task::none();
    }

    let next_index = visible_files[next_pos];

    if extend {
        let Some(anchor_path) = state
            .selection_anchor_path
            .clone()
            .or_else(|| state.selected_path.clone())
            .or_else(|| state.files.get(next_index).map(|file| file.path.clone()))
        else {
            return Task::none();
        };

        let Some(anchor_pos) = visible_files.iter().position(|index| {
            state
                .files
                .get(*index)
                .is_some_and(|file| file.path == anchor_path)
        }) else {
            return Task::none();
        };

        let range = if anchor_pos <= next_pos {
            &visible_files[anchor_pos..=next_pos]
        } else {
            &visible_files[next_pos..=anchor_pos]
        };

        state.selection_anchor_path = Some(anchor_path);
        state.selected_paths = range
            .iter()
            .filter_map(|index| state.files.get(*index).map(|file| file.path.clone()))
            .collect();
    } else {
        state.clear_explicit_selection();
    }

    let diff_task = load_selected_diff(state, next_index);
    let scroll_task = scroll_sidebar_to_selected(state);
    Task::batch([diff_task, scroll_task])
}

fn toggle_stage_for_targeted_files(state: &mut State) -> Task<Message> {
    let target_indices = state.targeted_file_indices();
    if target_indices.is_empty() {
        return Task::none();
    }

    let target_files: Vec<_> = target_indices
        .into_iter()
        .filter_map(|index| state.files.get(index).cloned())
        .collect();
    if target_files.is_empty() {
        return Task::none();
    }

    let all_staged = target_files.iter().all(|file| file.is_staged());
    let paths: Vec<String> = target_files.iter().map(|file| file.path.clone()).collect();
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

fn modifiers_without_shift(modifiers: keyboard::Modifiers) -> bool {
    !modifiers.control() && !modifiers.alt() && !modifiers.logo()
}

fn no_shortcut_modifiers(modifiers: keyboard::Modifiers) -> bool {
    !modifiers.shift() && !modifiers.control() && !modifiers.alt() && !modifiers.logo()
}
