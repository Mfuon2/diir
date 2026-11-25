//! Keep platform policy at the window boundary, not inside reusable components.
use diir_core::preferences::Preferences;
use gpui::WindowBackgroundAppearance;

pub fn background(preferences: Preferences) -> WindowBackgroundAppearance {
    // GPUI 0.2.2 exposes native blur as a mode, without a configurable radius.
    // Linux compositor blur support varies; prefer a fully opaque fallback.
    if preferences.uses_glass() && cfg!(any(target_os = "macos", target_os = "windows")) {
        WindowBackgroundAppearance::Blurred
    } else {
        WindowBackgroundAppearance::Opaque
    }
}

pub fn uses_native_glass(preferences: Preferences) -> bool {
    background(preferences) == WindowBackgroundAppearance::Blurred
}
