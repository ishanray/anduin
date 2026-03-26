use crate::app::{HistoryFocus, State};

pub(crate) fn history_enter_label(state: &State) -> Option<&'static str> {
    match state.history_focus {
        HistoryFocus::CommitList => (state.selected_commit.is_some()
            && !state.commit_files.is_empty())
        .then_some("Focus file list"),
        HistoryFocus::FileList => state.history_diff.is_some().then_some("Focus diff"),
        HistoryFocus::DiffView => None,
    }
}
