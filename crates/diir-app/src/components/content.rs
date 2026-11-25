use diir_core::theme::{Palette, ThemeId, density};
use gpui::{AnyElement, IntoElement, div, prelude::*, px, rgb};

/// Initialization view only. No file operations, sample backlog, or fake data.
pub fn content(palette: Palette, theme: ThemeId, glass: bool) -> AnyElement {
    if theme == ThemeId::WindowsExplorer {
        return super::explorer::explorer(palette).into_any_element();
    }
    div()
        .flex_1()
        .flex()
        .min_h_0()
        .p(px(density::PANEL_INSET))
        .gap(px(density::PANEL_INSET))
        .child(appearance_preview(palette, theme, glass))
        .child(content_surface(palette))
        .into_any_element()
}

/// Leave the backdrop exposed here; only the adjacent content panel is opaque.
fn appearance_preview(palette: Palette, theme: ThemeId, glass: bool) -> impl IntoElement {
    div()
        .w(px(density::PREVIEW_WIDTH))
        .flex_none()
        .flex()
        .flex_col()
        .justify_between()
        .p(px(density::PANEL_INSET))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(density::SPACING))
                .child(
                    div()
                        .text_color(rgb(palette.muted))
                        .child("Appearance preview"),
                )
                .child(div().text_lg().child(theme.name()))
                .child(div().text_color(rgb(palette.muted)).child(if glass {
                    "A tinted frame with native background blur."
                } else {
                    "A solid frame with an opaque background."
                })),
        )
        .child(
            div()
                .text_color(rgb(palette.muted))
                .child("Content and controls keep their own solid surfaces for clarity."),
        )
}

fn content_surface(palette: Palette) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .justify_center()
        .items_center()
        .gap(px(density::SPACING))
        .p(px(density::PANEL_INSET))
        .bg(rgb(palette.surface))
        .border_1()
        .border_color(rgb(palette.border))
        .rounded_md()
        .child(div().text_lg().child("Diir"))
        .child(
            div()
                .text_color(rgb(palette.muted))
                .child("A focused foundation. Ready to build."),
        )
}
