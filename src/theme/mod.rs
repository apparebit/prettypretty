//! Support for representing and querying terminal color themes.
//!
//! This module defines [`Theme`] to represent terminal color themes. It also
//! implements [`Theme::query_terminal`] for querying the terminal for its
//! current color theme.

mod query;
mod theming;

#[doc(hidden)]
pub use query::{apply, prepare, prepare_with, query1, query2, query3};
pub use theming::{Theme, ThemeEntry, ThemeEntryIterator, VGA_COLORS};
