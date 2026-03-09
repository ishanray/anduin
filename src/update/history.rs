use crate::app::{Commit, Message, SidebarTab, State};
use crate::git::diff::{ChangedFile, FileDiff};
use iced::Task;

pub(crate) fn handle_switch_sidebar_tab(state: &mut State, tab: SidebarTab) -> Task<Message> {
    state.sidebar_tab = tab;
    Task::none()
}

pub(crate) fn handle_commits_loaded(
    state: &mut State,
    result: Result<Vec<Commit>, String>,
) -> Task<Message> {
    let _ = (state, result);
    Task::none()
}

pub(crate) fn handle_select_commit(state: &mut State, index: usize) -> Task<Message> {
    let _ = (state, index);
    Task::none()
}

pub(crate) fn handle_commit_files_loaded(
    state: &mut State,
    result: Result<Vec<ChangedFile>, String>,
) -> Task<Message> {
    let _ = (state, result);
    Task::none()
}

pub(crate) fn handle_select_history_file(state: &mut State, index: usize) -> Task<Message> {
    let _ = (state, index);
    Task::none()
}

pub(crate) fn handle_history_diff_loaded(
    state: &mut State,
    request_id: u64,
    result: Result<FileDiff, String>,
) -> Task<Message> {
    let _ = (state, request_id, result);
    Task::none()
}

pub(crate) fn handle_load_more_commits(state: &mut State) -> Task<Message> {
    let _ = state;
    Task::none()
}
