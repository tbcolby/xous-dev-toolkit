//! # precursor-appkit-core
//!
//! The pure half of `precursor-appkit`. No Xous dependencies, so an app's
//! `core/` crate can use it and run `cargo test` on any machine.
//!
//! In an app's `core/Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! precursor-appkit-core = { git = "https://github.com/tbcolby/xous-dev-toolkit" }
//! [dev-dependencies]
//! precursor-appkit-core = { git = "https://github.com/tbcolby/xous-dev-toolkit", features = ["testing"] }
//! ```
//!
//! `precursor-appkit` re-exports every module here, so app shells keep using
//! `precursor_appkit::keys` etc.

#![forbid(unsafe_code)]



pub mod entropy;
pub mod keys;
pub mod layout;
pub mod records;
pub mod text;
