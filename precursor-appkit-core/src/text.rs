//! Text editing and layout that several apps re-implemented (and got wrong):
//! a UTF-8-safe single-line field, word wrapping, and paging long text.
//!
//! All positions are in `char`s, never bytes, so a cursor can't land inside a
//! multi-byte character (the bug in the original Writer and QR apps).

/// A single-line text field with a cursor. Optional `max_chars` limit.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct TextField {
    text: String,
    /// Cursor position in chars, `0..=len`.
    cursor: usize,
    max_chars: Option<usize>,
}

impl core::fmt::Debug for TextField {
    // Fields often hold secrets or personal data; don't leak them into logs.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "TextField {{ {} chars, cursor {} }}", self.len(), self.cursor)
    }
}

impl TextField {
    pub fn new() -> Self { Self::default() }

    pub fn with_max(max_chars: usize) -> Self { Self { max_chars: Some(max_chars), ..Self::default() } }

    /// Start with `text`, cursor at the end. Truncated to `max_chars` if set.
    pub fn with_text(mut self, text: &str) -> Self {
        self.set(text);
        self
    }

    pub fn set(&mut self, text: &str) {
        self.text = match self.max_chars {
            Some(m) => text.chars().take(m).collect(),
            None => text.to_string(),
        };
        self.cursor = self.len();
    }

    pub fn as_str(&self) -> &str { &self.text }

    pub fn len(&self) -> usize { self.text.chars().count() }

    pub fn is_empty(&self) -> bool { self.text.is_empty() }

    pub fn cursor(&self) -> usize { self.cursor }

    fn byte_at(&self, char_idx: usize) -> usize {
        self.text.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(self.text.len())
    }

    /// Insert at the cursor. Control characters are ignored; returns false if
    /// rejected (control char or field full).
    pub fn insert(&mut self, c: char) -> bool {
        if c.is_control() || self.max_chars.is_some_and(|m| self.len() >= m) {
            return false;
        }
        let b = self.byte_at(self.cursor);
        self.text.insert(b, c);
        self.cursor += 1;
        true
    }

    /// Delete the char before the cursor. Returns false if at the start.
    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        let b = self.byte_at(self.cursor);
        self.text.remove(b);
        true
    }

    /// Delete the char under the cursor. Returns false if at the end.
    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.len() {
            return false;
        }
        let b = self.byte_at(self.cursor);
        self.text.remove(b);
        true
    }

    pub fn left(&mut self) { self.cursor = self.cursor.saturating_sub(1); }

    pub fn right(&mut self) { self.cursor = (self.cursor + 1).min(self.len()); }

    pub fn home(&mut self) { self.cursor = 0; }

    pub fn end(&mut self) { self.cursor = self.len(); }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Text with a visible cursor marker inserted (e.g. `'|'`), for drawing.
    pub fn display_with_cursor(&self, marker: char) -> String {
        let mut s = self.text.clone();
        s.insert(self.byte_at(self.cursor), marker);
        s
    }

    /// Take the text out (e.g. on submit), leaving the field empty.
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        core::mem::take(&mut self.text)
    }
}

/// Greedy word wrap to at most `width` chars per line. Words longer than
/// `width` are split. Existing newlines are kept. `width == 0` is treated as 1.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        let mut line_len = 0;
        for word in para.split(' ').filter(|w| !w.is_empty()) {
            let mut word: Vec<char> = word.chars().collect();
            while word.len() > width {
                if line_len > 0 {
                    out.push(core::mem::take(&mut line));
                    line_len = 0;
                }
                out.push(word.drain(..width).collect());
            }
            let wl = word.len();
            if wl == 0 {
                continue;
            }
            if line_len > 0 && line_len + 1 + wl > width {
                out.push(core::mem::take(&mut line));
                line_len = 0;
            }
            if line_len > 0 {
                line.push(' ');
                line_len += 1;
            }
            line.extend(word);
            line_len += wl;
        }
        out.push(line);
    }
    out
}

/// Pages through a list of lines, `per_page` at a time. Never truncates
/// silently: [`Pager::label`] tells the user where they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pager {
    total: usize,
    per_page: usize,
    page: usize,
}

impl Pager {
    pub fn new(total: usize, per_page: usize) -> Self { Self { total, per_page: per_page.max(1), page: 0 } }

    pub fn pages(&self) -> usize { self.total.div_ceil(self.per_page).max(1) }

    pub fn page(&self) -> usize { self.page }

    /// Index range of the lines on the current page.
    pub fn range(&self) -> core::ops::Range<usize> {
        let start = (self.page * self.per_page).min(self.total);
        start..(start + self.per_page).min(self.total)
    }

    pub fn next_page(&mut self) -> bool {
        if self.page + 1 < self.pages() {
            self.page += 1;
            true
        } else {
            false
        }
    }

    pub fn prev_page(&mut self) -> bool {
        if self.page > 0 {
            self.page -= 1;
            true
        } else {
            false
        }
    }

    /// Change page size (e.g. after the canvas size is known), keeping the
    /// first visible line on screen.
    pub fn resize(&mut self, total: usize, per_page: usize) {
        let first = self.range().start;
        self.total = total;
        self.per_page = per_page.max(1);
        self.page = (first / self.per_page).min(self.pages() - 1);
    }

    /// "Page 2/5", or empty when everything fits.
    pub fn label(&self) -> String {
        if self.pages() <= 1 {
            String::new()
        } else {
            format!("Page {}/{}", self.page + 1, self.pages())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_never_splits_multibyte_chars() {
        let mut f = TextField::new().with_text("héllo→");
        assert_eq!(f.len(), 6);
        f.left();
        f.left();
        assert!(f.backspace()); // removes 'l'
        assert_eq!(f.as_str(), "hélo→");
        f.home();
        f.right();
        assert!(f.delete()); // removes 'é'
        assert_eq!(f.as_str(), "hlo→");
        f.end();
        f.insert('ü');
        assert_eq!(f.as_str(), "hlo→ü");
        assert_eq!(f.display_with_cursor('|'), "hlo→ü|");
    }

    #[test]
    fn edges_and_limits() {
        let mut f = TextField::with_max(3);
        assert!(!f.backspace());
        assert!(!f.delete());
        f.left();
        assert_eq!(f.cursor(), 0);
        assert!(f.insert('a') && f.insert('b') && f.insert('c'));
        assert!(!f.insert('d'), "max_chars");
        assert!(!f.insert('\r') && !f.insert('\u{8}'), "control chars rejected");
        f.right();
        assert_eq!(f.cursor(), 3);
        assert_eq!(f.take(), "abc");
        assert!(f.is_empty() && f.cursor() == 0);
        assert_eq!(TextField::with_max(2).with_text("xyz").as_str(), "xy");
    }

    #[test]
    fn debug_does_not_leak_contents() {
        let f = TextField::new().with_text("hunter2");
        assert!(!format!("{f:?}").contains("hunter2"));
    }

    #[test]
    fn wrap_basic_long_words_and_newlines() {
        assert_eq!(wrap("the quick brown fox", 10), ["the quick", "brown fox"]);
        assert_eq!(wrap("abcdefghij12", 5), ["abcde", "fghij", "12"]);
        assert_eq!(wrap("a\n\nb", 5), ["a", "", "b"]);
        assert_eq!(wrap("", 5), [""]);
        assert_eq!(wrap("ab  cd", 0), ["a", "b", "c", "d"]);
        assert_eq!(wrap("x verylongword y", 4), ["x", "very", "long", "word", "y"]);
        for line in wrap("€€€ €€ €€€€€ €", 3) {
            assert!(line.chars().count() <= 3);
        }
    }

    #[test]
    fn pager_ranges_labels_and_resize() {
        let mut p = Pager::new(10, 4);
        assert_eq!((p.pages(), p.range()), (3, 0..4));
        assert!(p.next_page() && p.next_page());
        assert_eq!(p.range(), 8..10);
        assert!(!p.next_page());
        assert_eq!(p.label(), "Page 3/3");
        p.resize(10, 5); // first visible line 8 -> page 1 (5..10)
        assert_eq!(p.range(), 5..10);
        assert!(p.prev_page() && !p.prev_page());
        assert_eq!(Pager::new(3, 10).label(), "");
        assert_eq!(Pager::new(0, 10).range(), 0..0);
    }
}
