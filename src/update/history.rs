use crate::actions::{load_commit_file_diff, load_commit_files, load_commits};
use crate::app::{ChangesFocus, Commit, HistoryFocus, Message, SidebarTab, State, StatusTone};
use crate::git::diff::{ChangedFile, FileDiff};
use iced::Task;

const COMMITS_PER_PAGE: usize = 50;

pub(crate) fn handle_switch_sidebar_tab(state: &mut State, tab: SidebarTab) -> Task<Message> {
    if state.sidebar_tab == tab {
        return Task::none();
    }

    state.sidebar_tab = tab;

    match tab {
        SidebarTab::History => {
            state.history_focus = HistoryFocus::CommitList;
            if state.commits.is_empty() && !state.commits_loading {
                load_commits_page(state, 0)
            } else if let Some(index) = state.selected_commit {
                // Re-load diff for the already-selected commit (cleared on tab switch)
                handle_select_commit(state, index)
            } else if !state.commits.is_empty() {
                handle_select_commit(state, 0)
            } else {
                Task::none()
            }
        }
        SidebarTab::Changes => {
            state.changes_focus = ChangesFocus::FileList;
            state.history_diff = None;
            state.history_commit_header = None;
            Task::none()
        }
    }
}

pub(crate) fn handle_commits_loaded(
    state: &mut State,
    result: Result<Vec<Commit>, String>,
) -> Task<Message> {
    state.commits_loading = false;
    match result {
        Ok(new_commits) => {
            if new_commits.len() < COMMITS_PER_PAGE {
                state.commits_exhausted = true;
            }
            let was_empty = state.commits.is_empty();
            state.commits.extend(new_commits);
            // Auto-select first commit on initial load
            if was_empty && !state.commits.is_empty() && state.selected_commit.is_none() {
                return handle_select_commit(state, 0);
            }
        }
        Err(error) => {
            state.set_status_message(error, StatusTone::Error);
        }
    }
    Task::none()
}

pub(crate) fn handle_select_commit(state: &mut State, index: usize) -> Task<Message> {
    state.history_focus = HistoryFocus::CommitList;
    state.selected_commit = Some(index);
    state.history_selected_file = None;
    state.history_selected_path = None;
    state.history_diff = None;
    state.commit_files.clear();

    let Some(commit) = state.commits.get(index).cloned() else {
        return Task::none();
    };

    state.history_commit_header = Some(commit.clone());

    let repo_path = state.repo_path.clone();
    let sha = commit.hash.clone();
    Task::perform(
        async move { load_commit_files(repo_path, sha) },
        Message::CommitFilesLoaded,
    )
}

pub(crate) fn handle_commit_files_loaded(
    state: &mut State,
    result: Result<Vec<ChangedFile>, String>,
) -> Task<Message> {
    match result {
        Ok(files) => {
            state.commit_files = files;
            if !state.commit_files.is_empty() {
                return handle_select_history_file(state, 0);
            }
        }
        Err(error) => {
            state.set_status_message(error, StatusTone::Error);
        }
    }
    Task::none()
}

pub(crate) fn handle_select_history_file(state: &mut State, index: usize) -> Task<Message> {
    state.history_focus = HistoryFocus::FileList;
    let Some(file) = state.commit_files.get(index).cloned() else {
        return Task::none();
    };
    let Some(commit) = state
        .selected_commit
        .and_then(|i| state.commits.get(i).cloned())
    else {
        return Task::none();
    };

    state.history_selected_file = Some(index);
    state.history_selected_path = Some(file.path.clone());

    let request_id = state.next_diff_request();
    let repo_path = state.repo_path.clone();
    let sha = commit.hash.clone();
    let path = file.path.clone();
    let status = file.status;

    Task::perform(
        async move { load_commit_file_diff(repo_path, sha, path, status) },
        move |result| Message::HistoryDiffLoaded(request_id, result),
    )
}

pub(crate) fn handle_history_diff_loaded(
    state: &mut State,
    request_id: u64,
    result: Result<FileDiff, String>,
) -> Task<Message> {
    if request_id != state.active_diff_request {
        return Task::none();
    }

    match result {
        Ok(diff) => {
            state.diff_editor.lose_focus();
            let task = state
                .diff_editor
                .reset(&diff.raw_patch)
                .map(Message::DiffEditor);
            state.history_diff = Some(diff);
            task
        }
        Err(error) => {
            state.set_status_message(error, StatusTone::Error);
            Task::none()
        }
    }
}

pub(crate) fn handle_load_more_commits(state: &mut State) -> Task<Message> {
    if state.commits_loading || state.commits_exhausted {
        return Task::none();
    }
    load_commits_page(state, state.commits.len())
}

fn load_commits_page(state: &mut State, skip: usize) -> Task<Message> {
    state.commits_loading = true;
    let repo_path = state.repo_path.clone();
    Task::perform(
        async move { load_commits(repo_path, COMMITS_PER_PAGE, skip) },
        Message::CommitsLoaded,
    )
}
