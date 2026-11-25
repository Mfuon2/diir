//! Semantic colors use 0xRRGGBB; surfaces are opaque unless explicitly composited.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeId {
    #[default]
    FrostedGlass,
    Dracula,
    Monokai,
    WindowsExplorer,
}

impl ThemeId {
    pub const ALL: [Self; 4] = [
        Self::FrostedGlass,
        Self::Dracula,
        Self::Monokai,
        Self::WindowsExplorer,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::FrostedGlass => "Frosted Glass",
            Self::Dracula => "Dracula",
            Self::Monokai => "Monokai",
            Self::WindowsExplorer => "Windows Explorer",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::FrostedGlass => Self::Dracula,
            Self::Dracula => Self::Monokai,
            Self::Monokai => Self::WindowsExplorer,
            Self::WindowsExplorer => Self::FrostedGlass,
        }
    }

    pub const fn control_radius(self) -> f32 {
        match self {
            Self::WindowsExplorer => 2.0,
            _ => 4.0,
        }
    }

    pub const fn palette(self) -> Palette {
        match self {
            Self::FrostedGlass => Palette {
                background: 0x141c2a,
                surface: 0x1c2636,
                selected: 0x293c54,
                border: 0x9baec7,
                text: 0xf1f5fb,
                muted: 0xc6d2e3,
                accent: 0xa5d2ff,
            },
            Self::Dracula => Palette {
                background: 0x282a36,
                surface: 0x21222c,
                selected: 0x383a4b,
                border: 0x9b9fb6,
                text: 0xf8f8f2,
                muted: 0xbfc2d4,
                accent: 0xbd93f9,
            },
            Self::Monokai => Palette {
                background: 0x272822,
                surface: 0x20211c,
                selected: 0x49483e,
                border: 0xa19f90,
                text: 0xf8f8f2,
                muted: 0xc4c2b5,
                accent: 0xa6e22e,
            },
            Self::WindowsExplorer => Palette {
                background: 0xf3f3f3,
                surface: 0xffffff,
                selected: 0xe5f1fb,
                border: 0x6b7280,
                text: 0x1f1f1f,
                muted: 0x525252,
                accent: 0x005a9e,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub background: u32,
    pub surface: u32,
    pub selected: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
}

/// Only the outer window tint is translucent; content surfaces stay opaque.
// A substantial tint preserves contrast while letting the native backdrop show through.
pub const GLASS_ALPHA: u8 = 208;

pub mod density {
    pub const ROW_HEIGHT: f32 = 28.0;
    pub const CONTROL_HEIGHT: f32 = 28.0;
    pub const HEADER_HEIGHT: f32 = 44.0;
    pub const STATUS_HEIGHT: f32 = 28.0;
    pub const FONT_SIZE: f32 = 13.0;
    pub const SPACING: f32 = 8.0;
    pub const PANEL_INSET: f32 = 16.0;
    pub const PREVIEW_WIDTH: f32 = 224.0;
    pub const MENU_WIDTH: f32 = 240.0;
    pub const EXPLORER_SIDEBAR_WIDTH: f32 = 184.0;
    pub const EXPLORER_TOOLBAR_HEIGHT: f32 = 40.0;
    pub const EXPLORER_NAME_WIDTH: f32 = 220.0;
    pub const EXPLORER_COLUMN_WIDTH: f32 = 140.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::Preferences;

    fn luminance(color: u32) -> f64 {
        let channel = |shift: u32| {
            let value = f64::from(((color >> shift) & 255) as u8) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }

    fn contrast(a: u32, b: u32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    fn composite(tint: u32, backdrop: u32) -> u32 {
        let alpha = u32::from(GLASS_ALPHA);
        [0, 8, 16].into_iter().fold(0, |result, shift| {
            let front = (tint >> shift) & 255;
            let back = (backdrop >> shift) & 255;
            result | (((front * alpha + back * (255 - alpha)) / 255) << shift)
        })
    }

    #[test]
    fn text_remains_readable_on_every_content_surface() {
        for theme in ThemeId::ALL {
            let p = theme.palette();
            for surface in [p.background, p.surface, p.selected] {
                for foreground in [p.text, p.muted, p.accent] {
                    assert!(contrast(foreground, surface) >= 4.5, "{}", theme.name());
                }
                assert!(
                    contrast(p.border, surface) >= 3.0,
                    "{} border",
                    theme.name()
                );
            }
        }
    }

    #[test]
    fn glass_text_survives_light_and_dark_desktop_backdrops() {
        let p = ThemeId::FrostedGlass.palette();
        for backdrop in [0x000000, 0xffffff] {
            let surface = composite(p.background, backdrop);
            assert!(contrast(p.text, surface) >= 4.5);
            assert!(contrast(p.muted, surface) >= 4.5);
            assert!(contrast(p.accent, surface) >= 4.5);
            assert!(contrast(p.border, surface) >= 3.0);
        }
    }

    #[test]
    fn reduced_transparency_overrides_glass_and_theme_cycle_returns_to_default() {
        let mut preferences = Preferences::default();
        assert!(preferences.uses_glass());
        preferences.reduce_transparency = true;
        assert!(!preferences.uses_glass());
        for theme in ThemeId::ALL {
            preferences.theme = theme;
            assert!(!preferences.uses_glass());
        }
        let mut theme = ThemeId::default();
        for expected in ThemeId::ALL {
            assert_eq!(theme, expected);
            theme = theme.next();
        }
        assert_eq!(theme, ThemeId::default());
    }
}
