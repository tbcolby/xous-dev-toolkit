//! Drawing helpers on top of GAM.
//!
//! What this module deliberately avoids:
//! * `TextView::invert` — GAM ignores it for app canvases (trust level), so
//!   "white text on a black bar" renders as a blank black bar. Selection is
//!   shown with a border and a `▸` marker instead.
//! * `gam.clear()`, `draw_textview`, `graphics_server::*` — none of these are
//!   app APIs. Everything here goes through `gam::Gam` and `gam::menu::*`.
//! * Hard-coded 336x536 — the size comes from `get_canvas_bounds`.
//!
//! Drawing is buffered by GAM; call [`Screen::flush`] once per frame.

use core::fmt::Write;

use gam::menu::*;
use gam::GlyphStyle;

use crate::layout::{rows_that_fit, ListState};

/// Horizontal padding used by the helpers.
pub const MARGIN: isize = 6;

pub struct Screen {
    gam: gam::Gam,
    gid: Gid,
    size: Point,
}

impl Screen {
    pub fn new(gam: gam::Gam, gid: Gid) -> Self {
        let size = gam.get_canvas_bounds(gid).expect("couldn't get canvas bounds");
        Self { gam, gid, size }
    }

    pub fn gam(&self) -> &gam::Gam { &self.gam }

    pub fn gid(&self) -> Gid { self.gid }

    /// Canvas size in pixels (width = 336 on Precursor; height is less than 536).
    pub fn size(&self) -> Point { self.size }

    /// Re-query the canvas size. Call on redraw after a focus change.
    pub fn refresh_size(&mut self) {
        if let Ok(s) = self.gam.get_canvas_bounds(self.gid) {
            self.size = s;
        }
    }

    /// Height of one line of text in `style`, in pixels.
    pub fn line_height(&self, style: GlyphStyle) -> isize {
        self.gam.glyph_height_hint(style).map(|h| h as isize).unwrap_or(16) + 4
    }

    /// Fill the whole canvas with white.
    pub fn clear(&self) { self.fill(Point::new(0, 0), self.size, PixelColor::Light); }

    /// Fill a rectangle.
    pub fn fill(&self, tl: Point, br: Point, color: PixelColor) {
        let style = DrawStyle { fill_color: Some(color), stroke_color: None, stroke_width: 0 };
        self.gam.draw_rectangle(self.gid, Rectangle::new_with_style(tl, br, style)).ok();
    }

    /// Outline a rectangle.
    pub fn frame(&self, tl: Point, br: Point, width: isize) {
        let style = DrawStyle::new(PixelColor::Light, PixelColor::Dark, width);
        self.gam.draw_rectangle(self.gid, Rectangle::new_with_style(tl, br, style)).ok();
    }

    pub fn line(&self, a: Point, b: Point) {
        let style = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1);
        self.gam.draw_line(self.gid, Line::new_with_style(a, b, style)).ok();
    }

    /// Set or clear one pixel (as a 1x1 rectangle). For many pixels, build a
    /// `GamObjectList` and call `gam().draw_list` instead.
    pub fn pixel(&self, p: Point, on: bool) {
        let c = if on { PixelColor::Dark } else { PixelColor::Light };
        self.fill(p, p, c);
    }

    /// Draw text inside a box. Returns the height actually used.
    pub fn text_in(&self, tl: Point, br: Point, style: GlyphStyle, text: &str) -> isize {
        let mut tv = TextView::new(self.gid, TextBounds::BoundingBox(Rectangle::new(tl, br)));
        tv.style = style;
        tv.clear_area = true;
        tv.draw_border = false;
        tv.margin = Point::new(0, 0);
        write!(tv.text, "{}", text).ok();
        self.gam.post_textview(&mut tv).ok();
        tv.bounds_computed.map(|r| r.br.y - r.tl.y).unwrap_or(br.y - tl.y)
    }

    /// Draw a single line of text at `y`, full width.
    pub fn text(&self, y: isize, style: GlyphStyle, text: &str) -> isize {
        let h = self.line_height(style);
        self.text_in(Point::new(MARGIN, y), Point::new(self.size.x - MARGIN, y + h), style, text);
        h
    }

    /// Draw centred text in a box.
    pub fn text_centered(&self, tl: Point, br: Point, style: GlyphStyle, text: &str) {
        let mut tv = TextView::new(self.gid, TextBounds::CenteredTop(Rectangle::new(tl, br)));
        tv.style = style;
        tv.clear_area = true;
        tv.margin = Point::new(0, 0);
        write!(tv.text, "{}", text).ok();
        self.gam.post_textview(&mut tv).ok();
    }

    /// Title row with a rule under it. Returns the y where content can start.
    pub fn header(&self, title: &str) -> isize {
        let h = self.line_height(GlyphStyle::Bold);
        self.text_in(Point::new(MARGIN, 2), Point::new(self.size.x - MARGIN, 2 + h), GlyphStyle::Bold, title);
        let y = 2 + h + 2;
        self.line(Point::new(0, y), Point::new(self.size.x - 1, y));
        y + 4
    }

    /// Key-hint row pinned to the bottom with a rule above it. Returns its top y
    /// (the bottom of the content area).
    pub fn footer(&self, hint: &str) -> isize {
        let h = self.line_height(GlyphStyle::Small);
        let top = self.size.y - h - 2;
        self.line(Point::new(0, top - 2), Point::new(self.size.x - 1, top - 2));
        self.text_in(Point::new(MARGIN, top), Point::new(self.size.x - MARGIN, self.size.y - 1), GlyphStyle::Small, hint);
        top - 4
    }

    /// Draw a scrolling list between `top` and `bottom`. The selected row gets a
    /// border and a `▸` marker. Returns the number of visible rows, which
    /// callers pass to [`ListState::step`].
    pub fn list<S: AsRef<str>>(&self, top: isize, bottom: isize, items: &[S], state: &ListState) -> usize {
        let row_h = self.line_height(GlyphStyle::Regular) + 4;
        let visible = rows_that_fit(bottom - top, row_h);
        self.fill(Point::new(0, top), Point::new(self.size.x - 1, bottom), PixelColor::Light);
        for (row, idx) in state.visible_range(items.len(), visible).enumerate() {
            let y = top + row as isize * row_h;
            let selected = idx == state.selected;
            let mut label = String::with_capacity(items[idx].as_ref().len() + 4);
            label.push_str(if selected { "▸ " } else { "  " });
            label.push_str(items[idx].as_ref());
            self.text_in(
                Point::new(MARGIN + 2, y + 2),
                Point::new(self.size.x - MARGIN - 2, y + row_h - 2),
                GlyphStyle::Regular,
                &label,
            );
            if selected {
                self.frame(Point::new(MARGIN - 2, y), Point::new(self.size.x - MARGIN + 1, y + row_h - 1), 1);
            }
        }
        if state.top > 0 {
            self.text_in(Point::new(self.size.x - 20, top), Point::new(self.size.x - 2, top + row_h), GlyphStyle::Small, "↑");
        }
        if state.top + visible < items.len() {
            let y = top + visible as isize * row_h - row_h;
            self.text_in(Point::new(self.size.x - 20, y), Point::new(self.size.x - 2, y + row_h), GlyphStyle::Small, "↓");
        }
        visible
    }

    /// Push buffered drawing to the display. Call once per frame.
    /// Fill many rectangles with one GAM message per batch instead of one
    /// blocking call each (QR modules, game boards, cellular automata).
    /// Each item is `(top_left, bottom_right)`, inclusive.
    pub fn fill_rects(&self, rects: impl IntoIterator<Item = (Point, Point)>, color: PixelColor) {
        let style = DrawStyle::new(color, color, 0);
        let mut list = gam::GamObjectList::new(self.gid);
        for (tl, br) in rects {
            let obj = gam::GamObjectType::Rect(Rectangle::new_with_style(tl, br, style));
            if let Err(obj) = list.push(obj) {
                self.gam.draw_list(list).ok();
                list = gam::GamObjectList::new(self.gid);
                list.push(obj).ok();
            }
        }
        self.gam.draw_list(list).ok();
    }

    /// Wrapped text between `y` and `bottom` with the standard margins.
    /// Returns the next free y (with a small gap), or `y` if there's no room.
    pub fn para(&self, y: isize, bottom: isize, style: GlyphStyle, text: &str) -> isize {
        if y >= bottom {
            return y;
        }
        y + self.text_in(Point::new(MARGIN, y), Point::new(self.size.x - MARGIN, bottom), style, text) + 4
    }

    pub fn flush(&self) { self.gam.redraw().ok(); }
}
