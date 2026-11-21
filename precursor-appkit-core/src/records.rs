//! Naming and loading for "one PDDB key per record" storage.
//!
//! Many small records (contacts, saved codes, notes) are best stored one per
//! key: delete really deletes, a corrupt record affects only itself, and each
//! record stays in the basis it was created in. Keys are `<prefix><16 hex>`,
//! named from fail-closed entropy so two devices never collide and no counter
//! has to be stored. The Xous side is `precursor_appkit::store::Store::load_records`.

use crate::entropy::{Entropy, EntropyError};

/// A fresh random record key: `prefix` followed by 16 lowercase hex digits.
pub fn new_key<E: Entropy + ?Sized>(prefix: &str, e: &mut E) -> Result<String, EntropyError> {
    let hi = e.try_u32()?;
    let lo = e.try_u32()?;
    Ok(format!("{prefix}{hi:08x}{lo:08x}"))
}

/// True if `name` was produced by [`new_key`] with this prefix. Lets a
/// dictionary also hold settings keys without them being parsed as records.
pub fn is_key(prefix: &str, name: &str) -> bool {
    name.strip_prefix(prefix)
        .is_some_and(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
}

/// Result of loading every record in a dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded<T> {
    /// `(key, record, basis)` for each record that parsed.
    pub items: Vec<(String, T, Option<String>)>,
    /// Records that exist but couldn't be read or parsed. Show this count and
    /// never overwrite those keys: they may be from a newer app version.
    pub unreadable: usize,
}

impl<T> Default for Loaded<T> {
    fn default() -> Self { Self { items: Vec::new(), unreadable: 0 } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entropy::testing::{Broken, Seq};

    #[test]
    fn new_key_format_and_fail_closed() {
        let k = new_key("c.", &mut Seq(vec![0xdead_beef, 0x0000_0001], 0)).unwrap();
        assert_eq!(k, "c.deadbeef00000001");
        assert!(is_key("c.", &k));
        assert_eq!(new_key("c.", &mut Broken), Err(EntropyError));
    }

    #[test]
    fn is_key_rejects_other_names() {
        assert!(!is_key("c.", "settings"));
        assert!(!is_key("c.", "c.DEADBEEF00000001"));
        assert!(!is_key("c.", "c.deadbeef"));
        assert!(!is_key("c.", "x.deadbeef00000001"));
        assert!(is_key("", "0123456789abcdef"));
    }
}
