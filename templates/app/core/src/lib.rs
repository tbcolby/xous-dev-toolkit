//! Pure state for {{display}}. Everything that can be tested without a
//! device lives here; the app crate only draws and routes messages.

use serde::{Deserialize, Serialize};

/// Bump when the stored format changes, and migrate in [`Tally::from_saved`].
pub const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub counts: Vec<u32>,
}

/// Why saved data couldn't be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Written by a newer version of the app; don't overwrite it.
    NewerVersion(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tally {
    names: Vec<String>,
    counts: Vec<u32>,
}

impl Tally {
    pub fn new(names: &[&str]) -> Self {
        Self { names: names.iter().map(|s| s.to_string()).collect(), counts: vec![0; names.len()] }
    }

    pub fn len(&self) -> usize { self.names.len() }

    pub fn is_empty(&self) -> bool { self.names.is_empty() }

    pub fn name(&self, i: usize) -> &str { &self.names[i] }

    pub fn count(&self, i: usize) -> u32 { self.counts[i] }

    /// Increment, saturating at u32::MAX.
    pub fn bump(&mut self, i: usize) {
        if let Some(c) = self.counts.get_mut(i) {
            *c = c.saturating_add(1);
        }
    }

    pub fn reset(&mut self, i: usize) {
        if let Some(c) = self.counts.get_mut(i) {
            *c = 0;
        }
    }

    pub fn to_saved(&self) -> Saved { Saved { version: STATE_VERSION, counts: self.counts.clone() } }

    /// Apply saved data. Extra counts are dropped, missing ones are zero.
    pub fn from_saved(&mut self, s: Saved) -> Result<(), LoadError> {
        if s.version > STATE_VERSION {
            return Err(LoadError::NewerVersion(s.version));
        }
        self.counts = s.counts;
        self.counts.resize(self.names.len(), 0);
        Ok(())
    }

    /// One display line per item.
    pub fn label(&self, i: usize) -> String { format!("{:<10} {:>5}", self.names[i], self.counts[i]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_saturates() {
        let mut t = Tally::new(&["a"]);
        t.from_saved(Saved { version: 1, counts: vec![u32::MAX] }).unwrap();
        t.bump(0);
        assert_eq!(t.count(0), u32::MAX);
    }

    #[test]
    fn out_of_range_is_ignored() {
        let mut t = Tally::new(&["a"]);
        t.bump(5);
        t.reset(5);
        assert_eq!(t.count(0), 0);
    }

    #[test]
    fn roundtrip_through_json() {
        let mut t = Tally::new(&["a", "b"]);
        t.bump(1);
        let json = serde_json::to_string(&t.to_saved()).unwrap();
        let mut u = Tally::new(&["a", "b"]);
        u.from_saved(serde_json::from_str(&json).unwrap()).unwrap();
        assert_eq!(t, u);
    }

    #[test]
    fn resizes_to_current_items() {
        let mut t = Tally::new(&["a", "b", "c"]);
        t.from_saved(Saved { version: 1, counts: vec![4] }).unwrap();
        assert_eq!((t.count(0), t.count(2)), (4, 0));
    }

    #[test]
    fn refuses_newer_version() {
        let mut t = Tally::new(&["a"]);
        assert_eq!(t.from_saved(Saved { version: 99, counts: vec![] }), Err(LoadError::NewerVersion(99)));
    }
}
