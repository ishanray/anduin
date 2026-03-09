use crate::app::{Message, ProjectSearch, State};
use crate::git::diff::FileStatus;
use crate::search::{self, ContextLine, ProjectSearchResult};
use crate::{MONO, PANEL_HEADER_HEIGHT, lucide};
use iced::theme::palette::Extended;
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, mouse_area, row, rule, scrollable, text, text_input,
};
use iced::{Color, Element, Fill, Theme};

/// Content area when project search is active.
/// Renders the search header + results for the currently selected file.
pub(crate) fn view_search_content<'a>(
    state: &'a State,
    search: &'a ProjectSearch,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let panel_bg = palette.background.weak.color;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);

    let header = view_search_header(state, search);

    let body: Element<'a, Message> = if search.query.is_empty() {
        container(
            column![
                lucide::search().size(32).color(muted_fg),
                text("Search across changed diffs")
                    .size(16)
                    .color(muted_fg),
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center),
        )
        .center(Fill)
        .into()
    } else if search.searching {
        container(text("Searching…").size(16).color(muted_fg))
            .center(Fill)
            .into()
    } else if search.results.is_empty() {
        container(
            column![
                text("No matches found").size(16).color(muted_fg),
                text(format!("for {:?}", search.query))
                    .size(12)
                    .font(MONO)
                    .color(muted_fg),
            ]
            .spacing(6)
            .align_x(iced::Alignment::Center),
        )
        .center(Fill)
        .into()
    } else if let Some(selected_path) = &state.selected_path {
        if let Some(&idx) = search.result_index_by_path.get(selected_path) {
            let result = &search.results[idx];
            view_file_search_results(state, search, result)
        } else {
            container(
                column![
                    text("No matches in this file")
                        .size(14)
                        .color(muted_fg),
                    text("Select a file with matches from the sidebar")
                        .size(12)
                        .font(MONO)
                        .color(muted_fg),
                ]
                .spacing(6)
                .align_x(iced::Alignment::Center),
            )
            .center(Fill)
            .into()
        }
    } else {
        container(
            text("Select a file to view matches")
                .size(14)
                .color(muted_fg),
        )
        .center(Fill)
        .into()
    };

    container(column![header, rule::horizontal(1), body].height(Fill))
        .width(Fill)
        .height(Fill)
        .style(move |_: &Theme| container::Style::default().background(panel_bg))
        .into()
}

fn view_search_header<'a>(state: &'a State, search: &'a ProjectSearch) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);
    let bg_base_color = palette.background.base.color;
    let bg_strong_color = palette.background.strong.color;
    let input_value_color = palette.background.base.text;
    let input_placeholder_color = muted_fg;
    let input_selection_color = palette.primary.weak.color;
    let input_icon_color = muted_fg;
    let summary = &search.cached_summary;

    let input_style = move |_theme: &Theme, _status: text_input::Status| text_input::Style {
        background: iced::Background::Color(Color::TRANSPARENT),
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: input_icon_color,
        placeholder: input_placeholder_color,
        value: input_value_color,
        selection: input_selection_color,
    };

    let case_button_label = if search.case_sensitive { "Aa" } else { "aa" };
    let case_button_bg = if search.case_sensitive {
        palette.primary.base.color
    } else {
        palette.background.strong.color
    };
    let case_button_bg_hover = if search.case_sensitive {
        palette.primary.strong.color
    } else {
        palette.background.strong.color.scale_alpha(0.9)
    };
    let case_button_fg = if search.case_sensitive {
        palette.primary.base.text
    } else {
        fg
    };
    let case_button_border = if search.case_sensitive {
        palette.primary.strong.color
    } else {
        palette.background.base.text.scale_alpha(0.12)
    };

    let case_button = button(
        text(case_button_label)
            .size(13)
            .font(MONO)
            .color(case_button_fg),
    )
    .on_press(Message::ProjectSearchToggleCase)
    .padding([4, 10])
    .style(move |_theme: &Theme, status: button::Status| {
        let background = match status {
            button::Status::Hovered => case_button_bg_hover,
            _ => case_button_bg,
        };
        button::Style {
            background: Some(iced::Background::Color(background)),
            text_color: case_button_fg,
            border: iced::Border {
                color: case_button_border,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    });

    let search_input_box = container(
        row![
            lucide::search().size(16).color(muted_fg),
            text_input("Search all diffs", &search.query)
                .id(search.input_id.clone())
                .on_input(Message::ProjectSearchQueryChanged)
                .padding([4, 8])
                .size(15)
                .width(Fill)
                .style(input_style),
            case_button,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([0, 12])
    .height(36)
    .center_y(36)
    .width(400)
    .style(move |_: &Theme| {
        container::Style::default()
            .background(bg_base_color)
            .border(iced::Border {
                color: bg_strong_color,
                width: 1.0,
                radius: 8.0.into(),
            })
    });

    container(
        row![
            search_input_box,
            text(summary).size(12).font(MONO).color(muted_fg),
            Space::new().width(Fill),
            button(lucide::x().size(20).color(fg))
                .on_press(Message::CloseProjectSearch)
                .style(button::text)
                .padding([6, 8]),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .padding([0, 16])
    .height(PANEL_HEADER_HEIGHT)
    .center_y(PANEL_HEADER_HEIGHT)
    .style(move |_: &Theme| container::Style::default().background(bg_base_color))
    .into()
}

fn view_file_search_results<'a>(
    state: &'a State,
    search: &'a ProjectSearch,
    result: &'a ProjectSearchResult,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);
    let status_color = file_status_color(palette, result.file_status, muted_fg);
    let (dir, name) = split_path_parts(&result.file_path);

    let file_header = container(
        row![
            container(lucide::file().size(14).color(muted_fg)).width(20),
            text(name).size(13).font(MONO).color(fg),
            text(format!(" {dir}")).size(11).font(MONO).color(muted_fg),
            Space::new().width(Fill),
            text(format!(
                "{} match{}",
                result.total_matches,
                if result.total_matches == 1 { "" } else { "es" }
            ))
            .size(11)
            .font(MONO)
            .color(status_color),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .padding([8, 4]);

    let mut block_elements: Vec<Element<'a, Message>> = Vec::new();
    for (index, context) in result.matches.iter().enumerate() {
        if index > 0 {
            let previous = &result.matches[index - 1];
            let omitted = context.start_line.saturating_sub(previous.end_line + 1);
            if omitted > 0 {
                block_elements.push(
                    container(
                        row![
                            lucide::ellipsis().size(14).color(muted_fg),
                            text(name).size(11).font(MONO).color(muted_fg),
                        ]
                        .align_y(iced::Alignment::Center),
                    )
                    .padding([2, 16])
                    .into(),
                );
            }
        }

        block_elements.push(view_search_match_context(state, search, result, context));
    }

    scrollable(
        column![file_header, column(block_elements).spacing(8)]
            .spacing(2)
            .padding([8, 20]),
    )
    .height(Fill)
    .into()
}

fn view_search_match_context<'a>(
    state: &'a State,
    search: &'a ProjectSearch,
    result: &'a ProjectSearchResult,
    context: &'a search::MatchContext,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let block_bg = palette.background.base.color;
    let border_color = palette.background.strong.color;

    let rows: Vec<Element<'a, Message>> = context
        .lines
        .iter()
        .map(|line| view_search_context_line(state, search, line))
        .collect();

    mouse_area(
        container(column(rows).spacing(2))
            .padding([8, 10])
            .style(move |_: &Theme| {
                container::Style::default()
                    .background(block_bg)
                    .border(iced::Border {
                        color: border_color,
                        width: 1.0,
                        radius: 8.0.into(),
                    })
            }),
    )
    .on_press(Message::ProjectSearchJumpTo(
        result.file_path.clone(),
        context
            .lines
            .iter()
            .find(|line| line.is_match)
            .map(|line| line.line_number)
            .unwrap_or(context.start_line),
    ))
    .into()
}

fn view_search_context_line<'a>(
    state: &'a State,
    search: &'a ProjectSearch,
    line: &'a ContextLine,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);
    let match_row_bg = if palette.is_dark {
        palette.primary.base.color.scale_alpha(0.14)
    } else {
        palette.primary.base.color.scale_alpha(0.08)
    };
    let match_token_bg = if palette.is_dark {
        palette.primary.base.color.scale_alpha(0.32)
    } else {
        palette.primary.base.color.scale_alpha(0.18)
    };
    let match_token_border = if palette.is_dark {
        palette.primary.base.color.scale_alpha(0.75)
    } else {
        palette.primary.base.color.scale_alpha(0.35)
    };
    let match_fg = fg;
    let prefix_color = diff_prefix_color(palette, line.text.chars().next(), muted_fg);
    let (prefix, rest) = split_diff_prefix(&line.text);
    let query = &search.query;

    let text_row: Element<'a, Message> = if line.is_match && !query.is_empty() {
        let (before, matched, after) =
            split_first_match(rest, query, &search.query_lower, search.case_sensitive);
        row![
            text(prefix)
                .size(13)
                .font(MONO)
                .color(prefix_color)
                .wrapping(Wrapping::None),
            text(before)
                .size(13)
                .font(MONO)
                .color(fg)
                .wrapping(Wrapping::None),
            container(
                text(matched)
                    .size(13)
                    .font(MONO)
                    .color(match_fg)
                    .wrapping(Wrapping::None),
            )
            .style(move |_: &Theme| {
                container::Style::default()
                    .background(match_token_bg)
                    .border(iced::Border {
                        color: match_token_border,
                        width: 1.0,
                        radius: 3.0.into(),
                    })
            }),
            text(after)
                .size(13)
                .font(MONO)
                .color(fg)
                .wrapping(Wrapping::None),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        row![
            text(prefix)
                .size(13)
                .font(MONO)
                .color(prefix_color)
                .wrapping(Wrapping::None),
            text(rest)
                .size(13)
                .font(MONO)
                .color(fg)
                .wrapping(Wrapping::None),
        ]
        .into()
    };

    container(
        row![
            text(&line.line_number_display)
                .size(12)
                .font(MONO)
                .color(muted_fg)
                .wrapping(Wrapping::None)
                .width(48),
            text_row,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .height(search::SEARCH_LINE_HEIGHT)
    .clip(true)
    .style(move |_: &Theme| {
        let background = if line.is_match {
            match_row_bg
        } else {
            Color::TRANSPARENT
        };
        container::Style::default().background(background)
    })
    .into()
}

fn split_path_parts(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some((dir, name)) => (dir, name),
        None => ("(repo root)", path),
    }
}

fn split_diff_prefix(line: &str) -> (&str, &str) {
    match line.chars().next() {
        Some(prefix @ ('+' | '-' | '@' | ' ')) => {
            let prefix_len = prefix.len_utf8();
            (&line[..prefix_len], &line[prefix_len..])
        }
        _ => ("", line),
    }
}

fn diff_prefix_color(palette: &Extended, prefix: Option<char>, muted_fg: Color) -> Color {
    match prefix {
        Some('+') => palette.success.base.color,
        Some('-') => palette.danger.base.color,
        Some('@') => palette.primary.base.color,
        Some(' ') => muted_fg,
        _ => muted_fg,
    }
}

pub(crate) fn split_first_match<'a>(
    line: &'a str,
    query: &str,
    query_lower: &str,
    case_sensitive: bool,
) -> (&'a str, &'a str, &'a str) {
    if query.is_empty() {
        return (line, "", "");
    }

    let start = if case_sensitive {
        line.find(query)
    } else {
        search::find_case_insensitive(line, query_lower)
    };

    if let Some(start) = start {
        let end = start + query.len();
        if let (Some(before), Some(matched), Some(after)) =
            (line.get(..start), line.get(start..end), line.get(end..))
        {
            return (before, matched, after);
        }
    }

    (line, "", "")
}

fn file_status_color(
    palette: &impl FileStatusPalette,
    status: FileStatus,
    muted_fg: Color,
) -> Color {
    match status {
        FileStatus::Added | FileStatus::Untracked => palette.success_color(),
        FileStatus::Deleted => palette.danger_color(),
        FileStatus::Modified => palette.warning_color(),
        FileStatus::Renamed => palette.primary_color(),
        FileStatus::Other => muted_fg,
    }
}

trait FileStatusPalette {
    fn success_color(&self) -> Color;
    fn danger_color(&self) -> Color;
    fn warning_color(&self) -> Color;
    fn primary_color(&self) -> Color;
}

impl FileStatusPalette for Extended {
    fn success_color(&self) -> Color {
        self.success.base.color
    }

    fn danger_color(&self) -> Color {
        self.danger.base.color
    }

    fn warning_color(&self) -> Color {
        self.warning.base.color
    }

    fn primary_color(&self) -> Color {
        self.primary.base.color
    }
}

#[cfg(test)]
mod tests {
    use super::split_first_match;

    #[test]
    fn split_first_match_case_sensitive() {
        assert_eq!(
            split_first_match("alpha beta gamma", "beta", "beta", true),
            ("alpha ", "beta", " gamma")
        );
    }

    #[test]
    fn split_first_match_case_insensitive() {
        assert_eq!(
            split_first_match("alpha BETA gamma", "beta", "beta", false),
            ("alpha ", "BETA", " gamma")
        );
    }

    #[test]
    fn split_first_match_handles_missing_query() {
        assert_eq!(
            split_first_match("alpha", "zzz", "zzz", false),
            ("alpha", "", "")
        );
    }
}
