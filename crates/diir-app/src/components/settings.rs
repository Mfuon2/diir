use diir_core::{
    preferences::Preferences,
    settings::{SettingsMenu, SettingsPage},
    theme::{ThemeId, density},
};
use gpui::{Context, IntoElement, deferred, div, prelude::*, px, rgb};

use crate::app::{Diir, MenuCommand};

pub fn settings(
    menu: Option<SettingsMenu>,
    preferences: Preferences,
    cx: &mut Context<Diir>,
) -> impl IntoElement + use<> {
    let palette = preferences.theme.palette();
    let view = cx.entity().downgrade();
    div()
        .relative()
        .child(super::button(
            "settings",
            "Settings",
            menu.is_some(),
            palette,
            preferences.theme.control_radius(),
            move |window, cx| {
                let command = if menu.is_some() {
                    MenuCommand::Close
                } else {
                    MenuCommand::Open
                };
                let _ = view.update(cx, |view, cx| view.handle_menu(command, window, cx));
            },
        ))
        .when_some(menu, |container, menu| {
            let mut rows = Vec::new();
            match menu.page {
                SettingsPage::Settings => rows.push(("Appearance".to_owned(), "›", false)),
                SettingsPage::Appearance => {
                    for theme in ThemeId::ALL {
                        rows.push((theme.name().to_owned(), "", preferences.theme == theme));
                    }
                    rows.push((
                        "Reduce transparency".to_owned(),
                        "",
                        preferences.reduce_transparency,
                    ));
                }
            }
            let popup =
                div()
                    .id("settings-menu")
                    .absolute()
                    .top(px(density::CONTROL_HEIGHT + density::SPACING))
                    .right_0()
                    .w(px(density::MENU_WIDTH))
                    .p(px(density::SPACING))
                    .bg(rgb(palette.surface))
                    .border_1()
                    .border_color(rgb(palette.border))
                    .rounded(px(preferences.theme.control_radius()))
                    .shadow_lg()
                    .occlude()
                    .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                        this.handle_menu(MenuCommand::Close, window, cx);
                    }))
                    .when(menu.page == SettingsPage::Appearance, |popup| {
                        popup.child(
                            div()
                                .id("settings-back")
                                .h(px(density::CONTROL_HEIGHT))
                                .text_color(rgb(palette.muted))
                                .cursor_pointer()
                                .child("‹  Settings / Appearance")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.handle_menu(MenuCommand::Back, window, cx);
                                })),
                        )
                    })
                    .children(rows.into_iter().enumerate().map(
                        |(index, (label, suffix, checked))| {
                            div()
                                .id(("settings-choice", index))
                                .flex()
                                .items_center()
                                .justify_between()
                                .h(px(density::CONTROL_HEIGHT))
                                .px(px(density::SPACING))
                                .border_1()
                                .rounded(px(preferences.theme.control_radius()))
                                .border_color(rgb(if menu.selected == index {
                                    palette.accent
                                } else {
                                    palette.surface
                                }))
                                .bg(rgb(if menu.selected == index {
                                    palette.selected
                                } else {
                                    palette.surface
                                }))
                                .cursor_pointer()
                                .hover(move |style| {
                                    style
                                        .bg(rgb(palette.selected))
                                        .border_color(rgb(palette.accent))
                                })
                                .child(format!("{}  {label}", if checked { "✓" } else { "  " }))
                                .child(suffix)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.handle_menu(MenuCommand::Choose(index), window, cx);
                                }))
                        },
                    ));
            container.child(deferred(popup).with_priority(1))
        })
}
