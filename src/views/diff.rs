use crate::app::{CommitComposer, Message, State};
use crate::{MONO, PANEL_HEADER_HEIGHT};
use iced::widget::{
    Space, button, column, container, row, rule, text, text_input,
};
use iced::{Element, Fill, Theme};

pub(crate) fn view_diff(state: &State) -> Element<'_, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);

    let main_content: Element<'_, Message> = match &state.current_diff {
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

            column![
                header,
                rule::horizontal(1),
                container(editor).width(Fill).height(Fill)
            ]
            .height(Fill)
            .into()
        }
        None => container(text("Select a file to view diff").size(16).color(muted_fg))
            .center(Fill)
            .into(),
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

    let panel_bg = palette.background.weak.color;

    container(content)
        .width(Fill)
        .height(Fill)
        .style(move |_: &Theme| container::Style::default().background(panel_bg))
        .into()
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
        format!("Committing {staged_count} staged file{}", if staged_count == 1 { "" } else { "s" })
    } else {
        format!(
            "Committing {staged_count} staged file{} • {unstaged_count} unstaged excluded",
            if staged_count == 1 { "" } else { "s" }
        )
    };

    let commit_button = if composer.can_submit(staged_count) {
        button(text(if composer.submitting { "Committing…" } else { "Commit" }))
            .on_press(Message::SubmitCommit)
            .style(button::primary)
    } else {
        button(text(if composer.submitting { "Committing…" } else { "Commit" }))
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
