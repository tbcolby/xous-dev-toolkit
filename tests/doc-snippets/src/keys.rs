//! STANDARDS.md section 1 key constants (what apps actually receive via rawkeys)
pub const KEY_UP: char = '\u{2191}'; // ↑
pub const KEY_DOWN: char = '\u{2193}'; // ↓
pub const KEY_LEFT: char = '\u{2190}'; // ←
pub const KEY_RIGHT: char = '\u{2192}'; // →
pub const KEY_ENTER: char = '\r'; // 0x0D
pub const KEY_BACKSPACE: char = '\u{0008}';
pub const KEY_SPACE: char = ' ';
pub const KEY_F1: char = '\u{0011}';
pub const KEY_F2: char = '\u{0012}';
pub const KEY_F3: char = '\u{0013}';
pub const KEY_F4: char = '\u{0014}';
// No KEY_MENU: '∴' (U+2234) is consumed by the GAM (opens the main menu) and never reaches apps.
// No KEY_ESC: no Precursor key produces 0x1B.

/// Decode the four scalar args of a Rawkeys message; 0 = no key.
pub fn decode_rawkeys(k1: usize, k2: usize, k3: usize, k4: usize) -> impl Iterator<Item = char> {
    [k1, k2, k3, k4].into_iter().filter_map(|k| char::from_u32(k as u32)).filter(|&c| c != '\u{0}')
}
