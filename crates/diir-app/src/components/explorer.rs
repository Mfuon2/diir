//! Explorer-style appearance preview. Labels do not represent live file operations.
use diir_core::theme::{Palette, density};
use gpui::{IntoElement, div, prelude::*, px, rgb};

pub fn explorer(palette: Palette) -> impl IntoElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(address_bar(palette))
        .child(command_bar(palette))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .child(navigation(palette))
                .child(file_area(palette)),
        )
}

fn address_bar(palette: Palette) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(density::SPACING))
        .p(px(density::SPACING))
        .child(div().text_color(rgb(palette.muted)).child("←   →   ↑"))
        .child(
            div()
                .flex_1()
                .h(px(density::CONTROL_HEIGHT))
                .flex()
                .items_center()
                .px(px(density::SPACING))
                .bg(rgb(palette.surface))
                .border_1()
                .border_color(rgb(palette.border))
                .child("This PC"),
        )
        .child(
            div()
                .text_color(rgb(palette.muted))
                .child("Appearance preview"),
        )
}

fn command_bar(palette: Palette) -> impl IntoElement {
    div()
        .h(px(density::EXPLORER_TOOLBAR_HEIGHT))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(density::PANEL_INSET))
        .px(px(density::PANEL_INSET))
        .bg(rgb(palette.surface))
        .border_b_1()
        .border_color(rgb(palette.border))
        .text_color(rgb(palette.muted))
        .child("New  ⌄")
        .child("│")
        .child("Sort  ⌄")
        .child("View  ⌄")
        .child("···")
}

fn navigation(palette: Palette) -> impl IntoElement {
    div()
        .w(px(density::EXPLORER_SIDEBAR_WIDTH))
        .flex_none()
        .p(px(density::SPACING))
        .border_r_1()
        .border_color(rgb(palette.border))
        .children(
            ["Home", "Gallery", "This PC", "Network"]
                .into_iter()
                .map(|label| {
                    let selected = label == "This PC";
                    div()
                        .h(px(density::ROW_HEIGHT))
                        .flex()
                        .items_center()
                        .px(px(density::SPACING))
                        .when(selected, |row| row.bg(rgb(palette.selected)))
                        .child(format!("{}  {label}", if selected { "▸" } else { "  " }))
                }),
        )
}

fn file_area(palette: Palette) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(rgb(palette.surface))
        .child(
            div()
                .h(px(density::ROW_HEIGHT))
                .flex_none()
                .flex()
                .overflow_hidden()
                .border_b_1()
                .border_color(rgb(palette.border))
                .children(
                    ["Name", "Date modified", "Type", "Size"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, label)| {
                            div()
                                .w(px(if index == 0 {
                                    density::EXPLORER_NAME_WIDTH
                                } else {
                                    density::EXPLORER_COLUMN_WIDTH
                                }))
                                .flex_none()
                                .flex()
                                .items_center()
                                .px(px(density::SPACING))
                                .border_r_1()
                                .border_color(rgb(palette.border))
                                .child(label)
                        }),
                ),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(density::SPACING))
                .child("No folder is open")
                .child(
                    div()
                        .text_color(rgb(palette.muted))
                        .child("Windows Explorer appearance preview"),
                ),
        )
}
