use crate::app::{HistoryFocus, Message, SidebarTab, State};
use crate::git::diff::FileStatus;
use crate::tree::SidebarRow;
use crate::views::context_menu::context_menu_area;
use crate::{MONO, PANEL_HEADER_HEIGHT, SIDEBAR_ROW_HEIGHT, TREE_INDENT, lucide};
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, Stack, button, column, container, mouse_area, row, rule, scrollable, text, text_input,
};
use iced::{Element, Fill, Length, Theme};

pub(crate) const PICKER_ROW_HEIGHT: f32 = 32.0;

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

    let branch_display = state.current_branch.as_deref().unwrap_or("Anduin");

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

                        let chevron_hit: Element<'_, Message> = mouse_area(
                            container(chevron_icon)
                                .width(24)
                                .center_y(Fill)
                                .padding([0, 4]),
                        )
                        .on_press(Message::ToggleRoot(recursive))
                        .into();

                        mouse_area(
                            container(
                                row![
                                    chevron_hit,
                                    container(folder_el).width(20),
                                    container(stage_indicator).width(12),
                                    text(name.as_str()).size(13).font(MONO).color(item_fg),
                                ]
                                .spacing(6)
                                .align_y(iced::Alignment::Center),
                            )
                            .width(Fill)
                            .padding([3, 8])
                            .style(move |_: &Theme| {
                                container::Style::default().background(item_bg)
                            }),
                        )
                        .on_press(Message::FocusRoot)
                        .on_double_click(Message::OpenInEditor(".".to_owned()))
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

                        let chevron_hit: Element<'_, Message> = mouse_area(
                            container(chevron_icon)
                                .width(24)
                                .center_y(Fill)
                                .padding([0, 4]),
                        )
                        .on_press(Message::ToggleDir(path.clone(), recursive))
                        .into();

                        let path_for_editor = path.clone();
                        let path_for_context = path.clone();
                        context_menu_area(
                            mouse_area(
                                container(
                                    row![
                                        Space::new().width((depth as f32) * TREE_INDENT),
                                        chevron_hit,
                                        container(folder_el).width(20),
                                        container(stage_indicator).width(12),
                                        text(name.as_str()).size(13).font(MONO).color(item_fg),
                                    ]
                                    .spacing(6)
                                    .align_y(iced::Alignment::Center),
                                )
                                .width(Fill)
                                .padding([3, 8])
                                .style(move |_: &Theme| {
                                    container::Style::default().background(item_bg)
                                }),
                            )
                            .on_press(Message::FocusDir(path.clone()))
                            .on_double_click(Message::OpenInEditor(path_for_editor)),
                            move |bounds| Message::ShowContextMenu {
                                path: path_for_context.clone(),
                                is_dir: true,
                                bounds,
                            },
                        )
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
                                let result_idx = search.result_index_by_path.get(&file.path)?;
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

                        let file_path = state
                            .files
                            .get(index)
                            .map(|f| f.path.clone())
                            .unwrap_or_default();

                        let file_path_for_context = file_path.clone();
                        context_menu_area(
                            mouse_area(
                                container(
                                    row![
                                        Space::new().width((depth as f32) * TREE_INDENT),
                                        container(lucide::file().size(14).color(muted_fg))
                                            .width(16),
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
                                .padding([3, 8])
                                .style(move |_: &Theme| {
                                    container::Style::default().background(item_bg)
                                }),
                            )
                            .on_press(Message::SelectFile(index))
                            .on_double_click(Message::OpenInEditor(file_path)),
                            move |bounds| Message::ShowContextMenu {
                                path: file_path_for_context.clone(),
                                is_dir: false,
                                bounds,
                            },
                        )
                        .into()
                    }
                }
            })
            .collect();

        scrollable(column(items).spacing(1).padding([4, 8]))
            .id(state.sidebar_scroll_id.clone())
            .on_scroll(|viewport| {
                Message::SidebarScrolled(viewport.absolute_offset().y, viewport.bounds().height)
            })
            .height(Fill)
            .into()
    };

    let sidebar_bg = palette.background.base.color;
    let sidebar_border = palette.background.strong.color;

    // Tab bar
    let tab_bar = {
        let primary_color = palette.primary.base.color;
        let changes_active = state.sidebar_tab == SidebarTab::Changes;
        let history_active = state.sidebar_tab == SidebarTab::History;

        let changes_fg = if changes_active {
            primary_color
        } else {
            muted_fg
        };
        let changes_underline = if changes_active {
            primary_color
        } else {
            iced::Color::TRANSPARENT
        };
        let history_fg = if history_active {
            primary_color
        } else {
            muted_fg
        };
        let history_underline = if history_active {
            primary_color
        } else {
            iced::Color::TRANSPARENT
        };

        let changes_tab: Element<'_, Message> = mouse_area(column![
            container(text("Changes").size(13).color(changes_fg)).padding([6, 12]),
            container(Space::new().height(0))
                .width(Fill)
                .height(2)
                .style(move |_: &Theme| {
                    container::Style::default().background(changes_underline)
                }),
        ])
        .on_press(Message::SwitchSidebarTab(SidebarTab::Changes))
        .into();

        let history_tab: Element<'_, Message> = mouse_area(column![
            container(text("History").size(13).color(history_fg)).padding([6, 12]),
            container(Space::new().height(0))
                .width(Fill)
                .height(2)
                .style(move |_: &Theme| {
                    container::Style::default().background(history_underline)
                }),
        ])
        .on_press(Message::SwitchSidebarTab(SidebarTab::History))
        .into();

        container(
            row![changes_tab, history_tab]
                .spacing(0)
                .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([0, 8])
    };

    // Main content: file list or commit list depending on active tab
    let main_list: Element<'_, Message> = match state.sidebar_tab {
        SidebarTab::Changes => file_list,
        SidebarTab::History => view_commit_list(state),
    };

    // Footer varies by tab
    let footer: Element<'_, Message> = match state.sidebar_tab {
        SidebarTab::Changes => {
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

            container(text(summary).size(12).font(MONO).color(fg))
                .padding([10, 14])
                .into()
        }
        SidebarTab::History => {
            let count = state.commits.len();
            let summary = if count == 0 && state.commits_loading {
                "Loading commits…".to_owned()
            } else if state.commits_exhausted {
                format!("{count} commits (all loaded)")
            } else {
                format!("{count} commits loaded")
            };
            container(text(summary).size(12).font(MONO).color(fg))
                .padding([10, 14])
                .into()
        }
    };

    let sidebar_column = column![
        header,
        rule::horizontal(1),
        tab_bar,
        rule::horizontal(1),
        main_list,
        rule::horizontal(1),
        footer,
    ]
    .height(Fill);

    // Build overlay for branch/project picker floating on top
    let has_overlay = state.is_branch_picker_open() || state.is_project_picker_open();

    let base: Element<'_, Message> = container(sidebar_column)
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
        .into();

    if !has_overlay {
        return base;
    }

    // Picker overlay floats below the header (48px + 1px rule)
    let overlay_content: Element<'_, Message> = if state.is_branch_picker_open() {
        view_branch_picker(state)
    } else {
        view_project_picker(state)
    };

    let overlay = container(overlay_content)
        .width(Fill)
        .padding(iced::Padding {
            top: PANEL_HEADER_HEIGHT + 1.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        });

    Stack::new()
        .push(base)
        .push(overlay.height(Length::Shrink))
        .height(Fill)
        .width(Fill)
        .into()
}

fn view_commit_list(state: &State) -> Element<'_, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);

    if state.commits.is_empty() && !state.commits_loading {
        return container(text("No commits").size(14).color(muted_fg))
            .padding([8, 16])
            .height(Fill)
            .into();
    }

    let mut items: Vec<Element<'_, Message>> = state
        .commits
        .iter()
        .enumerate()
        .map(|(i, commit)| {
            let is_selected = state.selected_commit == Some(i);
            let item_bg = if is_selected {
                palette.primary.weak.color
            } else {
                palette.background.weakest.color
            };
            let item_fg = if is_selected {
                palette.primary.weak.text
            } else {
                fg
            };

            let message_line = text(commit.message.as_str())
                .size(13)
                .font(MONO)
                .color(item_fg)
                .wrapping(Wrapping::None);

            let detail = format!("{} · {}", commit.author, commit.date);
            let detail_line = text(detail)
                .size(12)
                .color(muted_fg)
                .wrapping(Wrapping::None);

            mouse_area(
                container(column![message_line, detail_line].spacing(2))
                    .width(Fill)
                    .padding([6, 12])
                    .clip(true)
                    .style(move |_: &Theme| container::Style::default().background(item_bg)),
            )
            .on_press(Message::SelectCommit(i))
            .into()
        })
        .collect();

    // Loading indicator (commits auto-load on scroll)
    if state.commits_loading {
        items.push(
            container(text("Loading…").size(12).font(MONO).color(muted_fg))
                .padding([8, 12])
                .width(Fill)
                .into(),
        );
    }

    let commit_list_focused = state.history_focus == HistoryFocus::CommitList;
    let focus_color = palette.primary.base.color;

    let list = scrollable(column(items).spacing(1).padding([4, 8]))
        .id(state.commit_list_scroll_id.clone())
        .on_scroll(|viewport| {
            Message::CommitListScrolled(
                viewport.absolute_offset().y,
                viewport.bounds().height,
                viewport.content_bounds().height,
            )
        })
        .height(Fill);

    container(list)
        .height(Fill)
        .width(Fill)
        .style(move |_: &Theme| {
            container::Style::default().border(iced::Border {
                color: if commit_list_focused {
                    focus_color
                } else {
                    iced::Color::TRANSPARENT
                },
                width: if commit_list_focused { 2.0 } else { 0.0 },
                radius: 0.0.into(),
            })
        })
        .into()
}

fn view_picker_dropdown<'a>(
    input: Element<'a, Message>,
    list: Element<'a, Message>,
    error: Option<Element<'a, Message>>,
    bg: iced::Color,
    border_color: iced::Color,
) -> Element<'a, Message> {
    let mut content = column![input, rule::horizontal(1), list].spacing(0);

    if let Some(err) = error {
        content = content.push(err);
    }

    container(content)
        .width(Fill)
        .max_height(300.0)
        .style(move |_: &Theme| {
            container::Style::default()
                .background(bg)
                .border(iced::Border {
                    color: border_color,
                    width: 1.0,
                    radius: 8.0.into(),
                })
                .shadow(iced::Shadow {
                    color: iced::Color::BLACK.scale_alpha(0.15),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                })
        })
        .into()
}

fn view_branch_picker(state: &State) -> Element<'_, Message> {
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

    let Some(picker) = state.branch_picker.as_ref() else {
        return text("").into();
    };

    let input = text_input("Filter branches…", &picker.filter)
        .on_input(Message::BranchPickerFilterChanged)
        .id(picker.input_id.clone())
        .size(13)
        .padding([8, 12]);

    let filtered = picker.filtered_branches();

    let show_create = picker.should_show_create();
    let index_offset: usize = if show_create { 1 } else { 0 };

    let mut all_items: Vec<Element<'_, Message>> = Vec::new();

    // "Create <filter>" as first item when applicable
    if show_create {
        let is_selected = picker.selected_index == 0;
        let create_name = picker.filter.clone();
        let create_bg = if is_selected {
            success_color.scale_alpha(0.15)
        } else {
            bg
        };
        let create_fg = success_color;

        let row_content = row![
            lucide::plus().size(12).color(create_fg),
            text("Create ").size(13).color(create_fg),
            text(create_name.clone())
                .size(13)
                .font(MONO)
                .color(create_fg),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        all_items.push(
            mouse_area(
                container(row_content)
                    .width(Fill)
                    .height(PICKER_ROW_HEIGHT)
                    .padding([6, 12])
                    .style(move |_: &Theme| container::Style::default().background(create_bg)),
            )
            .on_press(Message::CreateBranch(create_name))
            .into(),
        );
    }

    // Existing branch items
    let branch_items: Vec<Element<'_, Message>> = filtered
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let is_current = *branch == picker.current;
            let is_selected = (i + index_offset) == picker.selected_index;
            let item_bg = if is_selected { hover_bg } else { bg };
            let item_fg = if is_selected { hover_fg } else { fg };
            let branch_owned = branch.to_string();

            let mut row_content = row![].spacing(8).align_y(iced::Alignment::Center);

            if is_current {
                row_content = row_content.push(lucide::check().size(12).color(success_color));
            } else {
                row_content = row_content.push(Space::new().width(12));
            }

            row_content = row_content.push(
                text(branch_owned.clone())
                    .size(13)
                    .font(MONO)
                    .color(item_fg),
            );

            mouse_area(
                container(row_content)
                    .width(Fill)
                    .height(PICKER_ROW_HEIGHT)
                    .padding([6, 12])
                    .style(move |_: &Theme| container::Style::default().background(item_bg)),
            )
            .on_press(Message::SwitchBranch(branch_owned))
            .into()
        })
        .collect();

    all_items.extend(branch_items);

    let branch_list: Element<'_, Message> = if all_items.is_empty() {
        container(text("No matching branches").size(12).color(empty_color))
            .padding([8, 12])
            .into()
    } else {
        scrollable(column(all_items).spacing(2))
            .id(picker.scroll_id.clone())
            .on_scroll(|viewport| {
                Message::BranchPickerScrolled(
                    viewport.absolute_offset().y,
                    viewport.bounds().height,
                )
            })
            .height(iced::Length::Shrink)
            .into()
    };

    let error_element = picker.error.as_ref().map(|error| {
        container(text(error.as_str()).size(12).font(MONO).color(danger_color))
            .padding([8, 12])
            .width(Fill)
            .style(move |_: &Theme| container::Style::default().background(danger_bg))
            .into()
    });

    view_picker_dropdown(input.into(), branch_list, error_element, bg, border_color)
}

fn view_project_picker(state: &State) -> Element<'_, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let bg = palette.background.base.color;
    let border_color = palette.background.base.text.scale_alpha(0.15);
    let hover_bg = palette.primary.weak.color;
    let hover_fg = palette.primary.weak.text;
    let empty_color = palette.background.strong.text.scale_alpha(0.6);

    let Some(picker) = state.project_picker.as_ref() else {
        return text("").into();
    };

    let input = text_input("Filter projects…", &picker.filter)
        .on_input(Message::ProjectPickerFilterChanged)
        .id(picker.input_id.clone())
        .size(13)
        .padding([8, 12]);

    let filtered = picker.filtered_repos();

    let repo_items: Vec<Element<'_, Message>> = filtered
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let is_selected = i == picker.selected_index;
            let item_bg = if is_selected { hover_bg } else { bg };
            let item_fg = if is_selected { hover_fg } else { fg };
            let repo_owned = repo.to_string();

            // Show just the directory name
            let name = repo.rsplit('/').next().unwrap_or(repo);

            let row_content = row![text(name).size(13).font(MONO).color(item_fg),]
                .spacing(8)
                .align_y(iced::Alignment::Center);

            mouse_area(
                container(row_content)
                    .width(Fill)
                    .height(PICKER_ROW_HEIGHT)
                    .padding([6, 12])
                    .style(move |_: &Theme| container::Style::default().background(item_bg)),
            )
            .on_press(Message::SwitchProject(repo_owned))
            .into()
        })
        .collect();

    let repo_list: Element<'_, Message> = if repo_items.is_empty() {
        container(text("No recent projects").size(12).color(empty_color))
            .padding([8, 12])
            .into()
    } else {
        scrollable(column(repo_items).spacing(2))
            .id(picker.scroll_id.clone())
            .on_scroll(|viewport| {
                Message::ProjectPickerScrolled(
                    viewport.absolute_offset().y,
                    viewport.bounds().height,
                )
            })
            .height(iced::Length::Shrink)
            .into()
    };

    view_picker_dropdown(input.into(), repo_list, None, bg, border_color)
}

pub(crate) fn selected_sidebar_row_bounds(state: &State) -> Option<(f32, f32)> {
    let row_index = state.focused_sidebar_row_index()?;
    let top = 8.0 + (row_index as f32) * SIDEBAR_ROW_HEIGHT;
    let bottom = top + SIDEBAR_ROW_HEIGHT;
    Some((top, bottom))
}
