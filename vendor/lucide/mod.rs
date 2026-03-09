use iced::Font;
use iced::widget::{Text, text};

pub const LUCIDE_FONT_BYTES: &[u8] = include_bytes!("lucide.ttf");
pub const LUCIDE_FONT: Font = Font::with_name("lucide");

fn icon(codepoint: char) -> Text<'static> {
    text(codepoint.to_string()).font(LUCIDE_FONT)
}

pub fn sun() -> Text<'static> {
    icon('\u{e17c}')
}

pub fn moon() -> Text<'static> {
    icon('\u{e122}')
}

pub fn plus() -> Text<'static> {
    icon('\u{e141}')
}

pub fn settings() -> Text<'static> {
    icon('\u{e158}')
}

pub fn search() -> Text<'static> {
    icon('\u{e155}')
}

pub fn x() -> Text<'static> {
    icon('\u{e1b2}')
}

pub fn chevron_down() -> Text<'static> {
    icon('\u{e071}')
}

pub fn chevron_right() -> Text<'static> {
    icon('\u{e073}')
}

pub fn folder_open() -> Text<'static> {
    icon('\u{e247}')
}

pub fn folder() -> Text<'static> {
    icon('\u{e0db}')
}

pub fn minus() -> Text<'static> {
    icon('\u{e120}')
}

pub fn pencil() -> Text<'static> {
    icon('\u{e1f9}')
}

pub fn arrow_right_left() -> Text<'static> {
    icon('\u{e41c}')
}

pub fn circle() -> Text<'static> {
    icon('\u{e07a}')
}

pub fn file() -> Text<'static> {
    icon('\u{e0c4}')
}

pub fn ellipsis() -> Text<'static> {
    icon('\u{e0ba}')
}

pub fn git_branch() -> Text<'static> {
    icon('\u{e0d4}')
}

pub fn check() -> Text<'static> {
    icon('\u{e06e}')
}
