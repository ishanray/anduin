mod diff;
mod repo;
mod search;

use crate::actions::maybe_run_project_search;
use crate::app::{Message, State};
use iced::Task;

pub(crate) fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::FilesLoaded(result) => repo::handle_files_loaded(state, result),
        Message::SelectFile(idx) => repo::handle_select_file(state, idx),
        Message::ToggleRoot(recursive) => {
            state.toggle_root(recursive);
            state.ensure_rows_cached();
            Task::none()
        }
        Message::ToggleDir(path, recursive) => {
            state.toggle_dir(&path, recursive);
            state.ensure_rows_cached();
            Task::none()
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
        Message::OpenProjectSearch => repo::handle_open_project_search(state),
        Message::CloseProjectSearch => {
            state.project_search = None;
            state.pending_diff_jump = None;
            Task::none()
        }
        Message::OpenCommitComposer => repo::handle_open_commit_composer(state),
        Message::CloseCommitComposer => repo::handle_close_commit_composer(state),
        Message::CommitSummaryChanged(summary) => repo::handle_commit_summary_changed(state, summary),
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
        Message::ProjectSearchScrollToFile(file_path) => {
            search::handle_project_search_scroll_to_file(state, &file_path)
        }
        Message::ProjectSearchJumpTo(file_path, line_number) => {
            search::handle_project_search_jump_to(state, file_path, line_number)
        }
    }
}
