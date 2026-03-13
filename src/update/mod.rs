mod diff;
mod history;
mod repo;
mod search;

use crate::actions::{load_selected_diff_without_focus_change, maybe_run_project_search};
use crate::app::{ActivePane, HistoryFocus, Message, SidebarContextMenu, SidebarTarget, State, StatusTone};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use iced::clipboard;
use iced::Task;

pub(crate) fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::FilesLoaded(result) => repo::handle_files_loaded(state, result),
        Message::SelectFile(idx) => repo::handle_select_file(state, idx),
        Message::ToggleRoot(recursive) => {
            state.active_pane = ActivePane::Sidebar;
            state.diff_editor.lose_focus();
            state.clear_explicit_selection();
            state.focused_sidebar_target = Some(SidebarTarget::Root);
            state.toggle_root(recursive);
            state.ensure_rows_cached();
            state.retain_sidebar_selection();
            state.ensure_sidebar_focus();
            if let Some(file_idx) = state.first_file_index_for_root() {
                load_selected_diff_without_focus_change(state, file_idx)
            } else {
                Task::none()
            }
        }
        Message::ToggleDir(path, recursive) => {
            state.active_pane = ActivePane::Sidebar;
            state.diff_editor.lose_focus();
            state.clear_explicit_selection();
            state.focused_sidebar_target = Some(SidebarTarget::Dir(path.clone()));
            state.toggle_dir(&path, recursive);
            state.ensure_rows_cached();
            state.retain_sidebar_selection();
            state.ensure_sidebar_focus();
            if let Some(file_idx) = state.first_file_index_for_dir(&path) {
                load_selected_diff_without_focus_change(state, file_idx)
            } else {
                Task::none()
            }
        }
        Message::DiffLoaded(request_id, result) => {
            diff::handle_diff_loaded(state, request_id, result)
        }
        Message::DiffEditor(message) => diff::handle_diff_editor(state, message),
        Message::OpenRepo => repo::handle_open_repo(),
        Message::RepoOpened(path) => repo::handle_repo_opened(state, path),
        Message::ToggleTheme => repo::handle_toggle_theme(state),
        Message::WatchEvent(event) => repo::handle_watch_event(state, event),
        Message::KeyboardEvent(event) => repo::handle_keyboard_event(state, event),
        Message::SidebarScrolled(offset, height) => {
            repo::handle_sidebar_scrolled(state, offset, height)
        }
        Message::OpenProjectSearch => repo::handle_open_project_search(state),
        Message::CloseProjectSearch => {
            if let Some(search) = state.project_search.as_mut() {
                search.is_open = false;
            }
            state.pending_diff_jump = None;
            state.active_pane = ActivePane::Sidebar;
            state.diff_editor.lose_focus();
            Task::none()
        }
        Message::OpenCommitComposer => repo::handle_open_commit_composer(state),
        Message::CloseCommitComposer => repo::handle_close_commit_composer(state),
        Message::CommitSummaryChanged(summary) => {
            repo::handle_commit_summary_changed(state, summary)
        }
        Message::SubmitCommit => repo::handle_submit_commit(state),
        Message::GitOperationFinished(result) => repo::handle_git_operation_finished(state, result),
        Message::CommitFinished(result) => repo::handle_commit_finished(state, result),
        Message::ProjectSearchQueryChanged(query) => {
            search::handle_project_search_query_changed(state, query)
        }
        Message::ProjectSearchToggleCase => search::handle_project_search_toggle_case(state),
        Message::ProjectSearchTick => maybe_run_project_search(state),
        Message::ProjectSearchResults(request_id, result) => {
            search::handle_project_search_results(state, request_id, result)
        }
        Message::ProjectSearchJumpTo(file_path, line_number) => {
            search::handle_project_search_jump_to(state, file_path, line_number)
        }
        Message::ToggleActionsPanel => {
            state.show_actions_panel = !state.show_actions_panel;
            Task::none()
        }
        Message::OpenBranchPicker => repo::handle_open_branch_picker(state),
        Message::BranchesFetched(result) => repo::handle_branches_fetched(state, result),
        Message::BranchPickerFilterChanged(filter) => {
            repo::handle_branch_picker_filter_changed(state, filter)
        }
        Message::SwitchBranch(branch) => repo::handle_switch_branch(state, branch),
        Message::BranchSwitched(result) => repo::handle_branch_switched(state, result),
        Message::CreateBranch(branch) => repo::handle_create_branch(state, branch),
        Message::BranchCreated(result) => repo::handle_branch_created(state, result),
        Message::CurrentBranchFetched(result) => {
            repo::handle_current_branch_fetched(state, result)
        }
        Message::OpenProjectPicker => repo::handle_open_project_picker(state),
        Message::CloseProjectPicker => repo::handle_close_project_picker(state),
        Message::ProjectPickerFilterChanged(filter) => {
            repo::handle_project_picker_filter_changed(state, filter)
        }
        Message::SwitchProject(repo) => repo::handle_switch_project(state, repo),
        Message::SwitchSidebarTab(tab) => history::handle_switch_sidebar_tab(state, tab),
        Message::CommitsLoaded(result) => history::handle_commits_loaded(state, result),
        Message::SelectCommit(index) => {
            state.history_focus = HistoryFocus::CommitList;
            history::handle_select_commit(state, index)
        }
        Message::CommitFilesLoaded(result) => history::handle_commit_files_loaded(state, result),
        Message::SelectHistoryFile(index) => {
            state.history_focus = HistoryFocus::FileList;
            history::handle_select_history_file(state, index)
        }
        Message::HistoryDiffLoaded(request_id, result) => {
            history::handle_history_diff_loaded(state, request_id, result)
        }
        Message::CopyCommitHash(hash) => clipboard::write(hash).discard(),
        Message::RequestDiscard => repo::handle_request_discard(state),
        Message::ConfirmDiscard => repo::handle_confirm_discard(state),
        Message::CancelDiscard => repo::handle_cancel_discard(state),
        Message::WindowResized(size) => {
            state.window_size = Some(size);
            state.pending_settings_save =
                Some(Instant::now() + Duration::from_millis(500));
            Task::none()
        }
        Message::SettingsSaveTick => {
            if state
                .pending_settings_save
                .is_some_and(|at| Instant::now() >= at)
            {
                state.pending_settings_save = None;
                state.persist_settings();
            }
            Task::none()
        }
        Message::FocusRoot => {
            state.active_pane = ActivePane::Sidebar;
            state.diff_editor.lose_focus();
            state.clear_explicit_selection();
            state.focused_sidebar_target = Some(SidebarTarget::Root);
            state.ensure_sidebar_focus();
            if let Some(file_idx) = state.first_file_index_for_root() {
                load_selected_diff_without_focus_change(state, file_idx)
            } else {
                Task::none()
            }
        }
        Message::FocusDir(path) => {
            state.active_pane = ActivePane::Sidebar;
            state.diff_editor.lose_focus();
            state.clear_explicit_selection();
            state.focused_sidebar_target = Some(SidebarTarget::Dir(path.clone()));
            state.ensure_sidebar_focus();
            if let Some(file_idx) = state.first_file_index_for_dir(&path) {
                load_selected_diff_without_focus_change(state, file_idx)
            } else {
                Task::none()
            }
        }
        Message::ShowContextMenu { path, is_dir, row_index } => {
            state.sidebar_context_menu = Some(SidebarContextMenu {
                path,
                is_dir,
                row_index,
            });
            Task::none()
        }
        Message::CloseContextMenu => {
            state.sidebar_context_menu = None;
            Task::none()
        }
        Message::AddToGitignore(path) => {
            state.sidebar_context_menu = None;
            let repo_path = state.repo_path.clone();
            Task::perform(
                async move { add_to_gitignore(repo_path, path) },
                Message::GitignoreFinished,
            )
        }
        Message::GitignoreFinished(result) => {
            match result {
                Ok(msg) => state.set_status_message(msg, StatusTone::Success),
                Err(msg) => state.set_status_message(msg, StatusTone::Error),
            }
            Task::none()
        }
        Message::OpenInEditor(rel_path) => {
            let full_path = state.repo_path.join(&rel_path);

            // Use macOS `open` to delegate to the default app and reuse
            // the existing window instead of spawning a fresh instance.
            if let Err(e) = Command::new("open").arg(&full_path).spawn() {
                eprintln!("Failed to open '{}': {e}", full_path.display());
            }
            Task::none()
        }
        Message::WindowCloseRequested => {
            state.persist_settings();
            iced::exit()
        }
        Message::CommitListScrolled(offset, viewport_height, content_height) => {
            state.commit_list_scroll_offset = offset;
            state.commit_list_viewport_height = viewport_height;
            // Auto-load more commits when scrolled near the bottom
            let near_bottom = offset + viewport_height >= content_height - 100.0;
            if near_bottom && !state.commits_loading && !state.commits_exhausted {
                history::handle_load_more_commits(state)
            } else {
                Task::none()
            }
        }
    }
}

/// Add a path to `.gitignore` and remove it from the index if tracked.
fn add_to_gitignore(
    repo_path: PathBuf,
    rel_path: String,
) -> Result<String, String> {
    use std::fs;
    use std::io::Write;

    let gitignore_path = repo_path.join(".gitignore");

    // Determine the ignore pattern: use trailing slash for directories
    let full_path = repo_path.join(&rel_path);
    let pattern = if full_path.is_dir() {
        format!("{rel_path}/")
    } else {
        rel_path.clone()
    };

    // Check if the pattern is already in .gitignore
    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)
            .map_err(|e| format!("Failed to read .gitignore: {e}"))?;
        let trimmed_pattern = pattern.trim_end_matches('/');
        for line in content.lines() {
            let trimmed_line = line.trim().trim_end_matches('/');
            if trimmed_line == trimmed_pattern {
                // Already ignored — still try to remove from index
                remove_from_index(&repo_path, &rel_path);
                return Ok(format!("{rel_path} already in .gitignore"));
            }
        }
    }

    // Append to .gitignore (ensure it ends with a newline)
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore_path)
        .map_err(|e| format!("Failed to open .gitignore: {e}"))?;

    // Ensure we start on a new line
    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path).unwrap_or_default();
        if !content.is_empty() && !content.ends_with('\n') {
            writeln!(file).map_err(|e| format!("Failed to write .gitignore: {e}"))?;
        }
    }

    writeln!(file, "{pattern}").map_err(|e| format!("Failed to write .gitignore: {e}"))?;

    // Remove from index if tracked
    remove_from_index(&repo_path, &rel_path);

    Ok(format!("Added {rel_path} to .gitignore"))
}

/// Remove a path from the git index (cached) without deleting the working copy.
fn remove_from_index(repo_path: &Path, rel_path: &str) {
    // Try `git rm -r --cached` — if the path isn't tracked, this is a no-op error we ignore.
    let _ = Command::new("git")
        .args(["rm", "-r", "--cached", "--quiet", rel_path])
        .current_dir(repo_path)
        .output();
}
