//! Terminal colors and other stylistic flourishes enabled by ANSI escapes.
//!
//! The module's domain model can be described simply as styles with colors and
//! formatting. In fact, its support for fluently assembling styles very much
//! feels like that. Consider the first line in the code example, which builds a
//! bold, underlined, red style:
//! ```
//! # use prettypretty::style::{stylist, AnsiColor, format::Format, StyleToken, TerminalColor};
//! let style = stylist().bold().foreground(AnsiColor::Red).underlined().go();
//!
//! for (index, token) in style.tokens().enumerate() {
//!     match token {
//!         StyleToken::Foreground(color) => {
//!             assert_eq!(index, 0);
//!             assert_eq!(color, TerminalColor::Ansi { color: AnsiColor::Red });
//!         }
//!         StyleToken::Format(format) => {
//!             assert_eq!(index, 1);
//!             assert_eq!(format, Format::new().bold().underlined());
//!         }
//!         _ => panic!("unexpected style token {:?}", token)
//!     }
//! }
//! ```
//! Alas, as hinted at by the loop below the fluent builder expression, it takes
//! a number of types to represent the various terminal colors as well as other
//! formats and to then combine them into a coherent domain model.
//!
//! In particular:
//!
//!   * [`TerminalColor`] represents the different color formats supported by
//!     terminals. It combines [`AnsiColor`], [`DefaultColor`], [`EmbeddedRgb`],
//!     [`GrayGradient`], and [`TrueColor`].
//!   * [`Format`](format::Format) combines some number of stylistic
//!     [`Attribute`](format::Attribute)s of text other than color.
//!   * [`Style] combines some number of [`StyleToken`]s, each of which can
//!     represent a [`Color`] or [`TerminalColor`] of the foreground or
//!     background, a [`Format`], or other changes to the terminal's
//!     appearance.
//!   * Amongst helper types, [`Layer`] distinguishes between foreground as well
//!     as background, and [`Fidelity`] represents a terminal's color support or
//!     a user's preferences.
//!
//! Additionally, this module defines several iterators and implements
//! conversions with `From` and `TryFrom`.

mod color;
mod context;
pub mod format;
mod styling;

#[cfg(feature = "pyffi")]
pub(crate) use color::into_terminal_color;

pub use color::{AnsiColor, DefaultColor, EmbeddedRgb, GrayGradient, TerminalColor, TrueColor};
pub use context::{Fidelity, Layer};
pub use styling::{stylist, RichText, Style, StyleToken, Stylist, TokenIterator};
