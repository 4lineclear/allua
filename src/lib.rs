//! a basic version of a lua-like langauge
#![deny(unsafe_code)]
#![deny(
    clippy::all,
    clippy::pedantic,
    // clippy::cargo,
    clippy::nursery,
    missing_docs,
    rustdoc::all,
    future_incompatible
)]
#![warn(missing_debug_implementations)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::single_match)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::equatable_if_let)]
// #![allow(clippy::multiple_crate_versions)]
// #![allow(clippy::must_use_candidate)]
// #![allow(clippy::wildcard_dependencies)]
// #![allow(clippy::wildcard_imports)]
// #![allow(clippy::unused_io_amount)]
// #![allow(clippy::cast_possible_truncation)]
// #![allow(clippy::new_without_default)]
#![allow(missing_docs)]
#![allow(rustdoc::invalid_html_tags)]
// #![allow(dead_code)]

pub mod error;
pub mod lex;
pub mod parse;
pub mod span;
#[allow(clippy::unicode_not_nfc)]
pub mod unicode;
pub mod util;
