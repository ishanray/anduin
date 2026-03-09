use crate::SIDEBAR_ROW_HEIGHT;
use crate::app::{DiffSearchCacheEntry, Message, ProjectSearchResponse, SidebarTarget, State};
use crate::git;
use crate::git::diff::{ChangedFile, FileDiff, FileStatus};
use crate::search::{self, ProjectSearchResult, find_match_line_indices_with_lower};
use crate::tree::expand_parent_dirs;
use crate::views::sidebar::selected_sidebar_row_bounds;
use iced::Task;
use iced::widget::operation::scroll_to;
use iced::widget::scrollable;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub(crate) fn load_changed_files(path: PathBuf) -> Result<Vec<ChangedFile>, String> {
    git::diff::get_changed_files(&path).map_err(|e| e.to_string())
}

pub(crate) fn load_file_diff(
    repo_path: PathBuf,
    file_path: String,
    status: FileStatus,
) -> Result<FileDiff, String> {
    let raw = git::cli::git_diff_file(&repo_path, &file_path).map_err(|e| e.to_string())?;
    Ok(git::diff::parse_unified_diff(&raw, &file_path, status))
}

pub(crate) fn stage_files(repo_path: PathBuf, paths: Vec<String>) -> Result<(), String> {
    git::cli::git_stage_paths(&repo_path, &paths).map_err(|e| e.to_string())
}

pub(crate) fn stage_all_files(repo_path: PathBuf) -> Result<(), String> {
    git::cli::git_stage_all(&repo_path).map_err(|e| e.to_string())
}

pub(crate) fn unstage_files(repo_path: PathBuf, paths: Vec<String>) -> Result<(), String> {
    git::cli::git_unstage_paths(&repo_path, &paths).map_err(|e| e.to_string())
}

pub(crate) fn unstage_all_files(repo_path: PathBuf) -> Result<(), String> {
    git::cli::git_unstage_all(&repo_path).map_err(|e| e.to_string())
}

pub(crate) fn commit_staged_changes(repo_path: PathBuf, summary: String) -> Result<String, String> {
    git::cli::git_commit(&repo_path, &summary).map_err(|e| e.to_string())
}

pub(crate) fn load_project_search_results(
    repo_path: PathBuf,
    files: Vec<ChangedFile>,
    query: String,
    case_sensitive: bool,
    mut diff_search_cache: HashMap<String, DiffSearchCacheEntry>,
) -> Result<ProjectSearchResponse, String> {
    if query.is_empty() {
        return Ok(ProjectSearchResponse {
            results: Vec::new(),
            cache: diff_search_cache,
        });
    }

    let query_lower = (!case_sensitive).then(|| query.to_lowercase());
    let mut results = Vec::new();
    let mut estimated_scroll_y = 0.0;

    for file in files {
        let cache_entry = match diff_search_cache.entry(file.path.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let raw_diff =
                    git::cli::git_diff_file(&repo_path, &file.path).map_err(|e| e.to_string())?;
                entry.insert(DiffSearchCacheEntry {
                    raw_diff: Arc::<str>::from(raw_diff),
                    raw_diff_lower: None,
                })
            }
        };

        if !case_sensitive && cache_entry.raw_diff_lower.is_none() {
            cache_entry.raw_diff_lower =
                Some(Arc::<str>::from(cache_entry.raw_diff.to_lowercase()));
        }

        let match_line_indices = find_match_line_indices_with_lower(
            &cache_entry.raw_diff,
            cache_entry.raw_diff_lower.as_deref(),
            &query,
            query_lower.as_deref(),
            case_sensitive,
        );

        if match_line_indices.is_empty() {
            continue;
        }

        let contexts = search::extract_match_contexts(
            &cache_entry.raw_diff,
            &match_line_indices,
            search::SEARCH_CONTEXT_RADIUS,
        );
        let total_matches = match_line_indices.len();

        results.push(ProjectSearchResult {
            file_path: file.path,
            file_status: file.status,
            matches: contexts,
            total_matches_display: total_matches.to_string(),
            total_matches,
            estimated_scroll_y,
        });

        if let Some(last) = results.last() {
            estimated_scroll_y += search::estimate_result_height(last);
        }
    }

    Ok(ProjectSearchResponse {
        results,
        cache: diff_search_cache,
    })
}

pub(crate) fn load_selected_diff(state: &mut State, index: usize) -> Task<Message> {
    let Some(file) = state.files.get(index).cloned() else {
        return Task::none();
    };

    if let Some(prev_path) = state.selected_path.as_ref() {
        let scroll_y = state.diff_editor.viewport_scroll();
        if scroll_y > 0.0 {
            state.scroll_positions.insert(prev_path.clone(), scroll_y);
        } else {
            state.scroll_positions.remove(prev_path);
        }
    }

    state.selected_file = Some(index);
    state.selected_path = Some(file.path.clone());
    state.focused_sidebar_target = Some(SidebarTarget::File(file.path.clone()));
    state.tree_root_expanded = true;
    expand_parent_dirs(&mut state.expanded_dirs, &file.path);
    state.tree_dirty = true;
    state.ensure_rows_cached();

    let request_id = state.next_diff_request();
    let path = file.path.clone();
    let status = file.status;

    if let Some(cache_entry) = state.diff_search_cache.get(&path) {
        let raw_diff = cache_entry.raw_diff.clone();
        return Task::perform(
            async move {
                Ok(git::diff::parse_unified_diff(
                    raw_diff.as_ref(),
                    &path,
                    status,
                ))
            },
            move |result| Message::DiffLoaded(request_id, result),
        );
    }

    let repo = state.repo_path.clone();
    Task::perform(
        async move { load_file_diff(repo, path, status) },
        move |result| Message::DiffLoaded(request_id, result),
    )
}

pub(crate) fn scroll_sidebar_to_selected(state: &State) -> Task<Message> {
    let Some((row_top, row_bottom)) = selected_sidebar_row_bounds(state) else {
        return Task::none();
    };

    if state.sidebar_viewport_height <= 0.0 {
        return scroll_to(
            state.sidebar_scroll_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: row_top },
        );
    }

    let reveal_padding = SIDEBAR_ROW_HEIGHT;
    let visible_top = state.sidebar_scroll_offset;
    let visible_bottom = visible_top + state.sidebar_viewport_height;

    let target_y = if row_top < visible_top + reveal_padding {
        Some((row_top - reveal_padding).max(0.0))
    } else if row_bottom > visible_bottom - reveal_padding {
        Some((row_bottom - state.sidebar_viewport_height + reveal_padding).max(0.0))
    } else {
        None
    };

    let Some(y) = target_y else {
        return Task::none();
    };

    scroll_to(
        state.sidebar_scroll_id.clone(),
        scrollable::AbsoluteOffset { x: 0.0, y },
    )
}

pub(crate) fn list_branches(repo_path: PathBuf) -> Result<(Vec<String>, String), String> {
    git::cli::git_list_branches(&repo_path).map_err(|e| e.to_string())
}

pub(crate) fn switch_branch(repo_path: PathBuf, branch: String) -> Result<(), String> {
    git::cli::git_switch_branch(&repo_path, &branch).map_err(|e| e.to_string())
}

pub(crate) fn fetch_current_branch(repo_path: PathBuf) -> Result<String, String> {
    git::cli::git_current_branch(&repo_path).map_err(|e| e.to_string())
}

pub(crate) fn maybe_run_project_search(state: &mut State) -> Task<Message> {
    let Some(search) = state.project_search.as_mut() else {
        return Task::none();
    };

    let Some(pending_run_at) = search.pending_run_at else {
        return Task::none();
    };

    if Instant::now() < pending_run_at {
        return Task::none();
    }

    let query = search.query.clone();
    let case_sensitive = search.case_sensitive;
    search.pending_run_at = None;

    if query.is_empty() {
        search.clear_results();
        return Task::none();
    }

    search.searching = true;
    search.rebuild_cached_summaries();
    let request_id = state.next_project_search_request();
    let repo_path = state.repo_path.clone();
    let files = state.files.clone();
    let diff_search_cache = state.diff_search_cache.clone();

    Task::perform(
        async move {
            load_project_search_results(repo_path, files, query, case_sensitive, diff_search_cache)
        },
        move |result| Message::ProjectSearchResults(request_id, result),
    )
}
