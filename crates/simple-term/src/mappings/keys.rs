//! Key mappings for terminal input
//!
//! This module maps GPUUI keystrokes to terminal escape sequences.

use std::borrow::Cow;

use alacritty_terminal::term::TermMode;
use gpui::Keystroke;

#[derive(Debug, PartialEq, Eq)]
enum AlacModifiers {
    None,
    Alt,
    Ctrl,
    Shift,
    CtrlShift,
    Other,
}

impl AlacModifiers {
    fn new(ks: &Keystroke) -> Self {
        match (
            ks.modifiers.alt,
            ks.modifiers.control,
            ks.modifiers.shift,
            ks.modifiers.platform,
        ) {
            (false, false, false, false) => AlacModifiers::None,
            (true, false, false, false) => AlacModifiers::Alt,
            (false, true, false, false) => AlacModifiers::Ctrl,
            (false, false, true, false) => AlacModifiers::Shift,
            (false, true, true, false) => AlacModifiers::CtrlShift,
            _ => AlacModifiers::Other,
        }
    }

    fn any(&self) -> bool {
        match &self {
            AlacModifiers::None => false,
            AlacModifiers::Alt => true,
            AlacModifiers::Ctrl => true,
            AlacModifiers::Shift => true,
            AlacModifiers::CtrlShift => true,
            AlacModifiers::Other => true,
        }
    }
}

/// Convert a GPUUI keystroke to a terminal escape sequence
pub fn to_esc_str(
    keystroke: &Keystroke,
    mode: &TermMode,
    option_as_meta: bool,
) -> Option<Cow<'static, str>> {
    // Normalize macOS Option+Arrow variants to readline word-movement escapes.
    // Depending on input path/layout, these can arrive as:
    // - alt-left/right
    // - alt + final CSI bytes ("D"/"C"/"S"), sometimes with shift also set
    //
    // This runs regardless of APP_CURSOR mode: Option+Arrow is a macOS
    // convention for word movement and should always emit \x1bb / \x1bf.
    // APP_CURSOR only affects plain arrow key encoding (normal vs application).
    if !option_as_meta && !keystroke.modifiers.control && !keystroke.modifiers.platform {
        // Preferred path when Alt modifier is present.
        if keystroke.modifiers.alt {
            match keystroke.key.as_ref() {
                "left" | "D" => return Some(Cow::Borrowed("\x1bb")),
                "right" | "C" | "S" => return Some(Cow::Borrowed("\x1bf")),
                _ => {}
            }
        }

        // Fallback path for some macOS input stacks that emit Option+Arrow as the final CSI
        // byte ("C"/"D"/"S") but drop modifier state, with no character payload.
        if keystroke.key_char.is_none() {
            match keystroke.key.as_ref() {
                "D" => return Some(Cow::Borrowed("\x1bb")),
                "C" | "S" => return Some(Cow::Borrowed("\x1bf")),
                _ => {}
            }
        }
    }

    let modifiers = AlacModifiers::new(keystroke);

    // Manual Bindings including modifiers
    let manual_esc_str: Option<&'static str> = match (keystroke.key.as_ref(), &modifiers) {
        //Basic special keys
        ("tab", AlacModifiers::None) => Some("\x09"),
        ("escape", AlacModifiers::None) => Some("\x1b"),
        ("enter", AlacModifiers::None) => Some("\x0d"),
        ("enter", AlacModifiers::Shift) => Some("\x0a"),
        ("enter", AlacModifiers::Alt) => Some("\x1b\x0d"),
        ("backspace", AlacModifiers::None) => Some("\x7f"),
        //Interesting escape codes
        ("tab", AlacModifiers::Shift) => Some("\x1b[Z"),
        ("backspace", AlacModifiers::Ctrl) => Some("\x08"),
        ("backspace", AlacModifiers::Alt) => Some("\x1b\x7f"),
        ("backspace", AlacModifiers::Shift) => Some("\x7f"),
        ("space", AlacModifiers::Ctrl) => Some("\x00"),
        ("home", AlacModifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => Some("\x1b[1;2H"),
        ("end", AlacModifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => Some("\x1b[1;2F"),
        ("pageup", AlacModifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => {
            Some("\x1b[5;2~")
        }
        ("pagedown", AlacModifiers::Shift) if mode.contains(TermMode::ALT_SCREEN) => {
            Some("\x1b[6;2~")
        }
        ("home", AlacModifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOH"),
        ("home", AlacModifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[H"),
        ("end", AlacModifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOF"),
        ("end", AlacModifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[F"),
        ("up", AlacModifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOA"),
        ("up", AlacModifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[A"),
        ("down", AlacModifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOB"),
        ("down", AlacModifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[B"),
        // Some macOS paths report Option+Arrow as final CSI keycodes without key_char.
        ("D", AlacModifiers::Alt) => Some("\x1bb"),
        ("C" | "S", AlacModifiers::Alt) => Some("\x1bf"),
        // Match common terminal readline behavior for word-wise cursor movement.
        ("right", AlacModifiers::Alt) => Some("\x1bf"),
        ("left", AlacModifiers::Alt) => Some("\x1bb"),
        ("right", AlacModifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOC"),
        ("right", AlacModifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[C"),
        ("left", AlacModifiers::None) if mode.contains(TermMode::APP_CURSOR) => Some("\x1bOD"),
        ("left", AlacModifiers::None) if !mode.contains(TermMode::APP_CURSOR) => Some("\x1b[D"),
        ("back", AlacModifiers::None) => Some("\x7f"),
        ("insert", AlacModifiers::None) => Some("\x1b[2~"),
        ("delete", AlacModifiers::None) => Some("\x1b[3~"),
        ("pageup", AlacModifiers::None) => Some("\x1b[5~"),
        ("pagedown", AlacModifiers::None) => Some("\x1b[6~"),
        ("f1", AlacModifiers::None) => Some("\x1bOP"),
        ("f2", AlacModifiers::None) => Some("\x1bOQ"),
        ("f3", AlacModifiers::None) => Some("\x1bOR"),
        ("f4", AlacModifiers::None) => Some("\x1bOS"),
        ("f5", AlacModifiers::None) => Some("\x1b[15~"),
        ("f6", AlacModifiers::None) => Some("\x1b[17~"),
        ("f7", AlacModifiers::None) => Some("\x1b[18~"),
        ("f8", AlacModifiers::None) => Some("\x1b[19~"),
        ("f9", AlacModifiers::None) => Some("\x1b[20~"),
        ("f10", AlacModifiers::None) => Some("\x1b[21~"),
        ("f11", AlacModifiers::None) => Some("\x1b[23~"),
        ("f12", AlacModifiers::None) => Some("\x1b[24~"),
        ("f13", AlacModifiers::None) => Some("\x1b[25~"),
        ("f14", AlacModifiers::None) => Some("\x1b[26~"),
        ("f15", AlacModifiers::None) => Some("\x1b[28~"),
        ("f16", AlacModifiers::None) => Some("\x1b[29~"),
        ("f17", AlacModifiers::None) => Some("\x1b[31~"),
        ("f18", AlacModifiers::None) => Some("\x1b[32~"),
        ("f19", AlacModifiers::None) => Some("\x1b[33~"),
        ("f20", AlacModifiers::None) => Some("\x1b[34~"),
        //Mappings for caret notation keys
        ("a", AlacModifiers::Ctrl) => Some("\x01"),
        ("A", AlacModifiers::CtrlShift) => Some("\x01"),
        ("b", AlacModifiers::Ctrl) => Some("\x02"),
        ("B", AlacModifiers::CtrlShift) => Some("\x02"),
        ("c", AlacModifiers::Ctrl) => Some("\x03"),
        ("C", AlacModifiers::CtrlShift) => Some("\x03"),
        ("d", AlacModifiers::Ctrl) => Some("\x04"),
        ("D", AlacModifiers::CtrlShift) => Some("\x04"),
        ("e", AlacModifiers::Ctrl) => Some("\x05"),
        ("E", AlacModifiers::CtrlShift) => Some("\x05"),
        ("f", AlacModifiers::Ctrl) => Some("\x06"),
        ("F", AlacModifiers::CtrlShift) => Some("\x06"),
        ("g", AlacModifiers::Ctrl) => Some("\x07"),
        ("G", AlacModifiers::CtrlShift) => Some("\x07"),
        ("h", AlacModifiers::Ctrl) => Some("\x08"),
        ("H", AlacModifiers::CtrlShift) => Some("\x08"),
        ("i", AlacModifiers::Ctrl) => Some("\x09"),
        ("I", AlacModifiers::CtrlShift) => Some("\x09"),
        ("j", AlacModifiers::Ctrl) => Some("\x0a"),
        ("J", AlacModifiers::CtrlShift) => Some("\x0a"),
        ("k", AlacModifiers::Ctrl) => Some("\x0b"),
        ("K", AlacModifiers::CtrlShift) => Some("\x0b"),
        ("l", AlacModifiers::Ctrl) => Some("\x0c"),
        ("L", AlacModifiers::CtrlShift) => Some("\x0c"),
        ("m", AlacModifiers::Ctrl) => Some("\x0d"),
        ("M", AlacModifiers::CtrlShift) => Some("\x0d"),
        ("n", AlacModifiers::Ctrl) => Some("\x0e"),
        ("N", AlacModifiers::CtrlShift) => Some("\x0e"),
        ("o", AlacModifiers::Ctrl) => Some("\x0f"),
        ("O", AlacModifiers::CtrlShift) => Some("\x0f"),
        ("p", AlacModifiers::Ctrl) => Some("\x10"),
        ("P", AlacModifiers::CtrlShift) => Some("\x10"),
        ("q", AlacModifiers::Ctrl) => Some("\x11"),
        ("Q", AlacModifiers::CtrlShift) => Some("\x11"),
        ("r", AlacModifiers::Ctrl) => Some("\x12"),
        ("R", AlacModifiers::CtrlShift) => Some("\x12"),
        ("s", AlacModifiers::Ctrl) => Some("\x13"),
        ("S", AlacModifiers::CtrlShift) => Some("\x13"),
        ("t", AlacModifiers::Ctrl) => Some("\x14"),
        ("T", AlacModifiers::CtrlShift) => Some("\x14"),
        ("u", AlacModifiers::Ctrl) => Some("\x15"),
        ("U", AlacModifiers::CtrlShift) => Some("\x15"),
        ("v", AlacModifiers::Ctrl) => Some("\x16"),
        ("V", AlacModifiers::CtrlShift) => Some("\x16"),
        ("w", AlacModifiers::Ctrl) => Some("\x17"),
        ("W", AlacModifiers::CtrlShift) => Some("\x17"),
        ("x", AlacModifiers::Ctrl) => Some("\x18"),
        ("X", AlacModifiers::CtrlShift) => Some("\x18"),
        ("y", AlacModifiers::Ctrl) => Some("\x19"),
        ("Y", AlacModifiers::CtrlShift) => Some("\x19"),
        ("z", AlacModifiers::Ctrl) => Some("\x1a"),
        ("Z", AlacModifiers::CtrlShift) => Some("\x1a"),
        ("@", AlacModifiers::Ctrl) => Some("\x00"),
        ("[", AlacModifiers::Ctrl) => Some("\x1b"),
        ("\\", AlacModifiers::Ctrl) => Some("\x1c"),
        ("]", AlacModifiers::Ctrl) => Some("\x1d"),
        ("^", AlacModifiers::Ctrl) => Some("\x1e"),
        ("_", AlacModifiers::Ctrl) => Some("\x1f"),
        ("?", AlacModifiers::Ctrl) => Some("\x7f"),
        _ => None,
    };
    if let Some(esc_str) = manual_esc_str {
        return Some(Cow::Borrowed(esc_str));
    }

    // Automated bindings applying modifiers
    if modifiers.any() {
        let modifier_code = modifier_code(keystroke);
        let modified_esc_str = match keystroke.key.as_ref() {
            "up" => Some(format!("\x1b[1;{}A", modifier_code)),
            "down" => Some(format!("\x1b[1;{}B", modifier_code)),
            "right" => Some(format!("\x1b[1;{}C", modifier_code)),
            "left" => Some(format!("\x1b[1;{}D", modifier_code)),
            "f1" => Some(format!("\x1b[1;{}P", modifier_code)),
            "f2" => Some(format!("\x1b[1;{}Q", modifier_code)),
            "f3" => Some(format!("\x1b[1;{}R", modifier_code)),
            "f4" => Some(format!("\x1b[1;{}S", modifier_code)),
            "f5" => Some(format!("\x1b[15;{}~", modifier_code)),
            "f6" => Some(format!("\x1b[17;{}~", modifier_code)),
            "f7" => Some(format!("\x1b[18;{}~", modifier_code)),
            "f8" => Some(format!("\x1b[19;{}~", modifier_code)),
            "f9" => Some(format!("\x1b[20;{}~", modifier_code)),
            "f10" => Some(format!("\x1b[21;{}~", modifier_code)),
            "f11" => Some(format!("\x1b[23;{}~", modifier_code)),
            "f12" => Some(format!("\x1b[24;{}~", modifier_code)),
            "f13" => Some(format!("\x1b[25;{}~", modifier_code)),
            "f14" => Some(format!("\x1b[26;{}~", modifier_code)),
            "f15" => Some(format!("\x1b[28;{}~", modifier_code)),
            "f16" => Some(format!("\x1b[29;{}~", modifier_code)),
            "f17" => Some(format!("\x1b[31;{}~", modifier_code)),
            "f18" => Some(format!("\x1b[32;{}~", modifier_code)),
            "f19" => Some(format!("\x1b[33;{}~", modifier_code)),
            "f20" => Some(format!("\x1b[34;{}~", modifier_code)),
            _ if modifier_code == 2 => None,
            "insert" => Some(format!("\x1b[2;{}~", modifier_code)),
            "pageup" => Some(format!("\x1b[5;{}~", modifier_code)),
            "pagedown" => Some(format!("\x1b[6;{}~", modifier_code)),
            "end" => Some(format!("\x1b[1;{}F", modifier_code)),
            "home" => Some(format!("\x1b[1;{}H", modifier_code)),
            _ => None,
        };
        if let Some(esc_str) = modified_esc_str {
            return Some(Cow::Owned(esc_str));
        }
    }

    if !cfg!(target_os = "macos") || option_as_meta {
        let is_alt_lowercase_ascii = modifiers == AlacModifiers::Alt && keystroke.key.is_ascii();
        let is_alt_uppercase_ascii =
            keystroke.modifiers.alt && keystroke.modifiers.shift && keystroke.key.is_ascii();
        if is_alt_lowercase_ascii || is_alt_uppercase_ascii {
            let key = if is_alt_uppercase_ascii {
                &keystroke.key.to_ascii_uppercase()
            } else {
                &keystroke.key
            };
            return Some(Cow::Owned(format!("\x1b{}", key)));
        }
    }

    None
}

///   Code     Modifiers
/// ---------+---------------------------
///    2     | Shift
///    3     | Alt
///    4     | Shift + Alt
///    5     | Control
///    6     | Shift + Control
///    7     | Alt + Control
///    8     | Shift + Alt + Control
/// ---------+---------------------------
/// from: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-PC-Style-Function-Keys
fn modifier_code(keystroke: &Keystroke) -> u32 {
    let mut modifier_code = 0;
    if keystroke.modifiers.shift {
        modifier_code |= 1;
    }
    if keystroke.modifiers.alt {
        modifier_code |= 1 << 1;
    }
    if keystroke.modifiers.control {
        modifier_code |= 1 << 2;
    }
    modifier_code + 1
}

#[cfg(test)]
mod tests {
    use gpui::Modifiers;

    use super::*;

    #[test]
    fn scroll_keys_are_mode_sensitive() {
        let shift_pageup = Keystroke::parse("shift-pageup").unwrap();
        let shift_pagedown = Keystroke::parse("shift-pagedown").unwrap();
        let shift_home = Keystroke::parse("shift-home").unwrap();
        let shift_end = Keystroke::parse("shift-end").unwrap();

        let none = TermMode::NONE;
        assert_eq!(to_esc_str(&shift_pageup, &none, false), None);
        assert_eq!(to_esc_str(&shift_pagedown, &none, false), None);
        assert_eq!(to_esc_str(&shift_home, &none, false), None);
        assert_eq!(to_esc_str(&shift_end, &none, false), None);

        let alt_screen = TermMode::ALT_SCREEN;
        assert_eq!(
            to_esc_str(&shift_pageup, &alt_screen, false),
            Some("\x1b[5;2~".into())
        );
        assert_eq!(
            to_esc_str(&shift_pagedown, &alt_screen, false),
            Some("\x1b[6;2~".into())
        );
        assert_eq!(
            to_esc_str(&shift_home, &alt_screen, false),
            Some("\x1b[1;2H".into())
        );
        assert_eq!(
            to_esc_str(&shift_end, &alt_screen, false),
            Some("\x1b[1;2F".into())
        );

        let pageup = Keystroke::parse("pageup").unwrap();
        let pagedown = Keystroke::parse("pagedown").unwrap();
        let any = TermMode::ANY;
        assert_eq!(to_esc_str(&pageup, &any, false), Some("\x1b[5~".into()));
        assert_eq!(to_esc_str(&pagedown, &any, false), Some("\x1b[6~".into()));
    }

    #[test]
    fn plain_non_ascii_input_falls_back_to_text_input_path() {
        let ks = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: false,
                shift: false,
                platform: false,
                function: false,
            },
            key: "🖖🏻".to_string(),
            key_char: None,
        };
        assert_eq!(to_esc_str(&ks, &TermMode::NONE, false), None);
    }

    #[test]
    fn cursor_keys_honor_app_cursor_mode() {
        let app_cursor = TermMode::APP_CURSOR;
        let none = TermMode::NONE;

        let up = Keystroke::parse("up").unwrap();
        let down = Keystroke::parse("down").unwrap();
        let left = Keystroke::parse("left").unwrap();
        let right = Keystroke::parse("right").unwrap();

        assert_eq!(to_esc_str(&up, &none, false), Some("\x1b[A".into()));
        assert_eq!(to_esc_str(&down, &none, false), Some("\x1b[B".into()));
        assert_eq!(to_esc_str(&right, &none, false), Some("\x1b[C".into()));
        assert_eq!(to_esc_str(&left, &none, false), Some("\x1b[D".into()));

        assert_eq!(to_esc_str(&up, &app_cursor, false), Some("\x1bOA".into()));
        assert_eq!(to_esc_str(&down, &app_cursor, false), Some("\x1bOB".into()));
        assert_eq!(
            to_esc_str(&right, &app_cursor, false),
            Some("\x1bOC".into())
        );
        assert_eq!(to_esc_str(&left, &app_cursor, false), Some("\x1bOD".into()));
    }

    #[test]
    fn ctrl_and_ctrl_shift_letters_map_to_same_control_codes() {
        let mode = TermMode::ANY;
        for (lower, upper) in ('a'..='z').zip('A'..='Z') {
            assert_eq!(
                to_esc_str(
                    &Keystroke::parse(&format!("ctrl-shift-{}", lower)).unwrap(),
                    &mode,
                    false,
                ),
                to_esc_str(
                    &Keystroke::parse(&format!("ctrl-{}", upper)).unwrap(),
                    &mode,
                    false,
                ),
                "letter {lower}/{upper}",
            );
        }
    }

    #[test]
    fn alt_as_meta_prefixes_ascii_with_escape() {
        for ch in ' '..='~' {
            let ks = Keystroke::parse(&format!("alt-{ch}")).unwrap();
            assert_eq!(
                to_esc_str(&ks, &TermMode::NONE, true).unwrap(),
                format!("\x1b{ch}")
            );
        }
    }

    #[test]
    fn shift_enter_maps_to_line_feed() {
        let shift_enter = Keystroke::parse("shift-enter").unwrap();
        let regular_enter = Keystroke::parse("enter").unwrap();
        let mode = TermMode::NONE;

        assert_eq!(to_esc_str(&shift_enter, &mode, false), Some("\x0a".into()));
        assert_eq!(
            to_esc_str(&regular_enter, &mode, false),
            Some("\x0d".into())
        );
    }

    #[test]
    fn modifier_code_matches_xterm_table() {
        assert_eq!(2, modifier_code(&Keystroke::parse("shift-a").unwrap()));
        assert_eq!(3, modifier_code(&Keystroke::parse("alt-a").unwrap()));
        assert_eq!(4, modifier_code(&Keystroke::parse("shift-alt-a").unwrap()));
        assert_eq!(5, modifier_code(&Keystroke::parse("ctrl-a").unwrap()));
        assert_eq!(6, modifier_code(&Keystroke::parse("shift-ctrl-a").unwrap()));
        assert_eq!(7, modifier_code(&Keystroke::parse("alt-ctrl-a").unwrap()));
        assert_eq!(
            8,
            modifier_code(&Keystroke::parse("shift-ctrl-alt-a").unwrap())
        );
    }

    #[test]
    fn alt_arrow_word_motion_matches_readline_expectations() {
        let none = TermMode::NONE;
        let app_cursor = TermMode::APP_CURSOR;
        let alt_left = Keystroke::parse("alt-left").unwrap();
        let alt_right = Keystroke::parse("alt-right").unwrap();

        assert_eq!(to_esc_str(&alt_left, &none, false), Some("\x1bb".into()));
        assert_eq!(to_esc_str(&alt_right, &none, false), Some("\x1bf".into()));

        // Option+Arrow always sends readline word-movement escapes, even in
        // APP_CURSOR mode. APP_CURSOR only affects plain (unmodified) arrow keys.
        assert_eq!(
            to_esc_str(&alt_left, &app_cursor, false),
            Some("\x1bb".into())
        );
        assert_eq!(
            to_esc_str(&alt_right, &app_cursor, false),
            Some("\x1bf".into())
        );
    }

    #[test]
    fn mac_option_arrow_fallback_keycodes_still_move_by_word() {
        let mode = TermMode::NONE;
        let alt_d = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: false,
                platform: false,
                function: false,
            },
            key: "D".to_string(),
            key_char: None,
        };
        let alt_s = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: false,
                platform: false,
                function: false,
            },
            key: "S".to_string(),
            key_char: None,
        };
        let alt_c = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: false,
                platform: false,
                function: false,
            },
            key: "C".to_string(),
            key_char: None,
        };

        assert_eq!(to_esc_str(&alt_d, &mode, false), Some("\x1bb".into()));
        assert_eq!(to_esc_str(&alt_s, &mode, false), Some("\x1bf".into()));
        assert_eq!(to_esc_str(&alt_c, &mode, false), Some("\x1bf".into()));
    }

    #[test]
    fn mac_option_arrow_fallback_keycodes_with_character_data_still_move_by_word() {
        let mode = TermMode::NONE;
        let alt_d = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: false,
                platform: false,
                function: false,
            },
            key: "D".to_string(),
            key_char: Some("D".to_string()),
        };
        let alt_s = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: false,
                platform: false,
                function: false,
            },
            key: "S".to_string(),
            key_char: Some("S".to_string()),
        };
        let alt_c = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: false,
                platform: false,
                function: false,
            },
            key: "C".to_string(),
            key_char: Some("C".to_string()),
        };

        assert_eq!(to_esc_str(&alt_d, &mode, false), Some("\x1bb".into()));
        assert_eq!(to_esc_str(&alt_s, &mode, false), Some("\x1bf".into()));
        assert_eq!(to_esc_str(&alt_c, &mode, false), Some("\x1bf".into()));
    }

    #[test]
    fn mac_option_arrow_fallback_keycodes_with_shift_modifier_still_move_by_word() {
        let mode = TermMode::NONE;
        let alt_shift_d = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: true,
                platform: false,
                function: false,
            },
            key: "D".to_string(),
            key_char: Some("D".to_string()),
        };
        let alt_shift_c = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: true,
                shift: true,
                platform: false,
                function: false,
            },
            key: "C".to_string(),
            key_char: Some("C".to_string()),
        };

        assert_eq!(to_esc_str(&alt_shift_d, &mode, false), Some("\x1bb".into()));
        assert_eq!(to_esc_str(&alt_shift_c, &mode, false), Some("\x1bf".into()));
    }

    #[test]
    fn mac_option_arrow_fallback_keycodes_without_alt_modifier_still_move_by_word_when_non_textual()
    {
        let mode = TermMode::NONE;
        let d = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: false,
                shift: false,
                platform: false,
                function: false,
            },
            key: "D".to_string(),
            key_char: None,
        };
        let c = Keystroke {
            modifiers: Modifiers {
                control: false,
                alt: false,
                shift: false,
                platform: false,
                function: false,
            },
            key: "C".to_string(),
            key_char: None,
        };

        assert_eq!(to_esc_str(&d, &mode, false), Some("\x1bb".into()));
        assert_eq!(to_esc_str(&c, &mode, false), Some("\x1bf".into()));
    }
}
