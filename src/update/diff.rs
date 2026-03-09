use crate::app::{DiffSearchCacheEntry, Message, State};
use crate::git;
use iced::Task;
use iced_code_editor::Message as EditorMessage;
use std::sync::Arc;

pub(crate) fn handle_diff_loaded(
    state: &mut State,
    request_id: u64,
    result: Result<git::diff::FileDiff, String>,
) -> Task<Message> {
    match result {
        Ok(diff) => {
            if request_id != state.active_diff_request {
                return Task::none();
            }

            state.diff_search_cache.insert(
                diff.path.clone(),
                DiffSearchCacheEntry {
                    raw_diff: Arc::<str>::from(diff.raw_patch.clone()),
                    raw_diff_lower: None,
                },
            );

            let patch_changed = state.current_diff.as_ref().is_none_or(|current| {
                current.raw_patch != diff.raw_patch || current.path != diff.path
            });

            state.current_diff = Some(diff);

            if patch_changed {
                if let Some(current_diff) = state.current_diff.as_ref() {
                    let jump_line = state
                        .pending_diff_jump
                        .as_ref()
                        .filter(|jump| jump.path == current_diff.path)
                        .map(|jump| jump.line_number);

                    let saved_scroll = state
                        .scroll_positions
                        .remove(&current_diff.path)
                        .unwrap_or(0.0);

                    let task = if let Some(line_number) = jump_line {
                        let mut task = state
                            .diff_editor
                            .reset(&current_diff.raw_patch)
                            .map(Message::DiffEditor);
                        task = Task::batch([
                            task,
                            state
                                .diff_editor
                                .scroll_to_line(line_number)
                                .map(Message::DiffEditor),
                        ]);
                        state.pending_diff_jump = None;
                        task
                    } else if saved_scroll > 0.0 {
                        state
                            .diff_editor
                            .reset_with_scroll(&current_diff.raw_patch, saved_scroll)
                            .map(Message::DiffEditor)
                    } else {
                        state
                            .diff_editor
                            .reset(&current_diff.raw_patch)
                            .map(Message::DiffEditor)
                    };
                    state.diff_editor.request_focus();
                    task
                } else {
                    Task::none()
                }
            } else {
                if state.pending_diff_jump.as_ref().is_some_and(|jump| {
                    state
                        .current_diff
                        .as_ref()
                        .is_some_and(|diff| diff.path == jump.path)
                }) {
                    let line_number = state
                        .pending_diff_jump
                        .take()
                        .map(|jump| jump.line_number)
                        .unwrap_or(0);
                    return state
                        .diff_editor
                        .scroll_to_line(line_number)
                        .map(Message::DiffEditor);
                }
                Task::none()
            }
        }
        Err(error) => {
            if request_id != state.active_diff_request {
                return Task::none();
            }

            state.error = Some(error);
            Task::none()
        }
    }
}

pub(crate) fn handle_diff_editor(state: &mut State, message: EditorMessage) -> Task<Message> {
    use iced_code_editor::Message as M;

    match message {
        M::MouseClick(_)
        | M::MouseDrag(_)
        | M::MouseRelease
        | M::Scrolled(_)
        | M::PageUp
        | M::PageDown
        | M::Home(_)
        | M::End(_)
        | M::ArrowKey(_, _)
        | M::CtrlHome
        | M::CtrlEnd
        | M::CanvasFocusGained
        | M::CanvasFocusLost
        | M::Tick
        | M::Copy
        | M::OpenSearch
        | M::CloseSearch
        | M::SearchQueryChanged(_)
        | M::ToggleCaseSensitive
        | M::FindNext
        | M::FindPrevious
        | M::SearchDialogTab
        | M::SearchDialogShiftTab => state.diff_editor.update(&message).map(Message::DiffEditor),

        M::CharacterInput(_)
        | M::Backspace
        | M::Delete
        | M::Enter
        | M::Tab
        | M::Paste(_)
        | M::DeleteSelection
        | M::Undo
        | M::Redo
        | M::ImeOpened
        | M::ImePreedit(_, _)
        | M::ImeCommit(_)
        | M::ImeClosed
        | M::OpenSearchReplace
        | M::ReplaceQueryChanged(_)
        | M::ReplaceNext
        | M::ReplaceAll
        | M::FocusNavigationTab
        | M::FocusNavigationShiftTab => Task::none(),
    }
}
