use rhythm_core::property::AnimatableProperty;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorShortcut {
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    Undo,
    Redo,
    Copy,
    Paste,
    Duplicate,
    SelectAll,
    DeleteSelection,
    FocusProperty(AnimatableProperty),
    KeyframeAction,
    TogglePlayback,
    ToggleCommandSearch,
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
    if modifiers.control && !modifiers.alt && !modifiers.super_key {
        if modifiers.shift {
            return match code {
                KeyCode::KeyS => Some(EditorShortcut::SaveProjectAs),
                KeyCode::KeyZ => Some(EditorShortcut::Redo),
                _ => None,
            };
        }

        return match code {
            KeyCode::KeyN => Some(EditorShortcut::NewProject),
            KeyCode::KeyO => Some(EditorShortcut::OpenProject),
            KeyCode::KeyS => Some(EditorShortcut::SaveProject),
            KeyCode::KeyZ => Some(EditorShortcut::Undo),
            KeyCode::KeyY => Some(EditorShortcut::Redo),
            KeyCode::KeyC => Some(EditorShortcut::Copy),
            KeyCode::KeyV => Some(EditorShortcut::Paste),
            KeyCode::KeyD => Some(EditorShortcut::Duplicate),
            KeyCode::KeyA => Some(EditorShortcut::SelectAll),
            KeyCode::KeyK => Some(EditorShortcut::ToggleCommandSearch),
            _ => None,
        };
    }

    if !modifiers.is_empty() {
        return None;
    }

    match code {
        KeyCode::Delete => Some(EditorShortcut::DeleteSelection),
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

    fn ctrl() -> ShortcutModifiers {
        ShortcutModifiers {
            control: true,
            ..ShortcutModifiers::default()
        }
    }

    fn ctrl_shift() -> ShortcutModifiers {
        ShortcutModifiers {
            control: true,
            shift: true,
            ..ShortcutModifiers::default()
        }
    }

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
    fn global_ctrl_shortcuts_dispatch_from_physical_keys() {
        for (code, expected) in [
            (KeyCode::KeyN, EditorShortcut::NewProject),
            (KeyCode::KeyO, EditorShortcut::OpenProject),
            (KeyCode::KeyS, EditorShortcut::SaveProject),
            (KeyCode::KeyZ, EditorShortcut::Undo),
            (KeyCode::KeyY, EditorShortcut::Redo),
            (KeyCode::KeyC, EditorShortcut::Copy),
            (KeyCode::KeyV, EditorShortcut::Paste),
            (KeyCode::KeyD, EditorShortcut::Duplicate),
            (KeyCode::KeyA, EditorShortcut::SelectAll),
            (KeyCode::KeyK, EditorShortcut::ToggleCommandSearch),
        ] {
            assert_eq!(dispatch_physical_shortcut(code, ctrl()), Some(expected));
        }

        assert_eq!(
            dispatch_physical_shortcut(KeyCode::KeyS, ctrl_shift()),
            Some(EditorShortcut::SaveProjectAs)
        );
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::KeyZ, ctrl_shift()),
            Some(EditorShortcut::Redo)
        );
        assert_eq!(
            dispatch_physical_shortcut(KeyCode::Delete, ShortcutModifiers::default()),
            Some(EditorShortcut::DeleteSelection)
        );
    }

    #[test]
    fn unsupported_modified_shortcuts_are_rejected() {
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
        assert_eq!(
            dispatch_physical_shortcut(
                KeyCode::KeyA,
                ShortcutModifiers {
                    alt: true,
                    ..ShortcutModifiers::default()
                },
            ),
            None
        );
    }
}
