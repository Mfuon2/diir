use diir_core::theme::{Palette, density};
use gpui::{IntoElement, div, prelude::*, px, rgb};

pub fn status(palette: Palette, glass: bool) -> impl IntoElement {
    let shortcut = if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_between()
        .h(px(density::STATUS_HEIGHT))
        .px_4()
        .border_t_1()
        .border_color(rgb(palette.border))
        .text_color(rgb(palette.muted))
        .child(if glass {
            "Native frosted glass"
        } else {
            "Opaque surfaces"
        })
        .child(format!(
            "{shortcut}+Shift+T  Theme     {shortcut}+Shift+G  Transparency"
        ))
}
