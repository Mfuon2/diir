use std::{cell::RefCell, error::Error, rc::Rc};

use diir_core::{
    preferences::Preferences,
    settings::{AppearanceChoice, SettingsMenu},
    theme::{GLASS_ALPHA, ThemeId, density},
};
use gpui::{
    App, Application, Bounds, Context, FocusHandle, KeyBinding, Window, WindowBounds,
    WindowOptions, actions, div, prelude::*, px, rgb, rgba, size,
};

use crate::{components, platform};

actions!(
    diir,
    [
        Quit,
        CycleTheme,
        ToggleTransparency,
        OpenSettings,
        MenuNext,
        MenuPrevious,
        MenuActivate,
        MenuBack,
        MenuClose
    ]
);

pub(crate) enum MenuCommand {
    Open,
    Close,
    Back,
    Choose(usize),
}

pub(crate) struct Diir {
    preferences: Preferences,
    focus: FocusHandle,
    menu: Option<SettingsMenu>,
}

impl Diir {
    pub(crate) fn handle_menu(
        &mut self,
        command: MenuCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            MenuCommand::Open => self.menu = Some(SettingsMenu::default()),
            MenuCommand::Close => self.menu = None,
            MenuCommand::Back => {
                if self.menu.as_mut().is_some_and(SettingsMenu::back) {
                    self.menu = None;
                }
            }
            MenuCommand::Choose(index) => {
                if let Some(menu) = self.menu.as_mut() {
                    menu.selected = index;
                    self.activate_menu(window, cx);
                }
            }
        }
        window.focus(&self.focus);
        cx.notify();
    }

    fn navigate_menu(&mut self, forward: bool, cx: &mut Context<Self>) {
        if let Some(menu) = self.menu.as_mut() {
            menu.navigate(forward);
            cx.notify();
        }
    }

    fn activate_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(choice) = self
            .menu
            .as_mut()
            .and_then(|menu| menu.activate(self.preferences))
        {
            self.menu = None;
            match choice {
                AppearanceChoice::Theme(theme) => self.set_theme(theme, window, cx),
                AppearanceChoice::ToggleTransparency => self.toggle_transparency(window, cx),
            }
        }
        cx.notify();
    }

    fn set_theme(&mut self, theme: ThemeId, window: &mut Window, cx: &mut Context<Self>) {
        self.preferences.theme = theme;
        self.refresh_appearance(window, cx);
    }

    fn toggle_transparency(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.preferences.reduce_transparency = !self.preferences.reduce_transparency;
        self.refresh_appearance(window, cx);
    }

    fn refresh_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.set_background_appearance(platform::background(self.preferences));
        cx.notify();
    }
}

impl Render for Diir {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.preferences.theme.palette();
        let glass = platform::uses_native_glass(self.preferences);
        let background = if glass {
            rgba((palette.background << 8) | u32::from(GLASS_ALPHA))
        } else {
            rgb(palette.background)
        };

        let settings = components::settings(self.menu, self.preferences, cx);

        div()
            .track_focus(&self.focus)
            .key_context(if self.menu.is_some() {
                "DiirSettings"
            } else {
                "Diir"
            })
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.handle_menu(MenuCommand::Open, window, cx)
            }))
            .on_action(cx.listener(|this, _: &MenuClose, window, cx| {
                this.handle_menu(MenuCommand::Close, window, cx)
            }))
            .on_action(cx.listener(|this, _: &MenuBack, window, cx| {
                this.handle_menu(MenuCommand::Back, window, cx)
            }))
            .on_action(cx.listener(|this, _: &MenuNext, _, cx| this.navigate_menu(true, cx)))
            .on_action(cx.listener(|this, _: &MenuPrevious, _, cx| this.navigate_menu(false, cx)))
            .on_action(
                cx.listener(|this, _: &MenuActivate, window, cx| this.activate_menu(window, cx)),
            )
            .on_action(cx.listener(|this, _: &CycleTheme, window, cx| {
                this.set_theme(this.preferences.theme.next(), window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleTransparency, window, cx| {
                this.toggle_transparency(window, cx);
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_size(px(density::FONT_SIZE))
            .text_color(rgb(palette.text))
            .child(components::header(palette, settings))
            .child(components::content(palette, self.preferences.theme, glass))
            .child(components::status(palette, glass))
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    // Report startup failures to main instead of panicking in the window callback.
    let startup_error = Rc::new(RefCell::new(None));
    let error_slot = startup_error.clone();
    Application::new().run(move |cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        let modifier = if cfg!(target_os = "macos") {
            "cmd"
        } else {
            "ctrl"
        };
        cx.bind_keys([
            KeyBinding::new(&format!("{modifier}-q"), Quit, None),
            KeyBinding::new(&format!("{modifier}-shift-t"), CycleTheme, None),
            KeyBinding::new(&format!("{modifier}-shift-g"), ToggleTransparency, None),
        ]);
        cx.bind_keys([
            KeyBinding::new(&format!("{modifier}-,"), OpenSettings, None),
            KeyBinding::new("down", MenuNext, Some("DiirSettings")),
            KeyBinding::new("tab", MenuNext, Some("DiirSettings")),
            KeyBinding::new("up", MenuPrevious, Some("DiirSettings")),
            KeyBinding::new("shift-tab", MenuPrevious, Some("DiirSettings")),
            KeyBinding::new("enter", MenuActivate, Some("DiirSettings")),
            KeyBinding::new("space", MenuActivate, Some("DiirSettings")),
            KeyBinding::new("right", MenuActivate, Some("DiirSettings")),
            KeyBinding::new("left", MenuBack, Some("DiirSettings")),
            KeyBinding::new("escape", MenuClose, Some("DiirSettings")),
        ]);
        let preferences = Preferences::default();
        let bounds = Bounds::centered(None, size(px(1080.), px(720.)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(720.), px(480.))),
                window_background: platform::background(preferences),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Diir".into()),
                    ..Default::default()
                }),
                app_id: Some("diir".into()),
                ..Default::default()
            },
            |window, cx| {
                window.on_window_should_close(cx, |_, cx| {
                    cx.quit();
                    true
                });
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus);
                    Diir {
                        preferences,
                        focus,
                        menu: None,
                    }
                })
            },
        );
        if let Err(error) = result {
            *error_slot.borrow_mut() = Some(error.to_string());
            cx.quit();
        } else {
            cx.activate(true);
        }
    });
    if let Some(error) = startup_error.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}
