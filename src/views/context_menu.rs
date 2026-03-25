use crate::app::{Message, State};
use crate::lucide;
use iced::mouse::Interaction;
use iced::widget::{Space, column, container, mouse_area, row, text};
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
    let menu_card = container(container(menu_content).padding([4, 0]))
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

    // Position vertically aligned to the bounds of the clicked row
    let row_top = menu.bounds.y;

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
            left: menu.bounds.x + menu.bounds.width + 4.0,
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
            row![container(icon).width(20), text(label).size(13).color(fg),]
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

use iced::advanced::Renderer as AdvancedRenderer;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, Shell, Widget, layout, mouse, renderer};
use iced::event::Event;
use iced::{Length, Rectangle, Size};

pub struct ContextMenuArea<'a, Message, Theme, Renderer>
where
    Renderer: AdvancedRenderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    on_right_press: Box<dyn Fn(Rectangle) -> Message + 'a>,
}

impl<'a, Message, Theme, Renderer> ContextMenuArea<'a, Message, Theme, Renderer>
where
    Renderer: AdvancedRenderer,
{
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_right_press: impl Fn(Rectangle) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_right_press: Box::new(on_right_press),
        }
    }
}

pub fn context_menu_area<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    on_right_press: impl Fn(Rectangle) -> Message + 'a,
) -> ContextMenuArea<'a, Message, Theme, Renderer>
where
    Renderer: AdvancedRenderer,
{
    ContextMenuArea::new(content, on_right_press)
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ContextMenuArea<'a, Message, Theme, Renderer>
where
    Renderer: AdvancedRenderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        )
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event
            && cursor.is_over(layout.bounds())
        {
            shell.publish((self.on_right_press)(layout.bounds()));
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<ContextMenuArea<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: AdvancedRenderer + 'a,
{
    fn from(area: ContextMenuArea<'a, Message, Theme, Renderer>) -> Self {
        Element::new(area)
    }
}
