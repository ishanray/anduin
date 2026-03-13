use crate::MONO;
use crate::actions_ui::{
    ActionsSurfaceCommand, available_actions_panel_commands, history_enter_label,
};
use crate::app::{HistoryFocus, Message, SidebarTab, SidebarTarget, State};
use iced::theme::palette::Extended;
use iced::widget::{Space, column, container, row, text};
use iced::{Border, Element, Fill, Font, Theme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FooterAction {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FooterSection {
    pub(crate) title: String,
    pub(crate) actions: Vec<FooterAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FooterModel {
    pub(crate) target_label: String,
    pub(crate) mode_label: String,
    pub(crate) preview_actions: Vec<FooterAction>,
    pub(crate) sections: Vec<FooterSection>,
}

fn action(key: &str, label: &str) -> FooterAction {
    FooterAction {
        key: key.to_owned(),
        label: label.to_owned(),
        enabled: true,
    }
}

fn maybe_action(key: &str, label: &str, enabled: bool) -> Option<FooterAction> {
    enabled.then(|| action(key, label))
}

fn section(title: &str, actions: Vec<FooterAction>) -> FooterSection {
    FooterSection {
        title: title.to_owned(),
        actions,
    }
}

fn section_opt(title: &str, actions: Vec<Option<FooterAction>>) -> Option<FooterSection> {
    let actions: Vec<_> = actions.into_iter().flatten().collect();
    (!actions.is_empty()).then(|| section(title, actions))
}

fn display_target(state: &State) -> String {
    if state.current_branch.is_none() && state.files.is_empty() && state.commits.is_empty() {
        return "no repository".to_owned();
    }

    match state.sidebar_tab {
        SidebarTab::Changes => {
            if let Some(target) = state.focused_sidebar_target.as_ref() {
                match target {
                    SidebarTarget::Root => "repository".to_owned(),
                    SidebarTarget::Dir(path) => format!("{path}/"),
                    SidebarTarget::File(path) => path.clone(),
                }
            } else {
                "repository".to_owned()
            }
        }
        SidebarTab::History => match state.history_focus {
            HistoryFocus::CommitList => state
                .history_commit_header
                .as_ref()
                .map(|commit| format!("commit {}", commit.short_hash))
                .unwrap_or_else(|| "history".to_owned()),
            HistoryFocus::FileList | HistoryFocus::DiffView => {
                if let Some(path) = state.history_selected_path.as_ref() {
                    if let Some(commit) = state.history_commit_header.as_ref() {
                        format!("{path} @ {}", commit.short_hash)
                    } else {
                        path.clone()
                    }
                } else if state.history_focus == HistoryFocus::DiffView {
                    "diff view".to_owned()
                } else {
                    state
                        .history_commit_header
                        .as_ref()
                        .map(|commit| format!("commit {}", commit.short_hash))
                        .unwrap_or_else(|| "history".to_owned())
                }
            }
        },
    }
}

pub(crate) fn build_footer_model(state: &State) -> FooterModel {
    if state.current_branch.is_none() && state.files.is_empty() && state.commits.is_empty() {
        let command_actions = available_actions_panel_commands(state);
        return FooterModel {
            target_label: "no repository".to_owned(),
            mode_label: "current: none".to_owned(),
            preview_actions: vec![action(".", "Actions"), action("o", "Open repo")],
            sections: vec![section(
                "Start",
                command_actions.into_iter().map(action_from_command).collect(),
            )],
        };
    }

    match state.sidebar_tab {
        SidebarTab::Changes => build_changes_model(state),
        SidebarTab::History => build_history_model(state),
    }
}

fn action_from_command(command: ActionsSurfaceCommand) -> FooterAction {
    action(command.key(), command.label())
}

fn command_actions_for_section(
    commands: &[ActionsSurfaceCommand],
    title: &str,
) -> Vec<FooterAction> {
    commands
        .iter()
        .copied()
        .filter(|command| command.section() == title)
        .map(action_from_command)
        .collect()
}

fn section_commands(
    commands: &[ActionsSurfaceCommand],
    title: &str,
) -> Option<FooterSection> {
    let actions = command_actions_for_section(commands, title);
    (!actions.is_empty()).then(|| section(title, actions))
}

fn build_changes_model(state: &State) -> FooterModel {
    let target_label = display_target(state);
    let mode_label = if state.files.is_empty() {
        "current: clean repo".to_owned()
    } else if matches!(state.focused_sidebar_target, Some(SidebarTarget::File(_))) {
        "current: file".to_owned()
    } else if matches!(state.focused_sidebar_target, Some(SidebarTarget::Dir(_))) {
        "current: dir".to_owned()
    } else {
        "current: repository".to_owned()
    };

    let has_unstaged = state.unstaged_file_count() > 0;
    let has_staged = state.staged_file_count() > 0;
    let has_changes = !state.files.is_empty();
    let has_diff = state.current_diff.is_some();

    let preview_actions = if has_changes {
        let mut actions = vec![action(".", "Actions")];
        if has_unstaged || has_staged {
            actions.push(action("space", "Stage"));
        }
        if has_staged {
            actions.push(action("c", "Commit"));
        }
        actions
    } else {
        vec![action(".", "Actions")]
    };

    let command_actions = available_actions_panel_commands(state);

    let sections = vec![
        section_opt(
            "Files",
            vec![
                maybe_action("space", "Stage / Unstage", has_changes),
                maybe_action("a", "Stage all", has_unstaged),
                maybe_action("u", "Unstage all", has_staged),
                maybe_action("k", "Discard", has_changes),
            ],
        ),
        section_commands(&command_actions, "Commit"),
        section_commands(&command_actions, "Repo"),
        section_commands(&command_actions, "Search & View"),
        section_opt(
            "Navigation",
            vec![
                maybe_action("↑↓", "Move selection", has_changes),
                maybe_action("←→", "Collapse / Expand", has_changes),
                maybe_action("enter", "Focus diff", has_diff),
            ],
        ),
    ]
    .into_iter()
    .flatten()
    .collect();

    FooterModel {
        target_label,
        mode_label,
        preview_actions,
        sections,
    }
}

fn build_history_model(state: &State) -> FooterModel {
    let target_label = display_target(state);
    let mode_label = "current: history".to_owned();
    let has_commit = state.history_commit_header.is_some();
    let has_file = state.history_selected_path.is_some();

    let preview_actions = vec![action(".", "Actions")];

    let history_move_enabled = has_commit || has_file;
    let command_actions = available_actions_panel_commands(state);

    let sections = vec![
        section_opt(
            "History",
            vec![
                maybe_action("↑↓", "Move history selection", history_move_enabled),
                history_enter_label(state).map(|label| action("enter", label)),
            ],
        ),
        section_commands(&command_actions, "Commit"),
        section_commands(&command_actions, "Search & View"),
        section_commands(&command_actions, "Repo"),
    ]
    .into_iter()
    .flatten()
    .collect();

    FooterModel {
        target_label,
        mode_label,
        preview_actions,
        sections,
    }
}

fn keycap<'a>(label: &str, palette: &Extended, enabled: bool) -> Element<'a, Message> {
    let bg = if enabled {
        palette.background.strong.color
    } else {
        palette.background.weak.color
    };
    let fg = if enabled {
        palette.background.strong.text
    } else {
        palette.background.strong.text.scale_alpha(0.5)
    };
    let border_color = palette.background.base.text.scale_alpha(0.15);

    container(text(label.to_owned()).size(12).font(MONO).color(fg))
        .padding([3, 7])
        .style(move |_: &Theme| {
            container::Style::default().background(bg).border(Border {
                color: border_color,
                width: 1.0,
                radius: 4.0.into(),
            })
        })
        .into()
}

fn action_chip<'a>(action: &FooterAction, palette: &Extended) -> Element<'a, Message> {
    let fg = if action.enabled {
        palette.background.base.text
    } else {
        palette.background.base.text.scale_alpha(0.5)
    };

    row![
        keycap(&action.key, palette, action.enabled),
        text(action.label.clone())
            .size(13)
            .font(Font::DEFAULT)
            .color(fg),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .into()
}

pub(crate) fn view_actions_footer(state: &State) -> Element<'_, Message> {
    let model = build_footer_model(state);
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let panel_bg = palette.background.base.color;
    let border = palette.background.strong.color;
    let title_fg = palette.background.base.text;
    let subtle_fg = palette.background.strong.text.scale_alpha(0.7);

    let mut preview = row![].spacing(12);
    for item in &model.preview_actions {
        preview = preview.push(action_chip(item, palette));
    }

    let closed = row![
        preview.align_y(iced::Alignment::Center),
        Space::new().width(Fill),
        text(model.mode_label.clone())
            .size(12)
            .font(MONO)
            .color(subtle_fg),
    ]
    .align_y(iced::Alignment::Center);

    let content: Element<'_, Message> = if state.show_actions_panel {
        let header = row![
            text(format!("Actions for: {}", model.target_label))
                .size(14)
                .color(title_fg),
            Space::new().width(Fill),
            action_chip(&action("esc", "close"), palette),
        ]
        .align_y(iced::Alignment::Center);

        let mut sections_col = column![header, Space::new().height(8.0)];
        for section_model in &model.sections {
            let mut actions_row = row![
                text(section_model.title.clone())
                    .size(13)
                    .font(MONO)
                    .color(subtle_fg)
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center);
            for item in &section_model.actions {
                actions_row = actions_row.push(action_chip(item, palette));
            }
            sections_col = sections_col
                .push(actions_row)
                .push(Space::new().height(6.0));
        }

        column![sections_col, Space::new().height(6.0), closed]
            .spacing(6)
            .into()
    } else {
        closed.into()
    };

    container(content)
        .width(Fill)
        .padding([8, 12])
        .style(move |_: &Theme| {
            container::Style::default()
                .background(panel_bg)
                .border(Border {
                    color: border,
                    width: 1.0,
                    radius: 0.0.into(),
                })
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::build_footer_model;
    use crate::app::{
        ActivePane, ChangesFocus, Commit, HistoryFocus, SidebarTab, SidebarTarget, State, ThemeMode,
    };
    use crate::git::diff::{ChangedFile, FileStatus};
    use iced::widget::Id;
    use iced_code_editor::CodeEditor;
    use std::collections::{HashMap, HashSet};

    fn test_state() -> State {
        let theme_mode = ThemeMode::Dark;
        let mut diff_editor = CodeEditor::new("", "diff");
        diff_editor.lose_focus();

        State {
            repo_path: "/tmp/test-repo".into(),
            files: Vec::new(),
            selected_file: None,
            selected_path: None,
            focused_sidebar_target: None,
            selected_sidebar_targets: HashSet::new(),
            selection_anchor_sidebar_target: None,
            current_diff: None,
            diff_editor,
            theme_mode,
            error: None,
            status_message: None,
            commit_composer: None,
            expanded_dirs: HashSet::new(),
            tree_root_expanded: true,
            alt_pressed: false,
            initialized_tree: false,
            cached_rows: Vec::new(),
            tree_dirty: true,
            refresh_in_flight: false,
            refresh_queued: false,
            active_diff_request: 0,
            diff_search_cache: HashMap::new(),
            scroll_positions: HashMap::new(),
            project_search: None,
            pending_diff_jump: None,
            sidebar_scroll_id: Id::unique(),
            sidebar_scroll_offset: 0.0,
            sidebar_viewport_height: 0.0,
            active_pane: ActivePane::Sidebar,
            cached_theme: theme_mode.app_theme(),
            show_actions_panel: false,
            current_branch: None,
            branch_picker: None,
            project_picker: None,
            recent_repos: Vec::new(),
            sidebar_tab: SidebarTab::Changes,
            commits: Vec::new(),
            selected_commit: None,
            commit_files: Vec::new(),
            commits_loading: false,
            commits_exhausted: false,
            history_selected_file: None,
            history_selected_path: None,
            history_diff: None,
            history_commit_header: None,
            history_focus: HistoryFocus::CommitList,
            changes_focus: ChangesFocus::FileList,
            commit_list_scroll_id: Id::unique(),
            commit_list_scroll_offset: 0.0,
            commit_list_viewport_height: 0.0,
            discard_confirm: None,
            sidebar_context_menu: None,
            window_size: None,
            pending_settings_save: None,
        }
    }

    fn sample_commit() -> Commit {
        Commit {
            hash: "abc1234".to_owned(),
            short_hash: "abc1234".to_owned(),
            author: "Test User".to_owned(),
            date: "2026-03-12".to_owned(),
            message: "Test commit".to_owned(),
        }
    }

    #[test]
    fn no_repo_model_shows_start_actions() {
        let state = test_state();
        let model = build_footer_model(&state);

        assert_eq!(model.target_label, "no repository");
        assert_eq!(model.mode_label, "current: none");
        assert_eq!(model.preview_actions[1].label, "Open repo");
        assert_eq!(model.sections[0].title, "Start");
    }

    #[test]
    fn changes_file_model_shows_file_target_and_only_available_actions() {
        let mut state = test_state();
        state.current_branch = Some("main".to_owned());
        state.files = vec![ChangedFile {
            path: "src/main.rs".to_owned(),
            status: FileStatus::Modified,
            staged: false,
            unstaged: true,
        }];
        state.focused_sidebar_target = Some(SidebarTarget::File("src/main.rs".to_owned()));

        let model = build_footer_model(&state);

        assert_eq!(model.target_label, "src/main.rs");
        assert_eq!(model.mode_label, "current: file");
        assert_eq!(model.preview_actions[1].label, "Stage");
        assert_eq!(model.sections[0].title, "Files");
        assert_eq!(model.sections[0].actions.len(), 3);
        assert!(model.sections.iter().all(|section| section
            .actions
            .iter()
            .all(|action| action.enabled)));
        assert!(model.sections.iter().all(|section| section
            .actions
            .iter()
            .all(|action| action.label != "Commit staged changes")));
    }

    #[test]
    fn clean_repo_model_hides_transient_only_preview_actions() {
        let mut state = test_state();
        state.current_branch = Some("main".to_owned());
        state.focused_sidebar_target = Some(SidebarTarget::Root);

        let model = build_footer_model(&state);

        assert_eq!(model.target_label, "repository");
        assert_eq!(model.mode_label, "current: clean repo");
        assert_eq!(model.preview_actions.len(), 1);
        assert_eq!(model.preview_actions[0].label, "Actions");
    }

    #[test]
    fn history_file_model_uses_commit_qualified_target_and_hides_transient_preview() {
        let mut state = test_state();
        state.current_branch = Some("main".to_owned());
        state.sidebar_tab = SidebarTab::History;
        state.history_focus = HistoryFocus::FileList;
        state.history_commit_header = Some(sample_commit());
        state.history_selected_path = Some("src/main.rs".to_owned());

        let model = build_footer_model(&state);

        assert_eq!(model.target_label, "src/main.rs @ abc1234");
        assert_eq!(model.mode_label, "current: history");
        assert_eq!(model.preview_actions.len(), 1);
        assert_eq!(model.preview_actions[0].label, "Actions");
        assert_eq!(model.sections[0].title, "History");
    }

    #[test]
    fn history_diff_without_loaded_diff_hides_search() {
        let mut state = test_state();
        state.current_branch = Some("main".to_owned());
        state.sidebar_tab = SidebarTab::History;
        state.history_focus = HistoryFocus::DiffView;
        state.history_commit_header = Some(sample_commit());
        state.history_selected_path = Some("src/main.rs".to_owned());

        let model = build_footer_model(&state);

        assert!(model.preview_actions.iter().all(|action| action.label != "Search"));
        assert!(model.sections.iter().all(|section| section
            .actions
            .iter()
            .all(|action| action.label != "Search diff")));
    }

    #[test]
    fn history_commit_list_without_loaded_files_hides_enter_action() {
        let mut state = test_state();
        state.current_branch = Some("main".to_owned());
        state.sidebar_tab = SidebarTab::History;
        state.history_focus = HistoryFocus::CommitList;
        state.history_commit_header = Some(sample_commit());
        state.commit_files.clear();

        let model = build_footer_model(&state);

        let history_section = match model.sections.iter().find(|section| section.title == "History") {
            Some(section) => section,
            None => panic!("history section present"),
        };
        assert!(history_section
            .actions
            .iter()
            .all(|action| action.key != "enter"));
    }
}
