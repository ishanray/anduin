use crate::app::{Message, ProjectSearch, State};
use crate::git::diff::FileStatus;
use crate::search::{
    self, ContextLine, ProjectSearchResult, SEARCH_LINE_HEIGHT, find_case_insensitive,
};
use crate::{MONO, PANEL_HEADER_HEIGHT, lucide};
use iced::alignment::Horizontal;
use iced::theme::{Palette, palette::Extended};
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Color, Element, Fill, Theme};

pub(crate) fn view_project_search<'a>(
    state: &'a State,
    search: &'a ProjectSearch,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);
    let panel_bg = palette.background.weak.color;
    let summary = &search.cached_summary;

    let case_button_label = if search.case_sensitive { "Aa" } else { "aa" };
    let case_button = button(text(case_button_label).font(MONO).size(13))
        .on_press(Message::ProjectSearchToggleCase)
        .padding([6, 10])
        .style(if search.case_sensitive {
            button::primary
        } else {
            button::secondary
        });

    let bg_base_color = palette.background.base.color;
    let bg_strong_color = palette.background.strong.color;

    let search_input_box = container(
        row![
            lucide::search().size(18).color(muted_fg),
            text_input("Search all diffs", &search.query)
                .id(search.input_id.clone())
                .on_input(Message::ProjectSearchQueryChanged)
                .padding([6, 8])
                .size(15)
                .font(MONO),
            case_button,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([8, 12])
    .width(400)
    .style(move |_: &Theme| {
        container::Style::default()
            .background(bg_base_color)
            .border(iced::Border {
                color: bg_strong_color,
                width: 1.0,
                radius: 8.0.into(),
            })
            .shadow(iced::Shadow {
                color: iced::Color::BLACK.scale_alpha(0.05),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            })
    });

    let header = container(
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
    .padding([10, 16])
    .height(PANEL_HEADER_HEIGHT)
    .style(move |_: &Theme| {
        container::Style::default()
            .background(bg_base_color)
            .border(iced::Border {
                color: bg_strong_color,
                width: 1.0,
                radius: 0.0.into(),
            })
    });

    let filtered_sidebar = view_search_sidebar(state, search);
    let results = view_project_search_results(state, search, muted_fg);

    container(
        column![
            header,
            row![
                container(filtered_sidebar).width(300),
                container(results).width(Fill)
            ]
            .height(Fill)
        ]
        .height(Fill),
    )
    .height(Fill)
    .style(move |_: &Theme| container::Style::default().background(panel_bg))
    .into()
}

fn view_search_sidebar<'a>(state: &'a State, search: &'a ProjectSearch) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);
    let item_bg = palette.background.base.color;
    let item_hover_bg = palette.background.weak.color;
    let sidebar_border = palette.background.strong.color;

    let body: Element<'a, Message> = if search.query.is_empty() {
        container(
            column![
                lucide::search().size(24).color(muted_fg),
                text("Type to search across changed diffs")
                    .size(13)
                    .color(muted_fg),
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center),
        )
        .center_x(Fill)
        .padding([32, 16])
        .into()
    } else if search.searching {
        container(text("Searching…").size(13).color(muted_fg))
            .center_x(Fill)
            .padding([32, 16])
            .into()
    } else if search.results.is_empty() {
        container(text("No matches in current diffs").size(13).color(muted_fg))
            .center_x(Fill)
            .padding([32, 16])
            .into()
    } else {
        let items: Vec<Element<'a, Message>> = state
            .files
            .iter()
            .filter(|file| search.matching_paths.contains(&file.path))
            .filter_map(|file| {
                let result = &search.results[*search.result_index_by_path.get(&file.path)?];
                let status_color = file_status_color(palette, result.file_status, muted_fg);
                let (dir, name) = split_path_parts(&result.file_path);
                let count_badge = if result.total_matches < 10 {
                    container(
                        text(&result.total_matches_display)
                            .size(11)
                            .font(MONO)
                            .color(status_color)
                            .width(Fill)
                            .align_x(Horizontal::Center)
                            .wrapping(Wrapping::None),
                    )
                    .width(32)
                } else {
                    container(
                        text(&result.total_matches_display)
                            .size(11)
                            .font(MONO)
                            .color(status_color)
                            .wrapping(Wrapping::None),
                    )
                };

                Some(
                    mouse_area(
                        container(
                            row![
                                container(lucide::file().size(14).color(muted_fg)).width(20),
                                container(
                                    column![
                                        text(name)
                                            .size(13)
                                            .font(MONO)
                                            .color(fg)
                                            .width(Fill)
                                            .wrapping(Wrapping::None),
                                        text(dir)
                                            .size(11)
                                            .font(MONO)
                                            .color(muted_fg)
                                            .width(Fill)
                                            .wrapping(Wrapping::None),
                                    ]
                                    .spacing(2)
                                    .width(Fill)
                                    .clip(true),
                                )
                                .width(Fill)
                                .clip(true),
                                count_badge.padding([3, 8]).style(move |_: &Theme| {
                                    container::Style::default()
                                        .background(status_color.scale_alpha(0.12))
                                        .border(iced::Border {
                                            color: status_color.scale_alpha(0.25),
                                            width: 1.0,
                                            radius: 10.0.into(),
                                        })
                                }),
                            ]
                            .spacing(8)
                            .align_y(iced::Alignment::Center)
                            .clip(true),
                        )
                        .width(Fill)
                        .padding([10, 14])
                        .style(move |_: &Theme| {
                            container::Style::default()
                                .background(item_bg)
                                .border(iced::Border {
                                    color: Color::TRANSPARENT,
                                    width: 0.0,
                                    radius: 8.0.into(),
                                })
                        }),
                    )
                    .on_press(Message::ProjectSearchScrollToFile(result.file_path.clone()))
                    .on_enter(Message::ProjectSearchScrollToFile(result.file_path.clone()))
                    .into(),
                )
            })
            .collect();

        scrollable(column(items).spacing(2).padding([6, 8]))
            .height(Fill)
            .into()
    };

    let file_count_text = &search.cached_file_summary;

    container(
        column![
            container(
                row![
                    text("Files").size(12).font(MONO).color(muted_fg),
                    Space::new().width(Fill),
                    text(file_count_text).size(12).font(MONO).color(fg),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([12, 16]),
            body,
        ]
        .height(Fill),
    )
    .style(move |_: &Theme| {
        container::Style::default()
            .background(item_hover_bg)
            .border(iced::Border {
                color: sidebar_border,
                width: 1.0,
                radius: 0.0.into(),
            })
    })
    .into()
}

fn view_project_search_results<'a>(
    state: &'a State,
    search: &'a ProjectSearch,
    muted_fg: Color,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let panel_bg = palette.background.base.color;
    let fg = palette.background.base.text;

    let content: Element<'a, Message> = if search.query.is_empty() {
        container(
            column![
                lucide::search().size(32).color(muted_fg),
                text("Search all changed diffs").size(16).color(muted_fg),
                text("Use ⌘⇧F to open search")
                    .size(12)
                    .font(MONO)
                    .color(muted_fg),
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center),
        )
        .center(Fill)
        .into()
    } else if search.searching {
        container(text("Searching diffs…").size(16).color(muted_fg))
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
    } else {
        let sections: Vec<Element<'a, Message>> = search
            .results
            .iter()
            .map(|result| view_search_result_file(state, search, result))
            .collect();

        let summary_bar = container(
            row![
                text("Results").size(12).font(MONO).color(muted_fg),
                Space::new().width(Fill),
                text(&search.cached_result_summary)
                    .size(12)
                    .font(MONO)
                    .color(fg),
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([12, 20]);

        column![
            summary_bar,
            scrollable(column(sections).spacing(16).padding([8, 20]))
                .id(search.results_scroll_id.clone())
                .height(Fill)
        ]
        .height(Fill)
        .into()
    };

    container(content)
        .height(Fill)
        .style(move |_: &Theme| container::Style::default().background(panel_bg))
        .into()
}

fn view_search_result_file<'a>(
    state: &'a State,
    search: &'a ProjectSearch,
    result: &'a ProjectSearchResult,
) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);
    let card_bg = palette.background.weak.color;
    let card_border = palette.background.strong.color;
    let status_color = file_status_color(palette, result.file_status, muted_fg);
    let (dir, name) = split_path_parts(&result.file_path);

    let header = container(
        row![
            container(lucide::file().size(18).color(muted_fg)).width(24),
            column![
                text(name).size(15).font(MONO).color(fg),
                text(dir).size(12).font(MONO).color(muted_fg),
            ]
            .spacing(2),
            Space::new().width(Fill),
            container(
                text(format!("{} matches", result.total_matches))
                    .size(11)
                    .font(MONO)
                    .color(status_color),
            )
            .padding([4, 10])
            .style(move |_: &Theme| {
                container::Style::default()
                    .background(status_color.scale_alpha(0.12))
                    .border(iced::Border {
                        color: status_color.scale_alpha(0.3),
                        width: 1.0,
                        radius: 12.0.into(),
                    })
            }),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([12, 16]);

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

    container(
        column![
            header,
            container(column(block_elements).spacing(8)).padding([0, 12])
        ]
        .spacing(4),
    )
    .style(move |_: &Theme| {
        container::Style::default()
            .background(card_bg)
            .border(iced::Border {
                color: card_border,
                width: 1.0,
                radius: 12.0.into(),
            })
            .shadow(iced::Shadow {
                color: iced::Color::BLACK.scale_alpha(0.04),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            })
    })
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
        .map(|line| view_search_context_line(state, search, result, line))
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
    result: &'a ProjectSearchResult,
    line: &'a ContextLine,
) -> Element<'a, Message> {
    let _ = result;
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
    .height(SEARCH_LINE_HEIGHT)
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
        find_case_insensitive(line, query_lower)
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

impl FileStatusPalette for Palette {
    fn success_color(&self) -> Color {
        self.success
    }

    fn danger_color(&self) -> Color {
        self.danger
    }

    fn warning_color(&self) -> Color {
        self.primary
    }

    fn primary_color(&self) -> Color {
        self.primary
    }
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
