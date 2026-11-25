use diir_core::theme::{Palette, density};
use gpui::{App, IntoElement, SharedString, Window, div, prelude::*, px, rgb};

pub fn button(
    id: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    palette: Palette,
    radius: f32,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(density::CONTROL_HEIGHT))
        .px_3()
        .flex()
        .items_center()
        .rounded(px(radius))
        .border_1()
        .border_color(rgb(if selected {
            palette.accent
        } else {
            palette.border
        }))
        .bg(rgb(if selected {
            palette.selected
        } else {
            palette.surface
        }))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(move |style| {
            style
                .bg(rgb(palette.selected))
                .border_color(rgb(palette.accent))
        })
        .child(label.into())
        .on_click(move |_, window, cx| on_click(window, cx))
}
