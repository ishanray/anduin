use crate::app::{HistoryFocus, SidebarTab, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionsSurfaceCommand {
    OpenRepo,
    OpenBranchPicker,
    OpenProjectPicker,
    OpenProjectSearch,
    OpenCommitComposer,
    SearchDiff,
    SwitchToHistoryTab,
    SwitchToChangesTab,
    CopyCommitHash,
}

impl ActionsSurfaceCommand {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::OpenRepo => "o",
            Self::OpenBranchPicker => "b",
            Self::OpenProjectPicker => "p",
            Self::OpenProjectSearch => "f",
            Self::OpenCommitComposer => "c",
            Self::SearchDiff => "/",
            Self::SwitchToHistoryTab => "h",
            Self::SwitchToChangesTab => "t",
            Self::CopyCommitHash => "y",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OpenRepo => "Open repository",
            Self::OpenBranchPicker => "Switch branch",
            Self::OpenProjectPicker => "Switch project",
            Self::OpenProjectSearch => "Project search",
            Self::OpenCommitComposer => "Commit staged changes",
            Self::SearchDiff => "Search diff",
            Self::SwitchToHistoryTab => "Open history tab",
            Self::SwitchToChangesTab => "Switch to changes tab",
            Self::CopyCommitHash => "Copy commit hash",
        }
    }

    pub(crate) fn section(self) -> &'static str {
        match self {
            Self::OpenRepo => "Start",
            Self::OpenBranchPicker | Self::OpenProjectPicker => "Repo",
            Self::OpenProjectSearch
            | Self::SearchDiff
            | Self::SwitchToHistoryTab
            | Self::SwitchToChangesTab => "Search & View",
            Self::OpenCommitComposer => "Commit",
            Self::CopyCommitHash => "Commit",
        }
    }
}

pub(crate) fn available_actions_panel_commands(state: &State) -> Vec<ActionsSurfaceCommand> {
    let mut commands = Vec::new();

    let no_repo =
        state.current_branch.is_none() && state.files.is_empty() && state.commits.is_empty();
    if no_repo {
        commands.push(ActionsSurfaceCommand::OpenRepo);
        commands.push(ActionsSurfaceCommand::OpenProjectPicker);
        return commands;
    }

    commands.push(ActionsSurfaceCommand::OpenBranchPicker);
    commands.push(ActionsSurfaceCommand::OpenProjectPicker);
    commands.push(ActionsSurfaceCommand::OpenProjectSearch);

    match state.sidebar_tab {
        SidebarTab::Changes => {
            if state.current_diff.is_some() {
                commands.push(ActionsSurfaceCommand::SearchDiff);
            }
            commands.push(ActionsSurfaceCommand::SwitchToHistoryTab);
            if state.staged_file_count() > 0 {
                commands.push(ActionsSurfaceCommand::OpenCommitComposer);
            }
        }
        SidebarTab::History => {
            if state.history_diff.is_some() {
                commands.push(ActionsSurfaceCommand::SearchDiff);
            }
            commands.push(ActionsSurfaceCommand::SwitchToChangesTab);
            if state.history_commit_header.is_some() {
                commands.push(ActionsSurfaceCommand::CopyCommitHash);
            }
        }
    }

    commands
}

pub(crate) fn is_actions_panel_command_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "o" | "b" | "p" | "f" | "c" | "/" | "h" | "t" | "y"
    )
}

pub(crate) fn actions_panel_command_for_key(
    state: &State,
    key: &str,
) -> Option<ActionsSurfaceCommand> {
    let normalized = key.to_ascii_lowercase();
    available_actions_panel_commands(state)
        .into_iter()
        .find(|command| command.key() == normalized)
}

pub(crate) fn history_enter_label(state: &State) -> Option<&'static str> {
    match state.history_focus {
        HistoryFocus::CommitList => (state.selected_commit.is_some()
            && !state.commit_files.is_empty())
        .then_some("Focus file list"),
        HistoryFocus::FileList => state.history_diff.is_some().then_some("Focus diff"),
        HistoryFocus::DiffView => None,
    }
}
