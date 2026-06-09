use crate::app::{Message, State};
use crate::lucide;
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

    // --- Zoom section ---
    let zoom_percent = format!("{:.0}%", state.zoom_level * 100.0);
    let zoom_label = text("Zoom").size(13).color(fg);
    let zoom_value = text(zoom_percent).size(13).font(iced::Font::MONOSPACE).color(fg);

    let zoom_btn_style = move |_: &Theme, status: button::Status| {
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

    let zoom_row = row![
        zoom_label,
        Space::new().width(Length::Fill),
        button(text("-").size(13).color(fg).center())
            .padding([4, 12])
            .style(zoom_btn_style)
            .on_press(Message::ZoomOut),
        zoom_value,
        button(text("+").size(13).color(fg).center())
            .padding([4, 12])
            .style(zoom_btn_style)
            .on_press(Message::ZoomIn),
        button(text("Reset").size(13).color(fg).center())
            .padding([4, 12])
            .style(zoom_btn_style)
            .on_press(Message::ZoomReset),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // --- Recent repos section ---
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
        .style(move |_: &Theme, status: button::Status| {
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
        })
        .on_press(Message::CloseSettings);

    let content = column![
        text("Settings").size(16).color(fg),
        Space::new().height(16),
        zoom_row,
        Space::new().height(16),
        recent_label,
        Space::new().height(4),
        repo_list,
        Space::new().height(16),
        row![Space::new().width(Fill), close_button]
            .align_y(iced::Alignment::Center),
    ]
    .spacing(0)
    .max_width(420);

    let card = container(content)
        .padding([24, 32])
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
