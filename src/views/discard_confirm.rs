use crate::app::{DiscardButton, Message, State};
use iced::widget::{Space, button, center, column, container, mouse_area, row, text};
use iced::{Border, Color, Element, Fill, Theme};

pub(crate) fn view_discard_confirm(state: &State) -> Element<'_, Message> {
    let Some(confirm) = &state.discard_confirm else {
        return Space::new().into();
    };

    let palette = state.cached_theme.extended_palette();
    let fg = palette.background.base.text;
    let card_bg = palette.background.base.color;
    let card_border = palette.background.base.text.scale_alpha(0.15);
    let backdrop_color = Color::from_rgba(0.0, 0.0, 0.0, 0.5);
    let focus_color = palette.primary.strong.color;

    let count = confirm.paths.len();
    let focused = confirm.focused_button;
    let description = if count == 1 {
        format!("Discard changes to \"{}\"?", confirm.paths[0])
    } else {
        format!("Discard changes to {count} files?")
    };

    let warning = "This action cannot be undone.";

    let danger_bg = palette.danger.base.color;
    let danger_text = palette.danger.base.text;
    let subtle_fg = palette.background.base.text.scale_alpha(0.7);
    let discard_focused = focused == DiscardButton::Discard;
    let cancel_focused = focused == DiscardButton::Cancel;

    let discard_button = button(text("Discard").size(13).color(danger_text).center())
        .padding([6, 20])
        .style(move |_: &Theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    iced::Background::Color(Color {
                        a: danger_bg.a * 0.85,
                        ..danger_bg
                    })
                }
                _ => iced::Background::Color(danger_bg),
            };
            let border = if discard_focused {
                Border {
                    color: focus_color,
                    width: 2.0,
                    radius: 6.0.into(),
                }
            } else {
                Border {
                    radius: 6.0.into(),
                    ..Border::default()
                }
            };
            button::Style {
                background: Some(bg),
                text_color: danger_text,
                border,
                ..button::Style::default()
            }
        })
        .on_press(Message::ConfirmDiscard);

    let cancel_button = button(text("Cancel").size(13).color(fg).center())
        .padding([6, 20])
        .style(move |_: &Theme, status| {
            let bg_color = palette.background.strong.color;
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    iced::Background::Color(Color {
                        a: bg_color.a * 0.85,
                        ..bg_color
                    })
                }
                _ => iced::Background::Color(bg_color),
            };
            let border = if cancel_focused {
                Border {
                    color: focus_color,
                    width: 2.0,
                    radius: 6.0.into(),
                }
            } else {
                Border {
                    radius: 6.0.into(),
                    ..Border::default()
                }
            };
            button::Style {
                background: Some(bg),
                text_color: fg,
                border,
                ..button::Style::default()
            }
        })
        .on_press(Message::CancelDiscard);

    // File list preview (up to 10 files)
    let mut file_list = column![].spacing(2);
    let display_count = count.min(10);
    for path in confirm.paths.iter().take(display_count) {
        file_list = file_list.push(
            text(format!("  {path}"))
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(subtle_fg),
        );
    }
    if count > display_count {
        file_list = file_list.push(
            text(format!("  … and {} more", count - display_count))
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(subtle_fg),
        );
    }

    let content = column![
        text("Discard Changes").size(16).color(fg),
        Space::new().height(8),
        text(description).size(13).color(fg),
        Space::new().height(6),
        file_list,
        Space::new().height(8),
        text(warning).size(12).color(palette.danger.base.color),
        Space::new().height(16),
        row![cancel_button, discard_button]
            .spacing(8)
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
    .on_press(Message::CancelDiscard);

    backdrop.into()
}
