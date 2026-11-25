use crate::theme::ThemeId;

/// Session preferences; persistence is deliberately outside initialization scope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preferences {
    pub theme: ThemeId,
    pub reduce_transparency: bool,
}

impl Preferences {
    pub fn uses_glass(self) -> bool {
        self.theme == ThemeId::FrostedGlass && !self.reduce_transparency
    }
}
