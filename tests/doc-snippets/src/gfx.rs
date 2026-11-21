//! graphics.md snippets
use std::fmt::Write;

use gam::menu::*; // Point, Rectangle, DrawStyle, TextView, Gid, ... (re-exported ux_api::minigfx)
use gam::{Gam, GamObjectList, GamObjectType, GlyphStyle, UxRegistration};

fn primitives(gam: &Gam, gid: Gid) -> Result<(), xous::Error> {
    // ---- Colors and Styles ----
    // Filled black
    let filled_dark = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1);
    // Filled white
    let filled_light = DrawStyle::new(PixelColor::Light, PixelColor::Light, 1);
    // Black outline only
    let outline_dark = DrawStyle {
        fill_color: None,
        stroke_color: Some(PixelColor::Dark),
        stroke_width: 2, // isize, not u16!
    };
    // White outline only
    let outline_light = DrawStyle { fill_color: None, stroke_color: Some(PixelColor::Light), stroke_width: 1 };

    // ---- Rectangle ----
    // Two-point constructor
    let rect = Rectangle::new(
        Point::new(10, 10),  // top-left
        Point::new(100, 50), // bottom-right
    );
    // Coordinate constructor
    let rect = Rectangle::new_coords(10, 10, 100, 50);
    // With style
    let rect = Rectangle::new_with_style(Point::new(10, 10), Point::new(100, 50), filled_dark);
    gam.draw_rectangle(gid, rect)?;

    // ---- Circle ----
    let circle = Circle::new_with_style(
        Point::new(168, 268), // center
        30,                   // radius (isize!)
        filled_dark,
    );
    gam.draw_circle(gid, circle)?;

    // ---- Line ----
    let line = Line::new_with_style(
        Point::new(0, 0),
        Point::new(100, 100),
        DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1),
    );
    gam.draw_line(gid, line)?;

    // ---- Rounded Rectangle ----
    let mut rrect = RoundedRectangle::new(
        Rectangle::new(Point::new(10, 10), Point::new(100, 50)),
        8, // corner radius
    );
    rrect.border.style = outline_dark;
    gam.draw_rounded_rectangle(gid, rrect)?;

    // ---- Batch Drawing ----
    let mut draw_list = GamObjectList::new(gid);
    draw_list.push(GamObjectType::Circ(circle)).unwrap();
    draw_list.push(GamObjectType::Rect(rect)).unwrap();
    draw_list.push(GamObjectType::Line(line)).unwrap();
    gam.draw_list(draw_list)?;

    // ---- Haptic ----
    gam.set_vibe(true)?; // Enable
    gam.set_vibe(false)?; // Disable
    Ok(())
}

// ---- TextView Basics ----
fn textview_basics(gam: &Gam, gid: Gid) -> Result<(), xous::Error> {
    let mut tv = TextView::new(gid, TextBounds::BoundingBox(Rectangle::new_coords(10, 10, 326, 520)));

    // Style configuration
    tv.style = GlyphStyle::Regular;
    tv.draw_border = true;
    tv.border_width = 1; // u16
    tv.rounded_border = Some(3); // Option<u16>
    tv.clear_area = true;
    tv.margin = Point::new(4, 4);
    // tv.invert is NOT available to apps: the GAM refuses to render inverted text
    // from canvases below the password-modal trust level.

    // Write text
    write!(tv.text, "Hello, Precursor!").unwrap();

    // Render
    gam.post_textview(&mut tv)?;
    Ok(())
}

fn bounds_modes(x0: isize, y0: isize, x1: isize, y1: isize, x: isize, y: isize, max_width: u16) {
    // Fixed bounding box
    let _ = TextBounds::BoundingBox(Rectangle::new_coords(x0, y0, x1, y1));
    // Grows from a corner (max_width is u16)
    let _ = TextBounds::GrowableFromTl(Point::new(x, y), max_width); // top-left
    let _ = TextBounds::GrowableFromBr(Point::new(x, y), max_width); // bottom-right
    let _ = TextBounds::GrowableFromBl(Point::new(x, y), max_width); // bottom-left
    let _ = TextBounds::GrowableFromTr(Point::new(x, y), max_width); // top-right
    // Centered in a rectangle
    let _ = TextBounds::CenteredTop(Rectangle::new_coords(x0, y0, x1, y1));
    let _ = TextBounds::CenteredBot(Rectangle::new_coords(x0, y0, x1, y1));
}

// ---- Standard Dimensions ----
const SCREEN_WIDTH: isize = 336;
const SCREEN_HEIGHT: isize = 536;
const HEADER_HEIGHT: isize = 30;
const FOOTER_HEIGHT: isize = 40;
const CONTENT_TOP: isize = HEADER_HEIGHT;
const CONTENT_BOTTOM: isize = SCREEN_HEIGHT - FOOTER_HEIGHT;
const MARGIN: isize = 8;

// ---- Screen Clear ----
fn clear_screen(gam: &Gam, gid: Gid, screensize: Point) {
    gam.draw_rectangle(
        gid,
        Rectangle::new_with_style(
            Point::new(0, 0),
            screensize,
            DrawStyle { fill_color: Some(PixelColor::Light), stroke_color: None, stroke_width: 0 },
        ),
    )
    .expect("can't clear");
}

// ---- Header Bar (no inverted text: light bar, bold title, rule underneath) ----
fn draw_header(gam: &Gam, gid: Gid, title: &str) {
    let mut tv = TextView::new(gid, TextBounds::BoundingBox(Rectangle::new_coords(8, 4, 328, 28)));
    tv.style = GlyphStyle::Bold;
    tv.clear_area = true;
    write!(tv.text, "{}", title).unwrap();
    gam.post_textview(&mut tv).ok();

    // 2px rule under the header
    gam.draw_line(
        gid,
        Line::new_with_style(Point::new(0, 29), Point::new(336, 29), DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 2)),
    )
    .ok();
}

// ---- Selection Highlight (outline, not a filled/inverted bar) ----
fn draw_selection(gam: &Gam, gid: Gid, y: isize, height: isize) {
    gam.draw_rounded_rectangle(
        gid,
        RoundedRectangle::new(
            Rectangle::new_with_style(
                Point::new(2, y),
                Point::new(334, y + height),
                DrawStyle { fill_color: None, stroke_color: Some(PixelColor::Dark), stroke_width: 2 },
            ),
            4,
        ),
    )
    .ok();
}

// ---- Progress Bar ----
fn draw_progress_bar(gam: &Gam, gid: Gid, x: isize, y: isize, width: isize, height: isize, progress: f32) {
    // Outline
    gam.draw_rectangle(
        gid,
        Rectangle::new_with_style(
            Point::new(x, y),
            Point::new(x + width, y + height),
            DrawStyle { fill_color: None, stroke_color: Some(PixelColor::Dark), stroke_width: 1 },
        ),
    )
    .ok();

    // Fill
    let fill_width = ((width - 2) as f32 * progress.clamp(0.0, 1.0)) as isize;
    if fill_width > 0 {
        gam.draw_rectangle(
            gid,
            Rectangle::new_with_style(
                Point::new(x + 1, y + 1),
                Point::new(x + 1 + fill_width, y + height - 1),
                DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 1),
            ),
        )
        .ok();
    }
}

// ---- List Item ----
fn draw_list_item(gam: &Gam, gid: Gid, y: isize, text: &str, selected: bool) {
    let height: isize = 24;

    let mut tv = TextView::new(gid, TextBounds::BoundingBox(Rectangle::new_coords(8, y + 4, 328, y + height - 4)));
    tv.style = if selected { GlyphStyle::Bold } else { GlyphStyle::Regular };
    write!(tv.text, "{}{}", if selected { "> " } else { "  " }, text).unwrap();
    gam.post_textview(&mut tv).ok();

    if selected {
        draw_selection(gam, gid, y, height); // outline after text so clear_area doesn't erase it
    }
}

// ---- Redraw Pattern ----
struct Ui;
impl Ui {
    fn draw_header(&self, gam: &Gam, gid: Gid) {}
    fn draw_content(&self, gam: &Gam, gid: Gid) {}
    fn draw_footer(&self, gam: &Gam, gid: Gid) {}

    fn redraw(&self, gam: &Gam, gid: Gid, screensize: Point) {
        // 1. Clear
        clear_screen(gam, gid, screensize);

        // 2. Draw UI elements
        self.draw_header(gam, gid);
        self.draw_content(gam, gid);
        self.draw_footer(gam, gid);

        // 3. CRITICAL: Commit to screen
        gam.redraw().unwrap();
    }
}

// ---- Glyph Height Hint / Pre-flight Bounds ----
fn layout_helpers(gam: &Gam, gid: Gid, long_text: &str) -> Result<(), xous::Error> {
    let height: usize = gam.glyph_height_hint(GlyphStyle::Regular)?;

    let mut tv = TextView::new(gid, TextBounds::BoundingBox(Rectangle::new_coords(10, 10, 326, 520)));
    write!(tv.text, "{}", long_text).unwrap();

    // Compute bounds without drawing
    gam.bounds_compute_textview(&mut tv)?;

    // overflow is Option<bool>, set by the compute pass
    if tv.overflow == Some(true) {
        log::warn!("Text overflows bounds, need scrolling/truncation");
    }
    // tv.bounds_computed: Option<Rectangle>; tv.cursor: end position after text
    Ok(())
}

// ---- Scrolling ----
struct ScrollState {
    offset: usize,  // First visible item index
    cursor: usize,  // Selected item index
    total: usize,   // Total item count
    visible: usize, // Items that fit on screen
}

impl ScrollState {
    fn scroll_down(&mut self) {
        if self.cursor < self.total.saturating_sub(1) {
            self.cursor += 1;
            if self.cursor >= self.offset + self.visible {
                self.offset = self.cursor - self.visible + 1;
            }
        }
    }

    fn scroll_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            if self.cursor < self.offset {
                self.offset = self.cursor;
            }
        }
    }

    fn visible_range(&self) -> std::ops::Range<usize> {
        self.offset..std::cmp::min(self.offset + self.visible, self.total)
    }
}

// ---- Advanced Modals ----
fn do_work(_i: u32) {}

fn modals_demo(xns: &xous_names::XousNames) -> Result<(), xous::Error> {
    let modals = modals::Modals::new(xns).unwrap();

    // Progress bar: start (blocking, takes focus)
    modals.start_progress("Syncing data...", 0, 100, 0)?;
    for i in 0..100u32 {
        do_work(i);
        if modals.update_progress(i).is_err() {
            xous::yield_slice(); // queue full, yield
        }
    }
    modals.finish_progress()?;

    // Slider (range input): title, min, max, initial, step
    let value: u32 = modals.slider("Set brightness:", 0, 100, 50, 5)?;

    // Checkbox list (multi-select)
    modals.add_stateful_list_item(false, "Option A")?;
    modals.add_stateful_list_item(true, "Option B")?; // Pre-checked
    modals.add_stateful_list_item(false, "Option C")?;
    let checked: Vec<String> = modals.get_checkbox("Select features:")?;

    // Radio buttons (single-select)
    modals.add_list_item("Mode A")?;
    modals.add_list_item("Mode B")?;
    modals.add_list_item("Mode C")?;
    let choice: String = modals.get_radiobutton("Select mode:")?;

    // Dynamic notification (long-lived, updatable, non-blocking)
    modals.dynamic_notification(Some("Connecting..."), Some("Please wait"))?;
    modals.dynamic_notification_update(Some("Authenticating..."), None)?;
    modals.dynamic_notification_close()?;

    // Notification with QR code; blocks until dismissed
    modals.show_notification("Scan to visit:\nhttps://example.com", Some("https://example.com"))?;
    Ok(())
}

// ---- Text Input with Validation ----
fn validate_username(input: &gam::TextEntryPayload) -> Option<gam::ValidatorErr> {
    if input.as_str().len() < 3 {
        Some(String::from("Min 3 characters")) // ValidatorErr is a String alias
    } else {
        None // Valid
    }
}

fn text_entry(modals: &modals::Modals) -> Result<(), xous::Error> {
    let result = modals
        .alert_builder("Enter credentials:")
        .field(Some("username".into()), Some(validate_username))
        .field(Some("password".into()), None)
        .field_placeholder_persist(Some("notes (optional)".into()), None) // Placeholder persists
        .set_growable() // Last field can expand
        .build()?;

    for (i, entry) in result.content().iter().enumerate() {
        log::info!("Field {}: {}", i, entry.as_str());
    }
    Ok(())
}
