use crate::actions::{load_selected_diff, maybe_run_project_search, scroll_sidebar_to_selected};
use crate::app::{Message, PendingDiffJump, ProjectSearchResponse, State};
use iced::Task;
use iced::widget::operation::scroll_to;
use iced::widget::scrollable;

pub(crate) fn handle_project_search_query_changed(
    state: &mut State,
    query: String,
) -> Task<Message> {
    if let Some(search) = state.project_search.as_mut() {
        search.query = query;
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

pub(crate) fn handle_project_search_scroll_to_file(
    state: &State,
    file_path: &str,
) -> Task<Message> {
    let Some(search) = state.project_search.as_ref() else {
        return Task::none();
    };
    let Some(&idx) = search.result_index_by_path.get(file_path) else {
        return Task::none();
    };
    let Some(result) = search.results.get(idx) else {
        return Task::none();
    };

    scroll_to(
        search.results_scroll_id.clone(),
        scrollable::AbsoluteOffset {
            x: 0.0,
            y: result.estimated_scroll_y,
        },
    )
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
    state.project_search = None;

    if let Some(index) = state.files.iter().position(|file| file.path == file_path) {
        let diff_task = load_selected_diff(state, index);
        let scroll_task = scroll_sidebar_to_selected(state);
        Task::batch([diff_task, scroll_task])
    } else {
        Task::none()
    }
}
