use crate::actions::{load_selected_diff, maybe_run_project_search, scroll_sidebar_to_selected};
use crate::app::{Message, PendingDiffJump, ProjectSearchResponse, State};
use iced::Task;

pub(crate) fn handle_project_search_query_changed(
    state: &mut State,
    query: String,
) -> Task<Message> {
    if let Some(search) = state.project_search.as_mut() {
        search.query = query;
        search.input_focused = true;
        search.update_query_lower();
        if search.query.is_empty() {
            search.pending_run_at = None;
            search.clear_results();
            return Task::none();
        }
    }
    state.queue_project_search();
    Task::none()
}

pub(crate) fn handle_project_search_toggle_case(state: &mut State) -> Task<Message> {
    if let Some(search) = state.project_search.as_mut() {
        search.case_sensitive = !search.case_sensitive;
        search.update_query_lower();
    }
    state.queue_project_search();
    maybe_run_project_search(state)
}

pub(crate) fn handle_project_search_results(
    state: &mut State,
    request_id: u64,
    result: Result<ProjectSearchResponse, String>,
) -> Task<Message> {
    match result {
        Ok(response) => {
            let Some(search) = state.project_search.as_ref() else {
                return Task::none();
            };

            if request_id != search.request_id {
                return Task::none();
            }

            state.diff_search_cache = response.cache;
            if let Some(search) = state.project_search.as_mut() {
                search.set_results(response.results);
            }

            // Auto-select first matching file if current selection has no matches
            let current_has_match = state.project_search.as_ref().is_some_and(|s| {
                state
                    .selected_path
                    .as_ref()
                    .is_some_and(|p| s.result_index_by_path.contains_key(p))
            });

            if let Some(index) = (!current_has_match)
                .then(|| {
                    state
                        .project_search
                        .as_ref()
                        .and_then(|s| s.results.first())
                        .and_then(|r| {
                            state.files.iter().position(|f| f.path == r.file_path)
                        })
                })
                .flatten()
            {
                let diff_task = load_selected_diff(state, index);
                let scroll_task = scroll_sidebar_to_selected(state);
                return Task::batch([diff_task, scroll_task]);
            }
            Task::none()
        }
        Err(error) => {
            let Some(search) = state.project_search.as_mut() else {
                return Task::none();
            };

            if request_id != search.request_id {
                return Task::none();
            }

            search.searching = false;
            state.error = Some(error);
            Task::none()
        }
    }
}

pub(crate) fn handle_project_search_jump_to(
    state: &mut State,
    file_path: String,
    line_number: usize,
) -> Task<Message> {
    state.pending_diff_jump = Some(PendingDiffJump {
        path: file_path.clone(),
        line_number,
    });
    if let Some(search) = state.project_search.as_mut() {
        search.is_open = false;
    }

    if let Some(index) = state.files.iter().position(|file| file.path == file_path) {
        let diff_task = load_selected_diff(state, index);
        let scroll_task = scroll_sidebar_to_selected(state);
        Task::batch([diff_task, scroll_task])
    } else {
        Task::none()
    }
}
