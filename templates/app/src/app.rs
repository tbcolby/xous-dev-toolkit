//! Rendering and input for {{display}}. State and rules live in the
//! `{{name}}-core` crate (host-tested); this file only maps keys to core
//! operations, persists, and draws.

use gam::GlyphStyle;
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::screen::Screen;
use precursor_appkit::store::Store;
use {{name_snake}}_core::{LoadError, Saved, Tally};

const STATE_KEY: &str = "state";

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

pub struct App {
    screen: Screen,
    store: Store,
    tally: Tally,
    list: ListState,
    visible_rows: usize,
    seconds: u64,
    dirty: bool,
    /// Set when saved data couldn't be used; blocks saves so it isn't overwritten.
    read_only: bool,
    status: String,
}

impl App {
    pub fn new(screen: Screen, store: Store) -> Self {
        let mut app = Self {
            screen,
            store,
            tally: Tally::new(&["Alpha", "Bravo", "Charlie", "Delta", "Echo"]),
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
        let names: Vec<String> = (0..self.tally.len()).map(|i| self.tally.name(i).to_string()).collect();
        let names: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        self.tally = Tally::new(&names);
        self.read_only = false;
        self.status.clear();
        match self.store.load::<Saved>(STATE_KEY) {
            Ok(Some(s)) => {
                if let Err(LoadError::NewerVersion(v)) = self.tally.from_saved(s) {
                    log::warn!("state version {} is newer than this app", v);
                    self.read_only = true;
                    self.status = "Saved data is from a newer version".into();
                }
            }
            Ok(None) => {}
            Err(e) => {
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
        match self.store.save(STATE_KEY, &self.tally.to_saved()) {
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
            Key::Up => self.list.step(-1, self.tally.len(), self.visible_rows),
            Key::Down => self.list.step(1, self.tally.len(), self.visible_rows),
            Key::Enter => {
                self.tally.bump(self.list.selected);
                self.dirty = true;
            }
            Key::F(1) => {
                self.tally.reset(self.list.selected);
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
        let labels: Vec<String> = (0..self.tally.len()).map(|i| self.tally.label(i)).collect();
        let status_h = if self.status.is_empty() { 0 } else { self.screen.line_height(GlyphStyle::Small) };
        self.visible_rows = self.screen.list(top, bottom - status_h, &labels, &self.list);
        if !self.status.is_empty() {
            self.screen.text(bottom - status_h, GlyphStyle::Small, &self.status);
        }
        self.draw_clock();
        self.screen.flush();
    }
}
