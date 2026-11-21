//! Key codes as Xous actually delivers them.
//!
//! Source of truth: `services/keyboard/src/mappings/qwerty.rs` and
//! `services/gam/src/contexts.rs` in xous-core.
//!
//! Rules that every app must follow:
//! * Arrows arrive as U+2190..U+2193. Apple PUA codes (U+F700..) never arrive.
//! * `∴` (U+2234, the menu key / D-pad centre) is consumed by GAM to open the
//!   main menu whenever an app has focus. **Apps never receive it.** Do not
//!   bind Back, Quit, or Menu to it.
//! * There is no Escape key. `0x1B` never arrives from the keyboard.
//! * Enter is `'\r'` (0x0D), Backspace is 0x08, F1..F4 are 0x11..0x14.
//! * Raw-key messages carry up to four chars; unused slots are `'\0'`.

/// Left arrow, U+2190.
pub const LEFT: char = '\u{2190}';
/// Up arrow, U+2191.
pub const UP: char = '\u{2191}';
/// Right arrow, U+2192.
pub const RIGHT: char = '\u{2192}';
/// Down arrow, U+2193.
pub const DOWN: char = '\u{2193}';
/// Enter / carriage return.
pub const ENTER: char = '\r';
/// Backspace.
pub const BACKSPACE: char = '\u{0008}';
/// F1 (DC1). Also the leftmost of the four keys above the display.
pub const F1: char = '\u{0011}';
/// F2 (DC2).
pub const F2: char = '\u{0012}';
/// F3 (DC3).
pub const F3: char = '\u{0013}';
/// F4 (DC4). Holding it is also a power-key gesture.
pub const F4: char = '\u{0014}';
/// The menu key. Listed only so code can document that it is never received.
pub const MENU_NEVER_DELIVERED: char = '\u{2234}';

/// A decoded key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Enter,
    Backspace,
    /// F1..F4 as 1..4.
    F(u8),
    /// Any other printable character, including space and emoji.
    Char(char),
}

impl Key {
    /// Decode one char. Returns `None` for `'\0'` padding and for control
    /// characters that apps should ignore (shift-in/out, etc.).
    pub fn from_char(c: char) -> Option<Key> {
        Some(match c {
            LEFT => Key::Left,
            RIGHT => Key::Right,
            UP => Key::Up,
            DOWN => Key::Down,
            ENTER | '\n' => Key::Enter,
            BACKSPACE | '\u{007f}' => Key::Backspace,
            F1 => Key::F(1),
            F2 => Key::F(2),
            F3 => Key::F(3),
            F4 => Key::F(4),
            c if c.is_control() => return None,
            c => Key::Char(c),
        })
    }

    /// True for keys that should act as "back / cancel" in a non-text screen.
    /// Backspace and F4 by convention; never `∴` (see module docs).
    pub fn is_back(&self) -> bool { matches!(self, Key::Backspace | Key::F(4)) }

    /// True for "confirm / select".
    pub fn is_select(&self) -> bool { matches!(self, Key::Enter) }
}

/// Decode the four scalar arguments of a GAM raw-keys message.
///
/// ```ignore
/// Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
///     for key in precursor_appkit::keys::decode_rawkeys([k1, k2, k3, k4]) { app.on_key(key); }
/// }),
/// ```
pub fn decode_rawkeys(args: [usize; 4]) -> impl Iterator<Item = Key> {
    args.into_iter().filter_map(|a| char::from_u32(a as u32)).filter_map(Key::from_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_are_unicode_arrows() {
        assert_eq!(Key::from_char('←'), Some(Key::Left));
        assert_eq!(Key::from_char('↑'), Some(Key::Up));
        assert_eq!(Key::from_char('→'), Some(Key::Right));
        assert_eq!(Key::from_char('↓'), Some(Key::Down));
        // Apple PUA must not be treated as arrows
        assert_eq!(Key::from_char('\u{F700}'), Some(Key::Char('\u{F700}')));
    }

    #[test]
    fn function_keys_are_dc1_to_dc4() {
        assert_eq!(Key::from_char('\u{11}'), Some(Key::F(1)));
        assert_eq!(Key::from_char('\u{14}'), Some(Key::F(4)));
        assert!(Key::F(4).is_back());
    }

    #[test]
    fn padding_and_shift_controls_are_dropped() {
        let keys: Vec<Key> = decode_rawkeys(['a' as usize, 0, 0x0f, 0x0e]).collect();
        assert_eq!(keys, vec![Key::Char('a')]);
    }

    #[test]
    fn enter_and_backspace() {
        assert_eq!(Key::from_char('\r'), Some(Key::Enter));
        assert_eq!(Key::from_char('\u{8}'), Some(Key::Backspace));
        assert!(Key::Enter.is_select());
    }

    #[test]
    fn emoji_is_a_char() {
        assert_eq!(Key::from_char('😊'), Some(Key::Char('😊')));
    }

    #[test]
    fn invalid_scalar_is_skipped() {
        let keys: Vec<Key> = decode_rawkeys([0xD800, 'x' as usize, 0, 0]).collect();
        assert_eq!(keys, vec![Key::Char('x')]);
    }
}
