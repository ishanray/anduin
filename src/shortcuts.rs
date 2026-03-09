use iced::keyboard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutPlatform {
    Mac,
    Linux,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutAction {
    OpenDiff,
    OpenProject,
    CloseActive,
}

pub(crate) fn current_shortcut_platform() -> ShortcutPlatform {
    if cfg!(target_os = "macos") {
        ShortcutPlatform::Mac
    } else if cfg!(target_os = "windows") {
        ShortcutPlatform::Windows
    } else {
        ShortcutPlatform::Linux
    }
}

pub(crate) fn event_modifiers(event: &keyboard::Event) -> keyboard::Modifiers {
    match event {
        keyboard::Event::KeyPressed { modifiers, .. } => *modifiers,
        keyboard::Event::KeyReleased { modifiers, .. } => *modifiers,
        keyboard::Event::ModifiersChanged(modifiers) => *modifiers,
    }
}

pub(crate) fn shortcut_action_for_event(
    platform: ShortcutPlatform,
    event: &keyboard::Event,
) -> Option<ShortcutAction> {
    let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
        return None;
    };

    shortcut_action_for_key(platform, key.as_ref(), *modifiers)
}

pub(crate) fn shortcut_action_for_key(
    platform: ShortcutPlatform,
    key: keyboard::Key<&str>,
    modifiers: keyboard::Modifiers,
) -> Option<ShortcutAction> {
    match key {
        keyboard::Key::Character(c) if is_primary_modifier_pressed(platform, modifiers) => {
            if c.eq_ignore_ascii_case("f") {
                if modifiers.shift() {
                    Some(ShortcutAction::OpenProject)
                } else {
                    Some(ShortcutAction::OpenDiff)
                }
            } else {
                None
            }
        }
        keyboard::Key::Named(keyboard::key::Named::Escape) => Some(ShortcutAction::CloseActive),
        _ => None,
    }
}

pub(crate) fn is_primary_modifier_pressed(
    platform: ShortcutPlatform,
    modifiers: keyboard::Modifiers,
) -> bool {
    match platform {
        ShortcutPlatform::Mac => modifiers.logo(),
        ShortcutPlatform::Linux | ShortcutPlatform::Windows => modifiers.control(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ShortcutAction, ShortcutPlatform, is_primary_modifier_pressed, shortcut_action_for_key,
    };
    use iced::keyboard;

    #[test]
    fn mac_shortcuts_use_logo_modifier() {
        let command = keyboard::Modifiers::LOGO;
        let command_shift = keyboard::Modifiers::LOGO | keyboard::Modifiers::SHIFT;

        assert!(is_primary_modifier_pressed(ShortcutPlatform::Mac, command));
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Mac,
                keyboard::Key::Character("f"),
                command
            ),
            Some(ShortcutAction::OpenDiff)
        );
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Mac,
                keyboard::Key::Character("f"),
                command_shift,
            ),
            Some(ShortcutAction::OpenProject)
        );
    }

    #[test]
    fn linux_shortcuts_use_control_modifier() {
        let command = keyboard::Modifiers::CTRL;
        let command_shift = keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT;

        assert!(is_primary_modifier_pressed(
            ShortcutPlatform::Linux,
            command
        ));
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Linux,
                keyboard::Key::Character("f"),
                command,
            ),
            Some(ShortcutAction::OpenDiff)
        );
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Linux,
                keyboard::Key::Character("f"),
                command_shift,
            ),
            Some(ShortcutAction::OpenProject)
        );
    }

    #[test]
    fn windows_shortcuts_use_control_modifier() {
        let command = keyboard::Modifiers::CTRL;
        let command_shift = keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT;

        assert!(is_primary_modifier_pressed(
            ShortcutPlatform::Windows,
            command
        ));
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Windows,
                keyboard::Key::Character("f"),
                command,
            ),
            Some(ShortcutAction::OpenDiff)
        );
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Windows,
                keyboard::Key::Character("f"),
                command_shift,
            ),
            Some(ShortcutAction::OpenProject)
        );
    }

    #[test]
    fn shortcut_actions_ignore_wrong_primary_modifier() {
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Mac,
                keyboard::Key::Character("f"),
                keyboard::Modifiers::CTRL,
            ),
            None
        );
        assert_eq!(
            shortcut_action_for_key(
                ShortcutPlatform::Linux,
                keyboard::Key::Character("f"),
                keyboard::Modifiers::LOGO,
            ),
            None
        );
    }

    #[test]
    fn escape_maps_to_close_on_all_platforms() {
        for platform in [
            ShortcutPlatform::Mac,
            ShortcutPlatform::Linux,
            ShortcutPlatform::Windows,
        ] {
            assert_eq!(
                shortcut_action_for_key(
                    platform,
                    keyboard::Key::Named(keyboard::key::Named::Escape),
                    keyboard::Modifiers::default(),
                ),
                Some(ShortcutAction::CloseActive)
            );
        }
    }
}
