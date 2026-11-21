//! App state and rendering. Pure logic lives here so it can be unit-tested;
//! main.rs only routes messages.

use gam::GlyphStyle;
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::screen::Screen;
use precursor_appkit::store::Store;
use serde::{Deserialize, Serialize};

const STATE_KEY: &str = "state";
/// Bump when the stored format changes; old records are migrated in `reload`.
const STATE_VERSION: u32 = 1;

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    version: u32,
    counts: Vec<u32>,
}

pub struct App {
    screen: Screen,
    store: Store,
    items: Vec<&'static str>,
    counts: Vec<u32>,
    list: ListState,
    visible_rows: usize,
    seconds: u64,
    dirty: bool,
    /// Set when saved data couldn't be read; blocks saves so it isn't overwritten.
    read_only: bool,
    status: String,
}

impl App {
    pub fn new(screen: Screen, store: Store) -> Self {
        let items = vec!["Alpha", "Bravo", "Charlie", "Delta", "Echo"];
        let mut app = Self {
            screen,
            store,
            counts: vec![0; items.len()],
            items,
            list: ListState::default(),
            visible_rows: 1,
            seconds: 0,
            dirty: false,
            read_only: false,
            status: String::new(),
        };
        app.reload();
        app
    }

    pub fn screen(&self) -> &Screen { &self.screen }

    /// (Re)load persisted state. Called at start and whenever the set of open
    /// PDDB bases changes, so nothing from a just-locked basis stays on screen.
    pub fn reload(&mut self) {
        match self.store.load::<Saved>(STATE_KEY) {
            Ok(Some(s)) if s.version == STATE_VERSION => {
                self.counts = s.counts;
                self.counts.resize(self.items.len(), 0);
                self.read_only = false;
                self.status.clear();
            }
            Ok(Some(s)) => {
                log::warn!("unknown state version {}; starting fresh without overwriting", s.version);
                self.counts = vec![0; self.items.len()];
                self.read_only = true;
                self.status = "Saved data is from a newer version".into();
            }
            Ok(None) => {
                self.counts = vec![0; self.items.len()];
                self.read_only = false;
                self.status.clear();
            }
            Err(e) => {
                // Corrupt or unreadable: keep it on disk, don't overwrite on next save.
                log::error!("couldn't load state: {:?}", e);
                self.read_only = true;
                self.status = "Couldn't read saved data".into();
            }
        }
        self.dirty = false;
        self.draw();
    }

    fn save(&mut self) {
        if !self.dirty || self.read_only {
            return;
        }
        let s = Saved { version: STATE_VERSION, counts: self.counts.clone() };
        match self.store.save(STATE_KEY, &s) {
            Ok(()) => {
                self.dirty = false;
                self.status.clear();
            }
            Err(e) => {
                log::error!("save failed: {:?}", e);
                self.status = "Save failed".into();
            }
        }
    }

    pub fn on_key(&mut self, key: Key) -> Action {
        match key {
            Key::Up => self.list.step(-1, self.items.len(), self.visible_rows),
            Key::Down => self.list.step(1, self.items.len(), self.visible_rows),
            Key::Enter => {
                self.counts[self.list.selected] = self.counts[self.list.selected].saturating_add(1);
                self.dirty = true;
            }
            Key::F(1) => {
                self.counts[self.list.selected] = 0;
                self.dirty = true;
            }
            k if k.is_back() => {
                self.save();
                return Action::Leave;
            }
            _ => {}
        }
        Action::Stay
    }

    pub fn on_tick(&mut self) {
        self.seconds += 1;
        self.draw_clock();
        self.screen.flush();
    }

    pub fn on_foreground(&mut self) {
        self.screen.refresh_size();
        self.draw();
    }

    /// Save only if something changed. Never rewrite unchanged data on every
    /// focus loss: it wears flash and can write into whichever basis is open.
    pub fn on_background(&mut self) { self.save(); }

    fn draw_clock(&self) {
        let w = self.screen.size().x;
        let h = self.screen.line_height(GlyphStyle::Small);
        let text = format!("{}:{:02}", self.seconds / 60, self.seconds % 60);
        self.screen.text_in(
            gam::menu::Point::new(w - 70, 4),
            gam::menu::Point::new(w - 4, 4 + h),
            GlyphStyle::Small,
            &text,
        );
    }

    pub fn draw(&mut self) {
        self.screen.clear();
        let top = self.screen.header("{{display}}");
        let bottom = self.screen.footer("↑↓ move · ⏎ +1 · F1 reset · ⌫ back");
        let labels: Vec<String> = self
            .items
            .iter()
            .zip(&self.counts)
            .map(|(name, n)| format!("{:<10} {:>5}", name, n))
            .collect();
        let status_h = if self.status.is_empty() { 0 } else { self.screen.line_height(GlyphStyle::Small) };
        self.visible_rows = self.screen.list(top, bottom - status_h, &labels, &self.list);
        if !self.status.is_empty() {
            self.screen.text(bottom - status_h, GlyphStyle::Small, &self.status);
        }
        self.draw_clock();
        self.screen.flush();
    }
}
