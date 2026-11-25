use crate::{preferences::Preferences, theme::ThemeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPage {
    Settings,
    Appearance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppearanceChoice {
    Theme(ThemeId),
    ToggleTransparency,
}

/// Small keyboard navigation state, independent of the rendering framework.
#[derive(Clone, Copy, Debug)]
pub struct SettingsMenu {
    pub page: SettingsPage,
    pub selected: usize,
}

impl Default for SettingsMenu {
    fn default() -> Self {
        Self {
            page: SettingsPage::Settings,
            selected: 0,
        }
    }
}

impl SettingsMenu {
    pub fn navigate(&mut self, forward: bool) {
        let count = match self.page {
            SettingsPage::Settings => 1,
            SettingsPage::Appearance => ThemeId::ALL.len() + 1,
        };
        self.selected = (self.selected + if forward { 1 } else { count - 1 }) % count;
    }

    pub fn activate(&mut self, preferences: Preferences) -> Option<AppearanceChoice> {
        if self.page == SettingsPage::Settings {
            self.page = SettingsPage::Appearance;
            self.selected = ThemeId::ALL
                .iter()
                .position(|theme| *theme == preferences.theme)
                .unwrap_or(0);
            None
        } else if let Some(theme) = ThemeId::ALL.get(self.selected) {
            Some(AppearanceChoice::Theme(*theme))
        } else {
            Some(AppearanceChoice::ToggleTransparency)
        }
    }

    /// Returns true when the menu should close.
    pub fn back(&mut self) -> bool {
        if self.page == SettingsPage::Appearance {
            *self = Self::default();
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_navigation_enters_current_theme_and_wraps_both_ways() {
        let preferences = Preferences {
            theme: ThemeId::WindowsExplorer,
            ..Preferences::default()
        };
        let mut menu = SettingsMenu::default();
        assert_eq!(menu.activate(preferences), None);
        assert_eq!(
            menu.activate(preferences),
            Some(AppearanceChoice::Theme(ThemeId::WindowsExplorer))
        );
        menu.navigate(true);
        assert_eq!(
            menu.activate(preferences),
            Some(AppearanceChoice::ToggleTransparency)
        );
        menu.navigate(true);
        assert_eq!(
            menu.activate(preferences),
            Some(AppearanceChoice::Theme(ThemeId::FrostedGlass))
        );
        menu.navigate(false);
        assert_eq!(
            menu.activate(preferences),
            Some(AppearanceChoice::ToggleTransparency)
        );
        assert!(!menu.back());
        assert_eq!(menu.page, SettingsPage::Settings);
        assert!(menu.back());
    }
}
