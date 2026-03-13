use crate::MONO;
use crate::app::{Message, State};
use iced::theme::palette::Extended;
use iced::widget::{Space, center, column, container, mouse_area, row, text};
use iced::{Border, Color, Element, Fill, Theme};

/// Render a keyboard-style key cap badge.
fn kbd<'a>(label: &str, palette: &Extended) -> Element<'a, Message> {
    let bg = palette.background.strong.color;
    let fg = palette.background.strong.text;
    let border_color = palette.background.base.text.scale_alpha(0.2);

    container(
        text(label.to_owned())
            .size(12)
            .font(MONO)
            .color(fg)
            .center(),
    )
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

fn shortcut_row<'a>(
    keys: Vec<Element<'a, Message>>,
    description: &str,
    palette: &Extended,
) -> Element<'a, Message> {
    let fg = palette.background.base.text;

    let mut key_row = row![].spacing(4).align_y(iced::Alignment::Center);
    for key in keys {
        key_row = key_row.push(key);
    }

    row![
        container(key_row).width(140),
        text(description.to_owned()).size(13).color(fg),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .into()
}

pub(crate) fn view_shortcuts_help(state: &State) -> Element<'_, Message> {
    let theme = state.app_theme();
    let palette = theme.extended_palette();
    let fg = palette.background.base.text;
    let card_bg = palette.background.base.color;
    let card_border = palette.background.base.text.scale_alpha(0.15);
    let backdrop_color = Color::from_rgba(0.0, 0.0, 0.0, 0.5);

    let cmd = if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl"
    };

    let shortcuts = column![
        text("Keyboard Shortcuts").size(16).color(fg),
        Space::new().height(8),
        // Navigation
        shortcut_row(
            vec![kbd("↑", palette), kbd("↓", palette)],
            "Navigate files",
            palette
        ),
        shortcut_row(
            vec![kbd("←", palette), kbd("→", palette)],
            "Collapse / Expand",
            palette
        ),
        shortcut_row(
            vec![kbd("⌥", palette), kbd("←", palette), kbd("→", palette)],
            "Recursive collapse / expand",
            palette
        ),
        shortcut_row(
            vec![kbd("⇧", palette), kbd("↑", palette), kbd("↓", palette)],
            "Extend selection",
            palette
        ),
        Space::new().height(4),
        // Actions
        shortcut_row(vec![kbd("Space", palette)], "Stage / Unstage", palette),
        shortcut_row(vec![kbd("A", palette)], "Toggle stage all", palette),
        shortcut_row(vec![kbd("U", palette)], "Unstage all", palette),
        shortcut_row(vec![kbd("C", palette)], "Commit", palette),
        shortcut_row(vec![kbd("⌫", palette)], "Discard changes", palette),
        shortcut_row(
            vec![kbd(cmd, palette), kbd("B", palette)],
            "Switch branch",
            palette
        ),
        shortcut_row(
            vec![kbd(cmd, palette), kbd("P", palette)],
            "Switch project",
            palette
        ),
        Space::new().height(4),
        // Global
        shortcut_row(
            vec![kbd(cmd, palette), kbd("F", palette)],
            "Search in diff",
            palette
        ),
        shortcut_row(
            vec![kbd(cmd, palette), kbd("⇧", palette), kbd("F", palette)],
            "Project search",
            palette
        ),
        shortcut_row(
            vec![kbd(cmd, palette), kbd("⇧", palette), kbd("[", palette), kbd("]", palette)],
            "Switch tab",
            palette
        ),
        shortcut_row(vec![kbd("Enter", palette)], "Focus deeper pane", palette),
        shortcut_row(vec![kbd("Esc", palette)], "Focus shallower pane", palette),
        shortcut_row(vec![kbd("?", palette)], "Show this help", palette),
    ]
    .spacing(8);

    let card = container(shortcuts)
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
    .on_press(Message::ToggleActionsPanel);

    backdrop.into()
}
