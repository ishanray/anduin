use crate::app::{ChangesFocus, CommitComposer, HistoryFocus, Message, SidebarTab, State};
use crate::git::diff::FileStatus;
use crate::views::project_search::view_search_content;
use crate::{MONO, PANEL_HEADER_HEIGHT, lucide};
use iced::theme::palette::Extended;
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, mouse_area, row, rule, scrollable, text, text_input,
};
use iced::{Element, Fill, Length, Theme};

pub(crate) fn view_diff(state: &State) -> Element<'_, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let panel_bg = palette.background.weak.color;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);

    if let Some(search) = state.project_search.as_ref().filter(|s| s.is_open) {
        return view_search_content(state, search);
    }

    let main_content: Element<'_, Message> = if state.sidebar_tab == SidebarTab::History {
        view_history_diff(state, muted_fg)
    } else {
        view_changes_diff(state, muted_fg)
    };

    let content = if let Some(composer) = state.commit_composer.as_ref() {
        column![
            main_content,
            rule::horizontal(1),
            view_commit_composer(state, composer, muted_fg),
        ]
        .height(Fill)
        .into()
    } else {
        main_content
    };

    container(content)
        .width(Fill)
        .height(Fill)
        .style(move |_: &Theme| container::Style::default().background(panel_bg))
        .into()
}

fn view_changes_diff<'a>(state: &'a State, muted_fg: iced::Color) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let diff_focused = state.changes_focus == ChangesFocus::DiffView;
    let focus_color = palette.primary.base.color;

    match &state.current_diff {
        Some(file) => {
            let header_fg = palette.background.base.text;
            let header_bg = palette.background.base.color;

            let header = container(
                row![
                    text(file.path.clone()).size(15).font(MONO).color(header_fg),
                    Space::new().width(Fill),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([12, 20])
            .height(PANEL_HEADER_HEIGHT)
            .width(Fill)
            .style(move |_: &Theme| container::Style::default().background(header_bg));

            let editor = state.diff_editor.view().map(Message::DiffEditor);

            let content = column![
                header,
                rule::horizontal(1),
                container(editor).width(Fill).height(Fill)
            ]
            .height(Fill);

            container(content)
                .width(Fill)
                .height(Fill)
                .style(move |_: &Theme| {
                    container::Style::default().border(iced::Border {
                        color: if diff_focused {
                            focus_color
                        } else {
                            iced::Color::TRANSPARENT
                        },
                        width: if diff_focused { 2.0 } else { 0.0 },
                        radius: 0.0.into(),
                    })
                })
                .into()
        }
        None => container(text("Select a file to view diff").size(16).color(muted_fg))
            .center(Fill)
            .into(),
    }
}

fn view_history_diff<'a>(state: &'a State, muted_fg: iced::Color) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();

    let Some(commit) = state.history_commit_header.as_ref() else {
        return container(
            text("Select a commit to view its changes")
                .size(16)
                .color(muted_fg),
        )
        .center(Fill)
        .into();
    };

    let header_fg = palette.background.base.text;
    let header_bg = palette.background.base.color;
    let border_color = palette.background.strong.color;

    // Commit header: message line
    let message_line = text(commit.message.clone())
        .size(15)
        .font(MONO)
        .color(header_fg)
        .wrapping(Wrapping::None);

    // Commit header: metadata line
    let copy_btn = button(lucide::copy().size(12).color(muted_fg))
        .on_press(Message::CopyCommitHash(commit.hash.clone()))
        .style(button::text)
        .padding(0);

    let meta_line = row![
        lucide::user().size(12).color(muted_fg),
        text(commit.author.clone())
            .size(12)
            .font(MONO)
            .color(muted_fg),
        Space::new().width(12),
        lucide::git_commit_horizontal().size(12).color(muted_fg),
        text(commit.short_hash.clone())
            .size(12)
            .font(MONO)
            .color(muted_fg),
        Space::new().width(4),
        copy_btn,
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    let commit_header = container(column![message_line, meta_line].spacing(4))
        .padding([12, 20])
        .width(Fill)
        .style(move |_: &Theme| container::Style::default().background(header_bg));

    // File list (left pane)
    let file_list = view_history_file_list(state, muted_fg, palette);
    let file_list_focused = state.history_focus == HistoryFocus::FileList;
    let focus_color = palette.primary.base.color;

    let file_list_pane = container(scrollable(file_list).height(Fill))
        .width(Length::Fixed(200.0))
        .height(Fill)
        .style(move |_: &Theme| {
            container::Style::default()
                .background(header_bg)
                .border(iced::Border {
                    color: if file_list_focused {
                        focus_color
                    } else {
                        border_color
                    },
                    width: if file_list_focused { 2.0 } else { 0.0 },
                    radius: 0.0.into(),
                })
        });

    // Right pane: diff editor or empty
    let diff_focused = state.history_focus == HistoryFocus::DiffView;
    let right_pane: Element<'a, Message> = if state.history_diff.is_some() {
        let diff_header_fg = header_fg;
        let diff_header_bg = header_bg;

        // Show file path header + diff editor
        let file_path = state
            .history_selected_file
            .and_then(|i| state.commit_files.get(i))
            .map(|f| f.path.clone())
            .unwrap_or_default();

        let path_header = container(
            row![
                text(file_path).size(15).font(MONO).color(diff_header_fg),
                Space::new().width(Fill),
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([12, 20])
        .height(PANEL_HEADER_HEIGHT)
        .width(Fill)
        .style(move |_: &Theme| container::Style::default().background(diff_header_bg));

        let editor = state.diff_editor.view().map(Message::DiffEditor);

        let diff_content = column![
            path_header,
            rule::horizontal(1),
            container(editor).width(Fill).height(Fill)
        ]
        .width(Fill)
        .height(Fill);

        container(diff_content)
            .width(Fill)
            .height(Fill)
            .style(move |_: &Theme| {
                container::Style::default().border(iced::Border {
                    color: if diff_focused {
                        focus_color
                    } else {
                        iced::Color::TRANSPARENT
                    },
                    width: if diff_focused { 2.0 } else { 0.0 },
                    radius: 0.0.into(),
                })
            })
            .into()
    } else {
        container(
            text("Select a file to view its diff")
                .size(14)
                .color(muted_fg),
        )
        .center(Fill)
        .into()
    };

    // Combine: commit header on top, file list + diff below
    let separator_color = border_color;

    let file_diff_row = row![
        file_list_pane,
        container(Space::new())
            .width(Length::Fixed(1.0))
            .height(Fill)
            .style(move |_: &Theme| container::Style::default().background(separator_color)),
        right_pane,
    ]
    .height(Fill);

    column![commit_header, rule::horizontal(1), file_diff_row,]
        .height(Fill)
        .into()
}

fn view_history_file_list<'a>(
    state: &'a State,
    muted_fg: iced::Color,
    palette: &Extended,
) -> Element<'a, Message> {
    let selected_idx = state.history_selected_file;
    let highlight_bg = palette.primary.weak.color;

    let items: Vec<Element<'a, Message>> = state
        .commit_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let is_selected = selected_idx == Some(i);
            let status = file.status;

            let status_icon: Element<'_, Message> = match status {
                FileStatus::Added | FileStatus::Untracked => lucide::plus()
                    .size(12)
                    .color(palette.success.base.color)
                    .into(),
                FileStatus::Deleted => lucide::minus()
                    .size(12)
                    .color(palette.danger.base.color)
                    .into(),
                FileStatus::Modified => lucide::pencil()
                    .size(12)
                    .color(palette.warning.base.color)
                    .into(),
                FileStatus::Renamed => lucide::arrow_right_left()
                    .size(12)
                    .color(palette.primary.base.color)
                    .into(),
                FileStatus::Other => lucide::circle().size(12).color(muted_fg).into(),
            };

            // Extract just the filename from the path
            let display_name = file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&file.path)
                .to_owned();

            let file_text = text(display_name)
                .size(12)
                .font(MONO)
                .color(if is_selected {
                    palette.primary.strong.text
                } else {
                    palette.background.base.text
                })
                .wrapping(Wrapping::None);

            let row_content = row![status_icon, file_text]
                .spacing(6)
                .align_y(iced::Alignment::Center);

            let bg = if is_selected {
                highlight_bg
            } else {
                iced::Color::TRANSPARENT
            };

            let row_container = container(row_content)
                .padding([4, 10])
                .width(Fill)
                .style(move |_: &Theme| container::Style::default().background(bg));

            mouse_area(row_container)
                .on_press(Message::SelectHistoryFile(i))
                .into()
        })
        .collect();

    column(items).width(Fill).into()
}

fn view_commit_composer<'a>(
    state: &'a State,
    composer: &'a CommitComposer,
    muted_fg: iced::Color,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let base_bg = palette.background.base.color;
    let border_color = palette.background.strong.color;
    let danger_color = palette.danger.base.color;
    let staged_count = state.staged_file_count();
    let unstaged_count = state.unstaged_file_count();

    let helper_text = if staged_count == 0 {
        "No staged changes".to_owned()
    } else if unstaged_count == 0 {
        format!(
            "Committing {staged_count} staged file{}",
            if staged_count == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "Committing {staged_count} staged file{} • {unstaged_count} unstaged excluded",
            if staged_count == 1 { "" } else { "s" }
        )
    };

    let commit_button = if composer.can_submit(staged_count) {
        button(text(if composer.submitting {
            "Committing…"
        } else {
            "Commit"
        }))
        .on_press(Message::SubmitCommit)
        .style(button::primary)
    } else {
        button(text(if composer.submitting {
            "Committing…"
        } else {
            "Commit"
        }))
        .style(button::secondary)
    };

    let error_line: Element<'a, Message> = if let Some(error) = composer.error.as_ref() {
        text(error.as_str())
            .size(12)
            .font(MONO)
            .color(danger_color)
            .into()
    } else {
        text("Cmd/Ctrl+Enter to commit • Esc to cancel")
            .size(11)
            .font(MONO)
            .color(muted_fg)
            .into()
    };

    container(
        column![
            row![
                text("Commit staged changes").size(15).color(fg),
                Space::new().width(Fill),
                button(text("Cancel").size(12).font(MONO))
                    .on_press(Message::CloseCommitComposer)
                    .style(button::text),
            ]
            .align_y(iced::Alignment::Center),
            text(helper_text).size(12).font(MONO).color(muted_fg),
            text_input("Commit summary", &composer.summary)
                .id(composer.input_id.clone())
                .on_input(Message::CommitSummaryChanged)
                .padding([8, 10])
                .size(15)
                .font(MONO),
            error_line,
            row![Space::new().width(Fill), commit_button]
                .spacing(12)
                .align_y(iced::Alignment::Center),
        ]
        .spacing(10),
    )
    .padding([16, 20])
    .style(move |_: &Theme| {
        container::Style::default()
            .background(base_bg)
            .border(iced::Border {
                color: border_color,
                width: 1.0,
                radius: 0.0.into(),
            })
    })
    .into()
}
