//! JSON records in the PDDB, done correctly.
//!
//! Facts about the PDDB this module is built around (xous-core `services/pddb`):
//! * `PddbKey` has **no** `set_len`/truncate. Writing a shorter value over a
//!   longer one leaves the old tail in place. The only correct update is
//!   `delete_key` then recreate — which is what upstream `apps/vault` does.
//! * With `basis_name = None`, reads see the union of all open bases, and a
//!   **new** key is created in the most recently opened basis. An existing key
//!   lives in exactly one basis; [`Store::save`] rewrites it *in that basis*, so
//!   data never migrates between bases behind the user's back.
//! * When the user locks or unlocks a basis, every cached value may be stale or
//!   may belong to a basis that just closed. Call [`watch_bases`] once and reload
//!   your cache when its message arrives. Do not cache secrets across that event.
//! * Errors are `std::io::Error`. `NotFound` is a normal "no record yet".

use std::io::{ErrorKind, Read, Write};

use serde::{de::DeserializeOwned, Serialize};

pub type Result<T> = std::io::Result<T>;

/// A dictionary of JSON records.
pub struct Store {
    pddb: pddb::Pddb,
    dict: String,
}

impl Store {
    /// Open a store on `dict`. Blocks until the PDDB is mounted (the user has
    /// entered their PIN), so call it from `main` before loading state.
    pub fn open(dict: &str) -> Self {
        let pddb = pddb::Pddb::new();
        pddb.is_mounted_blocking();
        Self { pddb, dict: dict.to_string() }
    }

    /// Load a record. `Ok(None)` if it doesn't exist; `Err` if it exists but
    /// can't be read or parsed — in that case do **not** overwrite it blindly.
    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.read_bytes(key)? {
            None => Ok(None),
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e.to_string())),
        }
    }

    /// Load a record or fall back to `T::default()` when it's missing. Parse
    /// errors are still returned so a corrupt record is never silently replaced.
    pub fn load_or_default<T: DeserializeOwned + Default>(&self, key: &str) -> Result<T> {
        Ok(self.load(key)?.unwrap_or_default())
    }

    /// Raw read of a whole key. `Ok(None)` if missing.
    pub fn read_bytes(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut k = match self.pddb.get(&self.dict, key, None, false, false, None, None::<fn()>) {
            Ok(k) => k,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let len = k.attributes().map(|a| a.len).unwrap_or(0);
        let mut buf = Vec::with_capacity(len);
        k.read_to_end(&mut buf)?;
        if buf.len() > len {
            buf.truncate(len);
        }
        Ok(Some(buf))
    }

    /// Save a record, replacing any previous value completely.
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let bytes =
            serde_json::to_vec(value).map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e.to_string()))?;
        self.write_bytes(key, &bytes)
    }

    /// Replace a key's bytes: delete in its current basis, recreate there, write, sync.
    /// New keys go to the most recently opened basis (shown by [`Store::target_basis`]).
    pub fn write_bytes(&self, key: &str, bytes: &[u8]) -> Result<()> {
        let basis = self.basis_of(key)?;
        if let Some(b) = basis.as_deref() {
            self.pddb.delete_key(&self.dict, key, Some(b))?;
        }
        let mut k = self.pddb.get(
            &self.dict,
            key,
            basis.as_deref(),
            true,
            true,
            Some(bytes.len().max(1)),
            None::<fn()>,
        )?;
        k.write_all(bytes)?;
        drop(k);
        self.pddb.sync()
    }

    /// Delete a record. Missing keys are not an error.
    pub fn delete(&self, key: &str) -> Result<()> {
        match self.basis_of(key)? {
            Some(b) => {
                self.pddb.delete_key(&self.dict, key, Some(&b))?;
                self.pddb.sync()
            }
            None => Ok(()),
        }
    }

    /// Names of all keys in the dictionary (union view). Empty if the dict doesn't exist.
    pub fn keys(&self) -> Result<Vec<String>> {
        match self.pddb.list_keys(&self.dict, None) {
            Ok(v) => Ok(v),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Which basis holds `key`, if it exists.
    pub fn basis_of(&self, key: &str) -> Result<Option<String>> {
        match self.pddb.get(&self.dict, key, None, false, false, None, None::<fn()>) {
            Ok(k) => Ok(Some(k.attributes()?.basis)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The basis new records will be written to. Show this to the user before
    /// saving anything sensitive.
    pub fn target_basis(&self) -> Option<String> { self.pddb.latest_basis() }

    pub fn dict(&self) -> &str { &self.dict }
}

/// Spawn a thread that sends `opcode` (non-blocking scalar) to `cid` every
/// time the set of open bases changes. On receipt, drop cached records and
/// reload them.
pub fn watch_bases(cid: xous::CID, opcode: usize) {
    std::thread::spawn(move || {
        let pddb = pddb::Pddb::new();
        loop {
            let open = pddb.monitor_basis(); // blocks until the basis list changes
            log::info!("basis set changed: {:?}", open);
            if xous::send_message(cid, xous::Message::new_scalar(opcode, open.len(), 0, 0, 0)).is_err() {
                break;
            }
        }
    });
}
