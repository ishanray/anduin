use crate::app::{Message, State};
use crate::config::AppTheme;
use crate::lucide;
use iced::theme::palette::Extended;
use iced::widget::{Space, button, center, column, container, mouse_area, row, scrollable, text};
use iced::{Border, Color, Element, Fill, Length, Theme};
use std::path::PathBuf;

pub(crate) fn view_settings_dialog(state: &State) -> Element<'_, Message> {
    let palette = state.cached_theme.extended_palette();
    let fg = palette.background.base.text;
    let card_bg = palette.background.base.color;
    let card_border = palette.background.base.text.scale_alpha(0.15);
    let backdrop_color = Color::from_rgba(0.0, 0.0, 0.0, 0.5);
    let muted_fg = palette.background.strong.text.scale_alpha(0.6);
    let danger_color = palette.danger.base.color;

    let btn_style = move |theme: &Theme, status: button::Status| {
        let palette = theme.extended_palette();
        let bg = palette.background.strong.color;
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => {
                Color { a: bg.a * 0.85, ..bg }
            }
            _ => bg,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: fg,
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    };

    let theme_label = text("Theme").size(13).color(fg);
    let current_name = state.current_theme.display_name();
    let theme_value = text(current_name).size(13).font(iced::Font::MONOSPACE).color(fg);

    let theme_icon = if state.current_theme.is_dark() {
        lucide::sun().size(16).color(fg)
    } else {
        lucide::moon().size(16).color(fg)
    };

    let chevron = if state.theme_dropdown_open {
        lucide::chevron_up().size(14).color(fg)
    } else {
        lucide::chevron_down().size(14).color(fg)
    };

    let dropdown_trigger = button(
        row![theme_value, Space::new().width(8), chevron]
            .align_y(iced::Alignment::Center),
    )
    .padding([4, 12])
    .style(btn_style)
    .on_press(Message::ToggleThemeDropdown);

    let theme_toggle_row = row![
        theme_label,
        Space::new().width(Length::Fill),
        dropdown_trigger,
        button(theme_icon)
            .padding([4, 12])
            .style(btn_style)
            .on_press(Message::ToggleTheme),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let theme_dropdown: Element<'_, Message> = if state.theme_dropdown_open {
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        items.push(
            container(text("Light").size(11).color(muted_fg))
                .padding(iced::Padding {
                    top: 6.0,
                    right: 12.0,
                    bottom: 2.0,
                    left: 12.0,
                })
                .into(),
        );
        for &t in AppTheme::all_light() {
            items.push(theme_dropdown_item(state, t, palette, fg));
        }

        items.push(
            container(text("Dark").size(11).color(muted_fg))
                .padding(iced::Padding {
                    top: 8.0,
                    right: 12.0,
                    bottom: 2.0,
                    left: 12.0,
                })
                .into(),
        );
        for &t in AppTheme::all_dark() {
            items.push(theme_dropdown_item(state, t, palette, fg));
        }

        let dropdown_bg = card_bg;
        let dropdown_border = card_border;

        container(
            scrollable(column(items).spacing(1).padding([4, 0]))
                .height(Length::Fixed(280.0)),
        )
        .width(Fill)
        .style(move |_: &Theme| {
            container::Style::default()
                .background(dropdown_bg)
                .border(Border {
                    color: dropdown_border,
                    width: 1.0,
                    radius: 8.0.into(),
                })
                .shadow(iced::Shadow {
                    color: Color::BLACK.scale_alpha(0.2),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                })
        })
        .into()
    } else {
        Space::new().height(0).into()
    };

    let zoom_percent = format!("{:.0}%", state.zoom_level * 100.0);
    let zoom_label = text("Zoom").size(13).color(fg);
    let zoom_value = text(zoom_percent).size(13).font(iced::Font::MONOSPACE).color(fg);

    let zoom_row = row![
        zoom_label,
        Space::new().width(Length::Fill),
        button(text("-").size(13).color(fg).center())
            .padding([4, 12])
            .style(btn_style)
            .on_press(Message::ZoomOut),
        zoom_value,
        button(text("+").size(13).color(fg).center())
            .padding([4, 12])
            .style(btn_style)
            .on_press(Message::ZoomIn),
        button(text("Reset").size(13).color(fg).center())
            .padding([4, 12])
            .style(btn_style)
            .on_press(Message::ZoomReset),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let recent_label = text("Recent Repositories")
        .size(13)
        .color(fg);

    let mut repo_items = Vec::new();
    for repo in &state.recent_repos {
        let name = repo.rsplit('/').next().unwrap_or(repo);
        let repo_owned = repo.clone();
        let remove_repo = repo.clone();

        let repo_row = row![
            text(name)
                .size(13)
                .font(iced::Font::MONOSPACE)
                .color(fg)
                .width(Fill),
            button(lucide::x().size(14).color(danger_color))
                .padding([4, 8])
                .style(button::text)
                .on_press(Message::RemoveRecentRepo(remove_repo)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let repo_el = mouse_area(
            container(repo_row)
                .width(Fill)
                .padding([6, 10])
                .style(move |_: &Theme| container::Style::default().background(iced::Color::TRANSPARENT)),
        )
        .on_press(Message::RepoOpened(Some(PathBuf::from(repo_owned))))
        .into();

        repo_items.push(repo_el);
    }

    let repo_list: Element<'_, Message> = if repo_items.is_empty() {
        container(text("No recent repositories").size(12).color(muted_fg))
            .padding([8, 10])
            .into()
    } else {
        scrollable(column(repo_items).spacing(2))
            .height(200.0)
            .into()
    };

    let close_button = button(text("Close").size(13).color(fg).center())
        .padding([6, 20])
        .style(btn_style)
        .on_press(Message::CloseSettings);

    let mut content_items = column![
        text("Settings").size(16).color(fg),
        Space::new().height(16),
        theme_toggle_row,
    ]
    .spacing(0)
    .max_width(420);

    if state.theme_dropdown_open {
        content_items = content_items.push(Space::new().height(8));
        content_items = content_items.push(theme_dropdown);
    }

    content_items = content_items
        .push(Space::new().height(16))
        .push(zoom_row)
        .push(Space::new().height(16))
        .push(recent_label)
        .push(Space::new().height(4))
        .push(repo_list)
        .push(Space::new().height(16))
        .push(
            row![Space::new().width(Fill), close_button]
                .align_y(iced::Alignment::Center),
        );

    let card = container(content_items)
        .padding([24, 32])
        .max_width(480)
        .style(move |_: &Theme| {
            container::Style::default()
                .background(card_bg)
                .border(Border {
                    color: card_border,
                    width: 1.0,
                    radius: 12.0.into(),
                })
        });

    let centered = center(card).width(Fill).height(Fill);

    let backdrop = mouse_area(
        container(centered)
            .width(Fill)
            .height(Fill)
            .style(move |_: &Theme| container::Style::default().background(backdrop_color)),
    )
    .on_press(Message::CloseSettings);

    backdrop.into()
}

fn theme_dropdown_item<'a>(
    state: &'a State,
    theme: AppTheme,
    palette: &'a Extended,
    fg: Color,
) -> Element<'a, Message> {
    let is_selected = state.current_theme == theme;
    let is_hovered = state.hover_preview_theme == Some(theme);
    let bg = if is_selected || is_hovered {
        palette.primary.weak.color.scale_alpha(0.35)
    } else {
        Color::TRANSPARENT
    };
    let text_color = if is_selected || is_hovered {
        palette.primary.weak.text
    } else {
        fg
    };

    let check_mark = if is_selected {
        text("✓").size(13).color(text_color)
    } else {
        text("").size(13)
    };

    let row_inner = row![
        text(theme.display_name())
            .size(13)
            .color(text_color)
            .width(Fill),
        check_mark,
    ]
    .align_y(iced::Alignment::Center);

    mouse_area(
        container(row_inner)
            .width(Fill)
            .padding([6, 12])
            .style(move |_: &Theme| {
                container::Style::default()
                    .background(bg)
                    .border(Border {
                        color: if is_selected {
                            palette.primary.weak.color.scale_alpha(0.6)
                        } else {
                            Color::TRANSPARENT
                        },
                        width: 1.0,
                        radius: 6.0.into(),
                    })
            }),
    )
    .on_enter(Message::ThemeHovered(theme))
    .on_exit(Message::ThemeHoverExited)
    .on_press(Message::ThemeSelected(theme))
    .into()
}
