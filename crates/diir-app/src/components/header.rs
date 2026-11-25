use diir_core::theme::{Palette, density};
use gpui::{IntoElement, div, prelude::*, px, rgb};

pub fn header(palette: Palette, settings: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_between()
        .h(px(density::HEADER_HEIGHT))
        .px_4()
        .border_b_1()
        .border_color(rgb(palette.border))
        .child(div().text_base().child("Diir"))
        .child(settings)
}
