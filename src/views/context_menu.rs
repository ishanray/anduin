use crate::app::{Message, State};
use crate::{SIDEBAR_ROW_HEIGHT, lucide};
use iced::mouse::Interaction;
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Border, Color, Element, Fill, Shadow, Theme, Vector};

/// Render the context menu overlay when right-clicking a sidebar item.
/// Positioned next to the sidebar, aligned to the row that was right-clicked.
pub(crate) fn view_context_menu(state: &State) -> Element<'_, Message> {
    let Some(menu) = &state.sidebar_context_menu else {
        return Space::new().into();
    };

    let palette = state.cached_theme.extended_palette();
    let fg = palette.background.base.text;
    let menu_bg = palette.background.base.color;
    let border_color = palette.background.base.text.scale_alpha(0.12);

    let path = menu.path.clone();

    // Menu items
    let gitignore_item = menu_item(
        lucide::eye_off().size(14).color(fg).into(),
        "Add to .gitignore",
        fg,
        Message::AddToGitignore(path),
    );

    let menu_content = column![gitignore_item].spacing(0);

    // macOS-style menu card
    let menu_card = container(
        container(menu_content).padding([4, 0]),
    )
    .style(move |_: &Theme| {
        container::Style::default()
            .background(menu_bg)
            .border(Border {
                color: border_color,
                width: 1.0,
                radius: 8.0.into(),
            })
            .shadow(Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            })
    })
    .width(220);

    // Position: 320px sidebar width, vertically aligned to the clicked row
    // Account for: header (48px) + rule (1px) + tab bar (~33px) + rule (1px) + padding (8px)
    let sidebar_header_height = 48.0 + 1.0 + 33.0 + 1.0 + 8.0;
    let row_top = sidebar_header_height
        + (menu.row_index as f32) * SIDEBAR_ROW_HEIGHT
        - state.sidebar_scroll_offset;

    // Transparent backdrop to catch clicks and close the menu
    let backdrop: Element<'_, Message> = mouse_area(
        container(Space::new())
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style::default()),
    )
    .on_press(Message::CloseContextMenu)
    .on_right_press(Message::CloseContextMenu)
    .into();

    let menu_positioned = container(menu_card)
        .padding(iced::Padding {
            top: row_top.max(0.0),
            left: 324.0,
            right: 0.0,
            bottom: 0.0,
        })
        .width(Fill)
        .height(Fill);

    iced::widget::Stack::new()
        .push(backdrop)
        .push(menu_positioned)
        .width(Fill)
        .height(Fill)
        .into()
}

/// A single context menu item with icon, label, and hover highlight.
fn menu_item<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    fg: Color,
    on_press: Message,
) -> Element<'a, Message> {
    mouse_area(
        container(
            row![
                container(icon).width(20),
                text(label).size(13).color(fg),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([6, 12])
        .style(move |_: &Theme| container::Style::default()),
    )
    .on_press(on_press)
    .interaction(Interaction::Pointer)
    .into()
}
