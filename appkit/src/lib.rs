//! # precursor-appkit
//!
//! Shared building blocks for Precursor apps running on Xous. Every module
//! exists because the same mistake was made independently in many apps:
//!
//! | Module    | Replaces the hand-rolled version that…                                   |
//! |-----------|---------------------------------------------------------------------------|
//! | [`keys`]  | used Apple PUA arrow codes and bound "Back" to `∴`, which GAM swallows     |
//! | [`rng`]   | turned a TRNG failure into zeros (`unwrap_or(0)`) and used biased `% n`    |
//! | [`layout`]| re-derived list scrolling and hard-coded a 336x536 canvas                  |
//! | [`store`] | called the nonexistent `PddbKey::set_len`, left stale tails, ignored bases |
//! | [`pump`]  | kept animating in the background, or doubled its tick rate on refocus     |
//! | [`screen`]| used `invert` (refused for apps), `gam.clear()`, or `graphics_server`      |
//! | [`app`]   | registered the UX with the xous-names server name instead of `APP_NAME_*`  |
//!
//! `keys`, `layout` and `entropy` come from `precursor-appkit-core` (pure,
//! usable from an app's `core/` crate); `rng::unbiased_below` is pure too.
//! All are unit-tested on the host with `cargo test --no-default-features`. The rest need Xous services.

pub use precursor_appkit_core::{date, entropy, keys, layout, records, text};
pub mod rng;

#[cfg(feature = "xous")]
pub mod app;
#[cfg(feature = "xous")]
pub mod pump;
#[cfg(feature = "xous")]
pub mod screen;
#[cfg(feature = "xous")]
pub mod store;

/// The xous-core commit this crate is verified against.
pub const XOUS_CORE_ANCHOR: &str = "31fbe23";
