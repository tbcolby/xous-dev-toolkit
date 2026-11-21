//! Pure layout math: list scrolling and row geometry.
//!
//! Never hard-code 336x536. A `Framebuffer` canvas starts below the status
//! bar, so its height is smaller than the panel. Always size from
//! `gam.get_canvas_bounds(gid)` (see [`crate::screen::Screen`]).

/// Cursor + scroll state for a vertical list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ListState {
    /// Index of the selected item.
    pub selected: usize,
    /// Index of the first visible item.
    pub top: usize,
}

impl ListState {
    /// Move the selection by `delta` (negative = up), clamped to `0..len`, and
    /// scroll so the selection stays within `visible` rows.
    pub fn step(&mut self, delta: isize, len: usize, visible: usize) {
        if len == 0 {
            *self = ListState::default();
            return;
        }
        let max = (len - 1) as isize;
        let s = (self.selected as isize + delta).clamp(0, max) as usize;
        self.selected = s;
        self.scroll_into_view(len, visible);
    }

    /// Like [`step`](Self::step) but wraps around the ends.
    pub fn step_wrapping(&mut self, delta: isize, len: usize, visible: usize) {
        if len == 0 {
            *self = ListState::default();
            return;
        }
        let l = len as isize;
        self.selected = (((self.selected as isize + delta) % l + l) % l) as usize;
        self.scroll_into_view(len, visible);
    }

    /// Re-clamp after the list changed length (e.g. an item was deleted).
    pub fn clamp(&mut self, len: usize, visible: usize) {
        if len == 0 {
            *self = ListState::default();
            return;
        }
        self.selected = self.selected.min(len - 1);
        self.scroll_into_view(len, visible);
    }

    fn scroll_into_view(&mut self, len: usize, visible: usize) {
        let visible = visible.max(1);
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + visible {
            self.top = self.selected + 1 - visible;
        }
        let max_top = len.saturating_sub(visible);
        self.top = self.top.min(max_top);
    }

    /// Range of item indices to draw.
    pub fn visible_range(&self, len: usize, visible: usize) -> core::ops::Range<usize> {
        let end = (self.top + visible).min(len);
        self.top.min(end)..end
    }
}

/// How many rows of height `row_h` fit in `avail` pixels.
pub fn rows_that_fit(avail: isize, row_h: isize) -> usize {
    if row_h <= 0 || avail <= 0 {
        return 0;
    }
    (avail / row_h) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_clamps_and_scrolls() {
        let mut s = ListState::default();
        s.step(-1, 10, 4);
        assert_eq!(s, ListState { selected: 0, top: 0 });
        for _ in 0..5 {
            s.step(1, 10, 4);
        }
        assert_eq!(s, ListState { selected: 5, top: 2 });
        s.step(100, 10, 4);
        assert_eq!(s, ListState { selected: 9, top: 6 });
        s.step(-7, 10, 4);
        assert_eq!(s, ListState { selected: 2, top: 2 });
    }

    #[test]
    fn wrapping() {
        let mut s = ListState::default();
        s.step_wrapping(-1, 5, 3);
        assert_eq!(s, ListState { selected: 4, top: 2 });
        s.step_wrapping(1, 5, 3);
        assert_eq!(s, ListState { selected: 0, top: 0 });
    }

    #[test]
    fn empty_list() {
        let mut s = ListState { selected: 3, top: 2 };
        s.step(1, 0, 4);
        assert_eq!(s, ListState::default());
        assert_eq!(s.visible_range(0, 4), 0..0);
    }

    #[test]
    fn clamp_after_delete() {
        let mut s = ListState { selected: 9, top: 6 };
        s.clamp(7, 4);
        assert_eq!(s, ListState { selected: 6, top: 3 });
        assert_eq!(s.visible_range(7, 4), 3..7);
    }

    #[test]
    fn short_list_never_scrolls() {
        let mut s = ListState::default();
        s.step(2, 3, 10);
        assert_eq!(s, ListState { selected: 2, top: 0 });
    }

    #[test]
    fn rows_fit() {
        assert_eq!(rows_that_fit(500, 24), 20);
        assert_eq!(rows_that_fit(10, 24), 0);
        assert_eq!(rows_that_fit(100, 0), 0);
    }
}
