use crate::app::{Message, State, StatusTone};
use crate::git::diff::FileStatus;
use crate::tree::SidebarRow;
use crate::{MONO, PANEL_HEADER_HEIGHT, SIDEBAR_ROW_HEIGHT, TREE_INDENT, lucide};
use iced::theme::palette::Extended;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, mouse_area, row, rule, scrollable, text, text_input};
use iced::{Element, Fill, Theme};

pub(crate) fn view_sidebar(state: &State) -> Element<'_, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);

    let theme_icon = if state.theme_mode.is_dark() {
        lucide::sun().size(16).color(fg)
    } else {
        lucide::moon().size(16).color(fg)
    };

    let branch_display = state
        .current_branch
        .as_deref()
        .unwrap_or("Anduin");

    let branch_label = button(
        row![
            lucide::git_branch().size(14).color(fg),
            text(branch_display).size(14).color(fg),
            lucide::chevron_down().size(12).color(muted_fg),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::OpenBranchPicker)
    .style(button::text)
    .padding([4, 8]);

    let header = container(
        row![
            branch_label,
            Space::new().width(Fill),
            button(lucide::plus().size(16).color(fg))
                .on_press(Message::OpenRepo)
                .style(button::text)
                .padding([4, 8]),
            button(theme_icon)
                .on_press(Message::ToggleTheme)
                .style(button::text)
                .padding([4, 8]),
            button(lucide::settings().size(16).color(fg))
                .style(button::text)
                .padding([4, 8]),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([12, 16])
    .height(PANEL_HEADER_HEIGHT);

    let file_list: Element<'_, Message> = if state.files.is_empty() {
        container(text("No changes").size(14).color(muted_fg))
            .padding([8, 16])
            .height(Fill)
            .into()
    } else {
        let items: Vec<Element<'_, Message>> = state
            .visible_cached_rows()
            .into_iter()
            .map(|row_data| {
                let target = state.sidebar_target_for_row(row_data);
                let is_focused = state.focused_sidebar_target.as_ref() == Some(&target);
                let is_range_selected = state.is_sidebar_target_selected(&target);
                let item_bg = if is_focused {
                    palette.primary.weak.color
                } else if is_range_selected {
                    palette.primary.weak.color.scale_alpha(0.45)
                } else {
                    palette.background.weakest.color
                };
                let item_fg = if is_focused {
                    palette.primary.weak.text
                } else {
                    palette.background.weakest.text
                };
                let stage_indicator: Element<'_, Message> =
                    if state.sidebar_target_is_fully_staged(&target) {
                        lucide::circle()
                            .size(10)
                            .color(palette.success.base.color)
                            .into()
                    } else {
                        text(" ").size(10).into()
                    };

                match row_data {
                    SidebarRow::Root { name, expanded } => {
                        let chevron_icon: Element<'_, Message> = if *expanded {
                            lucide::chevron_down().size(14).color(muted_fg).into()
                        } else {
                            lucide::chevron_right().size(14).color(muted_fg).into()
                        };
                        let folder_el: Element<'_, Message> = if *expanded {
                            lucide::folder_open()
                                .size(14)
                                .color(palette.primary.base.color)
                                .into()
                        } else {
                            lucide::folder()
                                .size(14)
                                .color(palette.primary.base.color)
                                .into()
                        };
                        let recursive = state.alt_pressed;

                        mouse_area(
                            container(
                                row![
                                    container(chevron_icon).width(16),
                                    container(folder_el).width(20),
                                    container(stage_indicator).width(12),
                                    text(name.as_str()).size(13).font(MONO).color(item_fg),
                                ]
                                .spacing(6)
                                .align_y(iced::Alignment::Center),
                            )
                            .width(Fill)
                            .padding([8, 12])
                            .style(move |_: &Theme| {
                                container::Style::default().background(item_bg)
                            }),
                        )
                        .on_press(Message::ToggleRoot(recursive))
                        .into()
                    }
                    SidebarRow::Dir {
                        name,
                        path,
                        depth,
                        expanded,
                    } => {
                        let chevron_icon: Element<'_, Message> = if *expanded {
                            lucide::chevron_down().size(14).color(muted_fg).into()
                        } else {
                            lucide::chevron_right().size(14).color(muted_fg).into()
                        };
                        let folder_el: Element<'_, Message> = if *expanded {
                            lucide::folder_open()
                                .size(14)
                                .color(palette.primary.base.color)
                                .into()
                        } else {
                            lucide::folder()
                                .size(14)
                                .color(palette.primary.base.color)
                                .into()
                        };
                        let recursive = state.alt_pressed;
                        let depth = *depth;

                        mouse_area(
                            container(
                                row![
                                    Space::new().width((depth as f32) * TREE_INDENT),
                                    container(chevron_icon).width(16),
                                    container(folder_el).width(20),
                                    container(stage_indicator).width(12),
                                    text(name.as_str()).size(13).font(MONO).color(item_fg),
                                ]
                                .spacing(6)
                                .align_y(iced::Alignment::Center),
                            )
                            .width(Fill)
                            .padding([8, 12])
                            .style(move |_: &Theme| {
                                container::Style::default().background(item_bg)
                            }),
                        )
                        .on_press(Message::ToggleDir(path.clone(), recursive))
                        .into()
                    }
                    SidebarRow::File {
                        name,
                        index,
                        depth,
                        status,
                    } => {
                        let index = *index;
                        let depth = *depth;
                        let status = *status;

                        let status_icon: Element<'_, Message> = match status {
                            FileStatus::Added | FileStatus::Untracked => lucide::plus()
                                .size(14)
                                .color(palette.success.base.color)
                                .into(),
                            FileStatus::Deleted => lucide::minus()
                                .size(14)
                                .color(palette.danger.base.color)
                                .into(),
                            FileStatus::Modified => lucide::pencil()
                                .size(14)
                                .color(palette.warning.base.color)
                                .into(),
                            FileStatus::Renamed => lucide::arrow_right_left()
                                .size(14)
                                .color(palette.primary.base.color)
                                .into(),
                            FileStatus::Other => lucide::circle().size(14).color(muted_fg).into(),
                        };

                        let match_badge: Element<'_, Message> = state
                            .project_search
                            .as_ref()
                            .filter(|s| s.is_open)
                            .and_then(|search| {
                                let file = state.files.get(index)?;
                                let result_idx =
                                    search.result_index_by_path.get(&file.path)?;
                                search.results.get(*result_idx)
                            })
                            .map(|result| {
                                let badge_color = palette.warning.base.color;
                                container(
                                    text(&result.total_matches_display)
                                        .size(10)
                                        .font(MONO)
                                        .color(badge_color)
                                        .wrapping(Wrapping::None),
                                )
                                .padding([1, 6])
                                .style(move |_: &Theme| {
                                    container::Style::default()
                                        .background(badge_color.scale_alpha(0.12))
                                        .border(iced::Border {
                                            color: badge_color.scale_alpha(0.25),
                                            width: 1.0,
                                            radius: 8.0.into(),
                                        })
                                })
                                .into()
                            })
                            .unwrap_or_else(|| Space::new().width(0).into());

                        mouse_area(
                            container(
                                row![
                                    Space::new().width((depth as f32) * TREE_INDENT),
                                    container(lucide::file().size(14).color(muted_fg)).width(16),
                                    container(status_icon).width(16),
                                    container(stage_indicator).width(12),
                                    container(
                                        text(name.as_str())
                                            .size(13)
                                            .font(MONO)
                                            .color(item_fg)
                                            .wrapping(Wrapping::None),
                                    )
                                    .width(Fill)
                                    .clip(true),
                                    match_badge,
                                ]
                                .spacing(8)
                                .align_y(iced::Alignment::Center),
                            )
                            .width(Fill)
                            .padding([8, 12])
                            .style(move |_: &Theme| {
                                container::Style::default().background(item_bg)
                            }),
                        )
                        .on_press(Message::SelectFile(index))
                        .into()
                    }
                }
            })
            .collect();

        scrollable(column(items).spacing(2).padding([8, 8]))
            .id(state.sidebar_scroll_id.clone())
            .on_scroll(|viewport| {
                Message::SidebarScrolled(viewport.absolute_offset().y, viewport.bounds().height)
            })
            .height(Fill)
            .into()
    };

    let selected_suffix = if state.selected_file_count() > 1 {
        format!(" • {} selected", state.selected_file_count())
    } else {
        String::new()
    };
    let summary = format!(
        "{} changed • {} staged{}",
        state.files.len(),
        state.staged_file_count(),
        selected_suffix
    );

    let mut footer_content = column![text(summary).size(12).font(MONO).color(fg)].spacing(6);

    if let Some(status) = state.status_message.as_ref() {
        let color = status_color(palette, status.tone);
        footer_content =
            footer_content.push(text(status.text.as_str()).size(12).font(MONO).color(color));
    }

    let footer = container(footer_content).padding([10, 14]);

    let sidebar_bg = palette.background.base.color;
    let sidebar_border = palette.background.strong.color;

    let mut sidebar_column = column![header, rule::horizontal(1)].height(Fill);

    if state.is_branch_picker_open() {
        sidebar_column = sidebar_column.push(view_branch_picker(state));
        sidebar_column = sidebar_column.push(rule::horizontal(1));
    }

    sidebar_column = sidebar_column.push(file_list);
    sidebar_column = sidebar_column.push(rule::horizontal(1));
    sidebar_column = sidebar_column.push(footer);

    container(sidebar_column)
    .style(move |_theme: &Theme| {
        container::Style::default()
            .background(sidebar_bg)
            .border(iced::Border {
                color: sidebar_border,
                width: 0.0,
                radius: 0.into(),
            })
    })
    .height(Fill)
    .into()
}

fn view_branch_picker<'a>(state: &'a State) -> Element<'a, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let bg = palette.background.base.color;
    let border_color = palette.background.base.text.scale_alpha(0.15);
    let hover_bg = palette.primary.weak.color;
    let hover_fg = palette.primary.weak.text;
    let success_color = palette.success.base.color;
    let danger_color = palette.danger.base.color;
    let empty_color = palette.background.strong.text.scale_alpha(0.6);
    let danger_bg = palette.danger.base.color.scale_alpha(0.1);

    let picker = state.branch_picker.as_ref().expect("picker must be Some");

    let input = text_input("Filter branches…", &picker.filter)
        .on_input(Message::BranchPickerFilterChanged)
        .id(picker.input_id.clone())
        .size(13)
        .padding([8, 12]);

    let filtered = picker.filtered_branches();

    let branch_items: Vec<Element<'a, Message>> = filtered
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let is_current = *branch == picker.current;
            let is_selected = i == picker.selected_index;
            let item_bg = if is_selected { hover_bg } else { bg };
            let item_fg = if is_selected { hover_fg } else { fg };
            let branch_owned = branch.to_string();

            let mut row_content = row![].spacing(8).align_y(iced::Alignment::Center);

            if is_current {
                row_content = row_content.push(
                    lucide::check().size(12).color(success_color),
                );
            } else {
                row_content = row_content.push(Space::new().width(12));
            }

            row_content = row_content.push(
                text(branch_owned.clone()).size(13).font(MONO).color(item_fg),
            );

            mouse_area(
                container(row_content)
                    .width(Fill)
                    .padding([6, 12])
                    .style(move |_: &Theme| {
                        container::Style::default().background(item_bg)
                    }),
            )
            .on_press(Message::SwitchBranch(branch_owned))
            .into()
        })
        .collect();

    let branch_list: Element<'a, Message> = if branch_items.is_empty() {
        container(
            text("No matching branches")
                .size(12)
                .color(empty_color),
        )
        .padding([8, 12])
        .into()
    } else {
        scrollable(column(branch_items).spacing(2))
            .height(iced::Length::Shrink)
            .into()
    };

    let mut content = column![input, rule::horizontal(1), branch_list].spacing(0);

    if let Some(error) = picker.error.as_ref() {
        content = content.push(
            container(
                text(error.as_str())
                    .size(12)
                    .font(MONO)
                    .color(danger_color),
            )
            .padding([8, 12])
            .width(Fill)
            .style(move |_: &Theme| {
                container::Style::default()
                    .background(danger_bg)
            }),
        );
    }

    container(content)
        .width(Fill)
        .max_height(300.0)
        .style(move |_: &Theme| {
            container::Style::default().background(bg).border(iced::Border {
                color: border_color,
                width: 1.0,
                radius: 8.0.into(),
            })
        })
        .into()
}

fn status_color(palette: &Extended, tone: StatusTone) -> iced::Color {
    match tone {
        StatusTone::Success => palette.success.base.color,
        StatusTone::Error => palette.danger.base.color,
    }
}

pub(crate) fn selected_sidebar_row_bounds(state: &State) -> Option<(f32, f32)> {
    let row_index = state.focused_sidebar_row_index()?;
    let top = 8.0 + (row_index as f32) * SIDEBAR_ROW_HEIGHT;
    let bottom = top + SIDEBAR_ROW_HEIGHT;
    Some((top, bottom))
}
