//! Terminal colors and other stylistic flourishes enabled by ANSI SGR escapes.
//!
//! This module supports styling terminal appearance with ANSI SGR escape
//! sequences through these **abstractions**:
//!
//!   * This module's primary abstraction is the [`Style`], which combines an
//!     optional text [`format::Format`] with an optional foreground and
//!     optional background [`Colorant`].
//!   * A colorant can be any of the terminal-specific color formats defined by
//!     this module, [`AnsiColor`], [`EmbeddedRgb`], [`GrayGradient`],
//!     [`EightBitColor`], and [`TrueColor`]. or high-resolution
//!     [`Color`](crate::Color), thus maximizing expressivity and user choice.
//!   * A terminal's level of support for ANSI escape codes and their various
//!     color formats is its [`Fidelity`].
//!
//! The **three steps in using styles** are:
//!
//!  1. Fluently assemble a style with [`Style::builder`] or preferrably
//!     [`stylist()`].
//!  2. Adjust the style to the terminal's fidelity level with [`Style::cap`],
//!     which can translate even high-resolution colors to ANSI colors.
//!  3. Apply the style by writing it to the terminal and restore default
//!     appearance again by writing its negation.
//!
//! The examples cover the same three steps.
//!
//!
//! # Examples
//!
//! ## Fluently Assemble Style
//!
//! Fluently assemble a style for bold, underlined red text using
//! [`Style::builder`] or [`stylist()`]:
//! ```
//! # use prettypretty::style::{stylist, Colorant, format::Format, TrueColor};
//! let style = stylist()
//!     .bold()
//!     .foreground(TrueColor::new(215, 40, 39))
//!     .underlined()
//!     .et_voila();
//!
//! assert_eq!(
//!     style.format(),
//!     Some(Format::new().bold().underlined())
//! );
//! assert_eq!(
//!     style.foreground(),
//!     Some(Colorant::Rgb(TrueColor::new(215, 40, 39))).as_ref()
//! );
//! assert_eq!(style.background(), None);
//! ```
//! <div class=color-swatch>
//! <div style="background-color: rgb(215 40 39);"></div>
//! </div>
//! <br>
//!
//! As demonstrated above, the order of builder method invocations does not
//! matter. If you set a color more than once, the most recent invocation wins.
//!
//! If `stylist()` and `et_voila()` are too sassy for you, prettypretty includes
//! [`Style::builder()`] and [`build()`](Stylist::build) as well.
//!
//!
//! ## Adjust Style to Terminal
//!
//! Prepare the style from the previous example for use in a terminal that
//! supports only ANSI colors:
//! ```
//! # use prettypretty::OkVersion;
//! # use prettypretty::style::{stylist, AnsiColor, Colorant, Fidelity, TrueColor};
//! # use prettypretty::trans::{Translator, VGA_COLORS};
//! # let style = stylist()
//! #     .bold()
//! #     .foreground(TrueColor::new(215, 40, 39))
//! #     .underlined()
//! #     .et_voila();
//! let translator = Translator::new(
//!     OkVersion::Revised, VGA_COLORS.clone());
//!
//! let style = style.cap(Fidelity::Ansi, &translator);
//!
//! assert_eq!(
//!     style.foreground(),
//!     Some(Colorant::Ansi(AnsiColor::Red)).as_ref()
//! );
//! ```
//! <div class=color-swatch>
//! <div style="background-color: rgb(170 0 0);"></div>
//! </div>
//! <br>
//!
//!
//! ## Apply Style to Text
//!
//! Apply the adjusted style from the previous example to `Wow!`, while also
//! restoring terminal appearance again:
//! ```
//! # use prettypretty::OkVersion;
//! # use prettypretty::style::{stylist, AnsiColor, Colorant, Fidelity, TrueColor};
//! # use prettypretty::trans::{Translator, VGA_COLORS};
//! # let style = stylist()
//! #     .bold()
//! #     .foreground(TrueColor::new(215, 40, 39))
//! #     .underlined()
//! #     .et_voila();
//! # let translator = Translator::new(
//! #     OkVersion::Revised, VGA_COLORS.clone());
//! # let style = style.cap(Fidelity::Ansi, &translator);
//! let s = format!("{}Wow!{}", style, !&style);
//!
//! assert_eq!(s, "\x1b[1;4;31mWow!\x1b[22;24;39m");
//! ```

mod color;
mod context;
pub mod format;
mod styling;

#[cfg(feature = "pyffi")]
pub(crate) use color::into_colorant;

pub use color::{AnsiColor, Colorant, EightBitColor, EmbeddedRgb, GrayGradient, TrueColor};
pub use context::{Fidelity, Layer};
pub use styling::{stylist, Style, Stylist};
