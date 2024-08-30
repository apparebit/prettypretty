//! Terminal colors and other stylistic flourishes of text enabled by ANSI
//! escapes.
//!
//! The module's basic domain model is extremely simple: Colors and other
//! formats combine to styles. Styles, in turn, determine the appearance of rich
//! text. However, in practice, it takes a few more types. In particular, styles
//! combine some number of style tokens, which include colors and text formats.
//! Text formats combine some number of text attributes. Formats may very well
//! disable text attributes, but such formats can only be constructed indirectly
//! through negation and subtraction.
//!
//!
//! # Styles
//!
//! A terminal [`Style`] is fluently assembled by stringing methods off
//! [`stylist()`] until a final `go()`. For example:
//! ```
//! # use prettypretty::style::{stylist, AnsiColor, format::Format, StyleToken, TerminalColor};
//! let style = stylist().bold().underlined().foreground(AnsiColor::Red).go();
//!
//! for token in style.tokens() {
//!     match token {
//!         StyleToken::Format(format) => {
//!             assert_eq!(format, Format::new().bold().underlined());
//!         }
//!         StyleToken::Foreground(color) => {
//!             assert_eq!(color, TerminalColor::Ansi { color: AnsiColor::Red });
//!         }
//!         _ => panic!("unexpected style token {:?}", token)
//!     }
//! }
//! ```
//! The example uses the [`StyleBuilder`] returned by [`stylist()`] to create a
//! style comprising bold, underlined, red text. As demonstrated by the loop
//! iterating over the [`StyleToken`]s, the style builder transparently combined
//! the bold and underlined attributes into a single format, here wrapped by a
//! style token.
//!
//! When styles are combined with text, they form [`RichText`].
//!
//!
//! # Terminal Colors
//!
//! The unifying [`TerminalColor`] abstraction combines, in order of decreasing
//! age and increasing resolution, [`DefaultColor`], [`AnsiColor`],
//! [`EmbeddedRgb`], [`GrayGradient`], and [`TrueColor`]. Out of these, default
//! and the extended ANSI colors not only have the lowest resolution—one default
//! color each for foreground and background as well as sixteen extended ANSI
//! colors—but they also are abstract. That is, their appearance is (coarsely)
//! defined, but they do not have standardized or widely accepted color values.
//!
//! Where possible, `From` and `TryFrom` trait implementations convert between
//! different terminal color abstractions. More complicated conversions are
//! implemented by the [`trans`](crate::trans) module.

mod color;
mod context;
pub mod format;
mod styling;

#[cfg(feature = "pyffi")]
pub(crate) use color::into_terminal_color;

pub use color::{AnsiColor, DefaultColor, EmbeddedRgb, GrayGradient, TerminalColor, TrueColor};
pub use context::{Fidelity, Layer};
pub use styling::{stylist, RichText, Style, StyleBuilder, StyleToken};
