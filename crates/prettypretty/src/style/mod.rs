//! Terminal-specific text fromatting and styles.
//!
//!
//! # I. Overview
//!
//! **`Style`**: This module supports styling terminal appearance with ANSI SGR
//! escape sequences through [`Style`], which combine an optional
//! [`FormatUpdate`] with an optional foreground
//! [`Colorant`](crate::termco::Colorant) and an optional background
//! [`Colorant`](crate::termco::Colorant).
//!
//! **`Fidelity`** and **`Layer`**: It also defines [`Layer`] to distinguish between
//! foreground and background colors as well as [`Fidelity`] to capture a
//! terminal's level of color support.
//!
//! **`Attribute`**, **`Format`**, and **`FormatUpdate`**: A [`FormatUpdate`]
//! comprises a disabling [`Format`] and an enabling [`Format`]. Each
//! [`Format`], in turn, comprises zero or more text [`Attribute`]s representing
//! formatting other than the default. All three types support addition,
//! negation, and subtraction:
//!
//!   * Addition combines text attributes.
//!   * Negation restores the default appearance again.
//!   * Subtraction determines minimal differences.
//!
//! Modelling format updates as disabling and enabling formats eliminates the
//! need for defining additional attributes that undo formatting and simplifies
//! the implementation of negation and subtraction.
//!
//!
//! # II. The One-Two-Three of Styles
//!
//! Using prettypretty's styles requires three steps:
//!
//!  1. **Assemble a style** by modifying the empty [`Style::default`].
//!  2. **Adjust the style** to the terminal's fidelity level with
//!     [`Style::cap`], which can translate even high-resolution colors to ANSI
//!     colors.
//!  3. **Apply the style** by writing it to the terminal and restore default
//!     appearance again by writing its negation.
//!
//! The examples cover the same three steps.
//!
//!
//! # III. Examples
//!
//! ## 1. Fluently Assemble Style
//!
//! Fluently assemble a style for bold, underlined red text:
//! ```
//! # use prettypretty::style::{Attribute, FormatUpdate, Style};
//! # use prettypretty::termco::{Colorant, Rgb};
//! let style = Style::default()
//!     .bold()
//!     .with_foreground(Rgb::new(215, 40, 39))
//!     .underlined();
//!
//! assert_eq!(
//!     style.format(),
//!     FormatUpdate::from(Attribute::Bold + Attribute::Underlined)
//! );
//! assert_eq!(
//!     style.foreground(),
//!     Some(Colorant::Rgb(Rgb::new(215, 40, 39))).as_ref()
//! );
//! assert_eq!(style.background(), None);
//! ```
//! <div class=color-swatch>
//! <div style="background-color: rgb(215 40 39);"></div>
//! </div>
//! <br>
//!
//! As demonstrated above, the order of method invocations does not matter when
//! assembling styles. If you set a color more than once, the most recent
//! invocation wins.
//!
//!
//! ## 2. Adjust Style to Terminal and User Preferences
//!
//! Prepare the style from the previous example for use in a terminal that
//! supports only ANSI colors:
//! ```
//! # use prettypretty::{OkVersion, Translator};
//! # use prettypretty::style::{Fidelity, Style};
//! # use prettypretty::termco::{AnsiColor, Colorant, Rgb};
//! # use prettypretty::theme::VGA_COLORS;
//! # let style = Style::default()
//! #     .bold()
//! #     .with_foreground(Rgb::new(215, 40, 39))
//! #     .underlined();
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
//! ## 3. Apply Style to Text
//!
//! Apply the adjusted style from the previous example to `Wow!`, while also
//! restoring terminal appearance again:
//! ```
//! # use prettypretty::{OkVersion, Translator};
//! # use prettypretty::style::{Fidelity, Style};
//! # use prettypretty::termco::{AnsiColor, Colorant, Rgb};
//! # use prettypretty::theme::VGA_COLORS;
//! # let style = Style::default()
//! #     .bold()
//! #     .with_foreground(Rgb::new(215, 40, 39))
//! #     .underlined();
//! # let translator = Translator::new(
//! #     OkVersion::Revised, VGA_COLORS.clone());
//! # let style = style.cap(Fidelity::Ansi, &translator);
//! let s = format!("{}Wow!{}", style, -&style);
//!
//! assert_eq!(s, "\x1b[1;4;31mWow!\x1b[22;24;39m");
//! ```
//! The terminal is impressed and exclaims:
//! <img style="display: inline-block; vertical-align: top"
//!     src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/wow.png"
//!     alt="wow!" width="44">
//!
//! <hr>

mod context;
mod format;
mod styling;

pub use context::{Fidelity, Layer};
pub use format::{Attribute, AttributeIter, Format, FormatUpdate};
pub use styling::Style;
