use rhythm_core::property::AnimatableProperty;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorShortcut {
    FocusProperty(AnimatableProperty),
    KeyframeAction,
    TogglePlayback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShortcutModifiers {
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl ShortcutModifiers {
    #[must_use]
    const fn is_empty(self) -> bool {
        !self.control && !self.shift && !self.alt && !self.super_key
    }
}

#[must_use]
pub const fn dispatch_physical_shortcut(
    code: KeyCode,
    modifiers: ShortcutModifiers,
) -> Option<EditorShortcut> {
    if !modifiers.is_empty() {
        return None;
    }

    match code {
        KeyCode::KeyP => Some(EditorShortcut::FocusProperty(AnimatableProperty::Position)),
        KeyCode::KeyS => Some(EditorShortcut::FocusProperty(AnimatableProperty::Scale)),
        KeyCode::KeyR => Some(EditorShortcut::FocusProperty(AnimatableProperty::Rotation)),
        KeyCode::KeyO => Some(EditorShortcut::FocusProperty(AnimatableProperty::Opacity)),
        KeyCode::KeyK => Some(EditorShortcut::KeyframeAction),
        KeyCode::Space => Some(EditorShortcut::TogglePlayback),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorShortcut, ShortcutModifiers, dispatch_physical_shortcut};
    use rhythm_core::property::AnimatableProperty;
    use winit::keyboard::KeyCode;

    #[test]
    fn physical_property_keys_dispatch_without_layout_text() {
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::KeyP, ShortcutModifiers::default()),
            Some(EditorShortcut::FocusProperty(AnimatableProperty::Position))
        );
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::KeyS, ShortcutModifiers::default()),
            Some(EditorShortcut::FocusProperty(AnimatableProperty::Scale))
        );
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::KeyR, ShortcutModifiers::default()),
            Some(EditorShortcut::FocusProperty(AnimatableProperty::Rotation))
        );
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::KeyO, ShortcutModifiers::default()),
            Some(EditorShortcut::FocusProperty(AnimatableProperty::Opacity))
        );
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::KeyK, ShortcutModifiers::default()),
            Some(EditorShortcut::KeyframeAction)
        );
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::Space, ShortcutModifiers::default()),
            Some(EditorShortcut::TogglePlayback)
        );
    }

    #[test]
    fn property_shortcuts_reject_modified_keys() {
        assert_eq!(
            dispatch_physical_shortcut(
                KeyCode::KeyP,
                ShortcutModifiers {
                    control: true,
                    ..ShortcutModifiers::default()
                },
            ),
            None
        );
        assert_eq!(
            dispatch_physical_shortcut(
                KeyCode::KeyK,
                ShortcutModifiers {
                    shift: true,
                    ..ShortcutModifiers::default()
                },
            ),
            None
        );
    }
}
