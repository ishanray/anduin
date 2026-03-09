//! Search and replace dialog UI.
//!
//! This module provides the visual interface for the search/replace functionality.

use iced::widget::{
    Space, Tooltip, button, checkbox, column, container, row, text, text_input,
    tooltip,
};
use iced::{Element, Font, Length};

use super::Message;
use super::search::{MAX_MATCHES, SearchState};
use crate::i18n::Translations;

const LUCIDE_FONT: Font = Font::with_name("lucide");

fn lucide_icon(codepoint: char, size: f32) -> iced::widget::Text<'static> {
    text(codepoint.to_string()).font(LUCIDE_FONT).size(size)
}

/// Creates the search/replace dialog UI element.
///
/// # Arguments
///
/// * `search_state` - Current search state
/// * `translations` - Translations for UI text
///
/// # Returns
///
/// An Iced element representing the search dialog, or empty space if closed
pub fn view<'a>(
    search_state: &SearchState,
    translations: &'a Translations,
) -> Element<'a, Message> {
    if !search_state.is_open {
        // Return empty Space when closed
        return Space::new().into();
    }

    // Search input field - compact, minimum practical width with placeholder
    let search_input =
        text_input(&translations.search_placeholder(), &search_state.query)
            .id(search_state.search_input_id.clone())
            .on_input(Message::SearchQueryChanged)
            .on_submit(Message::FindNext)
            .padding(4)
            .width(Length::Fixed(180.0));

    // Match counter display
    let match_info = if search_state.query.is_empty() {
        text("")
    } else if search_state.match_count() == 0 {
        text("0").size(11)
    } else {
        let count_display = if search_state.match_count() >= MAX_MATCHES {
            format!("{}+", MAX_MATCHES)
        } else {
            format!("{}", search_state.match_count())
        };

        if let Some(idx) = search_state.current_match_index {
            text(format!("{}/{}", idx + 1, count_display)).size(11)
        } else {
            text(count_display).size(11)
        }
    };

    // Navigation buttons with Lucide icons and tooltips
    let prev_button = Tooltip::new(
        button(lucide_icon('\u{e072}', 12.0))
            .on_press(Message::FindPrevious)
            .padding(2),
        text(translations.previous_match_tooltip()),
        tooltip::Position::Bottom,
    )
    .style(container::rounded_box);

    let next_button = Tooltip::new(
        button(lucide_icon('\u{e073}', 12.0))
            .on_press(Message::FindNext)
            .padding(2),
        text(translations.next_match_tooltip()),
        tooltip::Position::Bottom,
    )
    .style(container::rounded_box);

    // Case sensitivity checkbox
    let case_checkbox = checkbox(search_state.case_sensitive)
        .on_toggle(|_| Message::ToggleCaseSensitive);

    let case_icon = text("Aa").size(11);
    let case_label_text = text(translations.case_sensitive_label()).size(11);

    // Combined navigation + counter + case sensitivity row (all on one line)
    let nav_and_options_row = row![
        prev_button,
        next_button,
        match_info,
        Space::new().width(Length::Fixed(8.0)),
        case_checkbox,
        case_icon,
        Space::new().width(Length::Fixed(4.0)),
        case_label_text,
    ]
    .spacing(3)
    .align_y(iced::Alignment::Center);

    // Build the main content
    let mut content = column![search_input, nav_and_options_row].spacing(5);

    // Add replace fields if in replace mode
    if search_state.is_replace_mode {
        let replace_input = text_input(
            &translations.replace_placeholder(),
            &search_state.replace_with,
        )
        .id(search_state.replace_input_id.clone())
        .on_input(Message::ReplaceQueryChanged)
        .on_submit(Message::ReplaceNext)
        .padding(4)
        .width(Length::Fixed(180.0));

        let replace_btn = Tooltip::new(
            button(lucide_icon('\u{e3df}', 12.0))
                .on_press(Message::ReplaceNext)
                .padding(2),
            text(translations.replace_current_tooltip()),
            tooltip::Position::Bottom,
        )
        .style(container::rounded_box);

        let replace_all_btn = Tooltip::new(
            button(lucide_icon('\u{e3e0}', 12.0))
                .on_press(Message::ReplaceAll)
                .padding(2),
            text(translations.replace_all_tooltip()),
            tooltip::Position::Bottom,
        )
        .style(container::rounded_box);

        let replace_row = row![replace_btn, replace_all_btn].spacing(3);

        content = content.push(replace_input).push(replace_row);
    }

    // Close button with Lucide icon and tooltip
    let close_button = Tooltip::new(
        button(lucide_icon('\u{e1b2}', 12.0))
            .on_press(Message::CloseSearch)
            .padding(2),
        text(translations.close_search_tooltip()),
        tooltip::Position::Left,
    )
    .style(container::rounded_box);

    // Title bar with close button and Lucide search icon
    let title_row = row![
        lucide_icon('\u{e155}', 12.0),
        Space::new().width(Length::Fill),
        close_button
    ]
    .width(Length::Fixed(180.0))
    .align_y(iced::Alignment::Center);

    // Final dialog container - minimal padding with semi-transparency
    let dialog = column![title_row, content].spacing(5).padding(8);

    // Custom style with 90% opacity for semi-transparency
    container(dialog)
        .padding(6)
        .style(|theme| {
            let base = container::rounded_box(theme);
            container::Style {
                background: base.background.map(|bg| match bg {
                    iced::Background::Color(color) => {
                        iced::Background::Color(iced::Color {
                            a: 0.85, // 85% opacity
                            ..color
                        })
                    }
                    _ => bg,
                }),
                ..base
            }
        })
        .into()
}
