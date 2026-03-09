//! Search dialog UI – minimal inline search bar.

use iced::widget::{Space, container, row, text, text_input};
use iced::{Element, Font, Length};

use super::Message;
use super::search::{MAX_MATCHES, SearchState};
use crate::i18n::Translations;

const LUCIDE_FONT: Font = Font::with_name("lucide");

fn lucide_icon(codepoint: char, size: f32) -> iced::widget::Text<'static> {
    text(codepoint.to_string()).font(LUCIDE_FONT).size(size)
}

/// Creates the search dialog UI element.
pub fn view<'a>(
    search_state: &SearchState,
    translations: &'a Translations,
) -> Element<'a, Message> {
    if !search_state.is_open {
        return Space::new().into();
    }

    let search_input =
        text_input(&translations.search_placeholder(), &search_state.query)
            .id(search_state.search_input_id.clone())
            .on_input(Message::SearchQueryChanged)
            .on_submit(Message::FindNext)
            .padding([4, 8])
            .width(Length::Fixed(220.0));

    let match_info: Element<'a, Message> = if search_state.query.is_empty() {
        Space::new().into()
    } else if search_state.match_count() == 0 {
        text("No results").size(11).into()
    } else {
        let count_display = if search_state.match_count() >= MAX_MATCHES {
            format!("{}+", MAX_MATCHES)
        } else {
            format!("{}", search_state.match_count())
        };

        if let Some(idx) = search_state.current_match_index {
            text(format!("{}/{}", idx + 1, count_display)).size(11).into()
        } else {
            text(count_display).size(11).into()
        }
    };

    let bar = row![
        lucide_icon('\u{e155}', 13.0),
        search_input,
        match_info,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    container(bar)
        .padding([6, 10])
        .style(|theme| {
            let base = container::rounded_box(theme);
            container::Style {
                background: base.background.map(|bg| match bg {
                    iced::Background::Color(color) => {
                        iced::Background::Color(iced::Color {
                            a: 0.92,
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
