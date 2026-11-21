//! storage.md snippets
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};

use pddb::Pddb;
use serde::{Deserialize, Serialize};

fn setup() -> Pddb {
    let pddb = Pddb::new();
    pddb.is_mounted_blocking(); // wait for the user to unlock (does not spin)
    pddb
}

// ---- Write a NEW key ----
fn write_new(pddb: &Pddb) -> std::io::Result<()> {
    let mut key = pddb.get(
        "myapp.settings", // dictionary (max 110 bytes)
        "config",         // key (max 94 bytes)
        None,             // basis: None = most recently unlocked basis
        true,             // create dict if missing
        true,             // create key if missing
        Some(256),        // alloc hint (optional)
        None::<fn()>,     // key-changed callback
    )?;
    key.write_all(b"data here")?; // writes at offset 0; does NOT truncate an existing longer value
    pddb.sync()
}

// ---- Read ----
fn read(pddb: &Pddb) -> std::io::Result<Option<Vec<u8>>> {
    match pddb.get("myapp.settings", "config", None, false, false, None, None::<fn()>) {
        Ok(mut key) => {
            let mut data = Vec::new();
            key.read_to_end(&mut data)?;
            Ok(Some(data))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None), // dict or key missing
        Err(e) => Err(e),
    }
}

// ---- Overwrite: delete-then-recreate in the key's own basis (pattern from apps/vault) ----
/// PddbKey has no set_len/truncate, so a shorter rewrite would leave stale trailing bytes.
fn store(pddb: &Pddb, dict: &str, key: &str, data: &[u8]) -> std::io::Result<()> {
    // Which basis holds the key now? (reads see a union of all open bases)
    let basis: Option<String> = match pddb.get(dict, key, None, false, false, None, None::<fn()>) {
        Ok(existing) => Some(existing.attributes()?.basis),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    if let Some(b) = basis.as_deref() {
        pddb.delete_key(dict, key, Some(b))?;
    }
    {
        // Recreate in the same basis (None => most recently unlocked basis)
        let mut k = pddb.get(dict, key, basis.as_deref(), true, true, Some(data.len()), None::<fn()>)?;
        k.write_all(data)?;
    }
    pddb.sync()
}

fn list_and_delete(pddb: &Pddb) -> std::io::Result<()> {
    // ---- Delete Key ----
    pddb.delete_key("myapp.data", "old_item", None)?;
    pddb.sync()?;

    // ---- List Keys ----
    let keys: Vec<String> = pddb.list_keys("myapp.data", None)?;
    for key_name in keys {
        log::info!("Found key: {}", key_name);
    }

    // ---- List Dictionaries ----
    let dicts: Vec<String> = pddb.list_dict(None)?;
    Ok(())
}

// ---- JSON ----
#[derive(Serialize, Deserialize, Default)]
struct AppSettings {
    theme: String,
    font_size: u8,
    notifications: bool,
}

fn save_settings_json(pddb: &Pddb, settings: &AppSettings) -> std::io::Result<()> {
    let json = serde_json::to_vec(settings)?; // serde_json::Error converts into io::Error
    store(pddb, "myapp.settings", "config", &json)
}

fn load_settings_json(pddb: &Pddb) -> AppSettings {
    let mut buf = Vec::new();
    match pddb.get("myapp.settings", "config", None, false, false, None, None::<fn()>) {
        Ok(mut key) => match key.read_to_end(&mut buf) {
            Ok(_) => serde_json::from_slice(&buf).unwrap_or_default(), // corrupt -> defaults
            Err(_) => AppSettings::default(),
        },
        Err(_) => AppSettings::default(), // missing -> defaults
    }
}

// ---- Binary (fixed size) ----
fn save_settings(pddb: &Pddb, vibrate: bool, notify: bool, volume: u8) -> std::io::Result<()> {
    let data = [vibrate as u8, notify as u8, volume];
    // Fixed-length record: overwriting in place is safe because the length never changes.
    let mut key = pddb.get("myapp.settings", "flags", None, true, true, Some(3), None::<fn()>)?;
    key.seek(SeekFrom::Start(0))?;
    key.write_all(&data)?;
    pddb.sync()
}

fn load_settings(pddb: &Pddb) -> (bool, bool, u8) {
    if let Ok(mut key) = pddb.get("myapp.settings", "flags", None, false, false, None, None::<fn()>) {
        let mut buf = [0u8; 3];
        if key.read_exact(&mut buf).is_ok() {
            return (buf[0] != 0, buf[1] != 0, buf[2]);
        }
    }
    (true, true, 50) // Defaults
}

// ---- Length-Prefixed Strings ----
fn write_string_list(key: &mut pddb::PddbKey, strings: &[String]) -> std::io::Result<()> {
    key.write_all(&(strings.len() as u32).to_le_bytes())?;
    for s in strings {
        let bytes = s.as_bytes();
        key.write_all(&(bytes.len() as u32).to_le_bytes())?;
        key.write_all(bytes)?;
    }
    Ok(())
}

fn read_string_list(key: &mut pddb::PddbKey) -> std::io::Result<Vec<String>> {
    let mut count_buf = [0u8; 4];
    key.read_exact(&mut count_buf)?;
    let count = u32::from_le_bytes(count_buf) as usize;

    let mut strings = Vec::with_capacity(count.min(1024)); // don't trust a corrupt count
    for _ in 0..count {
        let mut len_buf = [0u8; 4];
        key.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        key.read_exact(&mut buf)?;
        strings.push(String::from_utf8(buf).unwrap_or_default());
    }
    Ok(strings)
}

// ---- Collection of items / Indexed collection ----
fn add_item(pddb: &Pddb, next_id: u32, data: &[u8]) -> std::io::Result<()> {
    let keys = pddb.list_keys("myapp.items", None)?;
    let key_name = format!("item_{:03}", next_id);
    store(pddb, "myapp.items", &key_name, data)
}

fn save_index(pddb: &Pddb, order: &[String]) -> std::io::Result<()> {
    let json = serde_json::to_vec(order)?;
    store(pddb, "myapp.items", "index", &json)
}

// ---- Time-Based Keys ----
fn date_key(prefix: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    format!("{}_{}", prefix, now)
}

// ---- Error Handling ----
fn error_handling(pddb: &Pddb) {
    match pddb.get("myapp.settings", "config", None, false, false, None, None::<fn()>) {
        Ok(key) => { /* success */ }
        Err(e) => match e.kind() {
            ErrorKind::NotFound => { /* dict or key missing: create or use default */ }
            ErrorKind::InvalidInput => { /* name too long */ }
            ErrorKind::OutOfMemory => { /* PDDB full */ }
            _ => log::error!("PDDB error: {:?}", e),
        },
    }
}

// ---- Batching ----
fn batch(pddb: &Pddb, items: &[(String, Vec<u8>)]) -> std::io::Result<()> {
    for (name, data) in items {
        let mut key = pddb.get("myapp.items", name, None, true, true, Some(data.len()), None::<fn()>)?;
        key.write_all(data)?;
    }
    pddb.sync() // one sync at the end
}

// ---- Basis management ----
fn bases(pddb: &Pddb) -> std::io::Result<()> {
    // Create a new basis (Precursor: the PDDB service prompts the user for a password)
    pddb.create_basis("myapp.secrets")?;
    // Unlock (prompts for its password); None = default retention policy
    pddb.unlock_basis("myapp.secrets", None)?;
    // Lock
    pddb.lock_basis("myapp.secrets")?;
    // Delete permanently
    pddb.delete_basis("myapp.secrets")?;

    // All currently open bases, and the most recently opened one
    let bases: Vec<String> = pddb.list_basis();
    let latest: Option<String> = pddb.latest_basis();
    Ok(())
}

/// Reload when bases are locked/unlocked (pattern from services/status monitor_basis thread).
fn spawn_basis_watch(cid_to_main: xous::CID, reload_opcode: usize) {
    std::thread::spawn(move || {
        let pddb = pddb::Pddb::new();
        loop {
            let bases = pddb.monitor_basis(); // blocks until the open-basis list changes
            log::info!("bases now: {:?}", bases);
            xous::send_message(cid_to_main, xous::Message::new_scalar(reload_opcode, 0, 0, 0, 0)).ok();
        }
    });
}

// ---- Bulk operations ----
#[derive(Deserialize)]
struct Item {
    name: String,
}

fn bulk(pddb: &Pddb) -> std::io::Result<()> {
    pddb.delete_dict("myapp.old_data", None)?;

    let keys_to_delete: Vec<String> = vec!["item_001".into(), "item_002".into(), "item_003".into()];
    pddb.delete_key_list("myapp.items", keys_to_delete, None)?;
    pddb.sync()?;

    // read_dict(dict, basis, size_limit) -> Vec<PddbKeyRecord> (default limit 32 KiB total)
    for rec in pddb.read_dict("myapp.items", None, None)? {
        let data = match rec.data {
            Some(d) => d,
            None if rec.len == 0 => continue, // empty key
            None => {
                // over the bulk limit: read it individually
                let mut k = pddb.get("myapp.items", &rec.name, Some(&rec.basis), false, false, None, None::<fn()>)?;
                let mut d = Vec::new();
                k.read_to_end(&mut d)?;
                d
            }
        };
        let item: Item = serde_json::from_slice(&data)?;
    }
    Ok(())
}

// ---- Version Key Pattern ----
const CURRENT_VERSION: u32 = 2;
fn migrate_v0_to_v1(_p: &Pddb) {}
fn migrate_v1_to_v2(_p: &Pddb) {}

fn ensure_migrated(pddb: &Pddb) {
    let version = load_version(pddb).unwrap_or(0);
    if version < 1 {
        migrate_v0_to_v1(pddb);
    }
    if version < 2 {
        migrate_v1_to_v2(pddb);
    }
    save_version(pddb, CURRENT_VERSION).ok();
}

fn load_version(pddb: &Pddb) -> Option<u32> {
    let mut key = pddb.get("myapp.meta", "version", None, false, false, None, None::<fn()>).ok()?;
    let mut buf = [0u8; 4];
    key.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

fn save_version(pddb: &Pddb, version: u32) -> std::io::Result<()> {
    store(pddb, "myapp.meta", "version", &version.to_le_bytes())
}

// ---- Key Change Callback ----
fn with_callback(pddb: &Pddb) -> std::io::Result<()> {
    let key = pddb.get(
        "myapp.settings",
        "config",
        None,
        false,
        false,
        None,
        Some(|| {
            // Runs on a PDDB callback thread when a basis lock invalidates the key.
            // Keep it tiny: send a message to your main loop and re-read there.
            log::warn!("Key invalidated by basis lock! Re-read needed.");
        }),
    )?;
    Ok(())
}

// ---- Mount Lifecycle ----
fn mount(pddb: &Pddb) -> std::io::Result<()> {
    pddb.is_mounted_blocking(); // block until mounted (no CPU spinning)

    let poller = pddb::PddbMountPoller::new();
    let mounted: bool = poller.is_mounted_nonblocking(); // never blocks

    pddb.sync()?;
    Ok(())
}
