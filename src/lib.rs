//! # Pretty 🌸 Pretty
//!
//! This library brings 2020s color science to 1970s terminals to help build
//! awesome looking and adaptable terminal user interfaces. It supports
//! high-resolution colors, accurate conversion between color spaces, finding
//! the closest matching color, gamut testing and mapping, and computing text
//! contrast.
//!
//!
//! ## 1. High-Resolution Colors
//!
//! High-resolution colors from the 2020s have floating point coordinates and
//! explicit color spaces:
//!
//!   * [`ColorSpace`] enumerates supported color spaces.
//!   * [`Color`] adds `f64` coordinates to precisely represent colors.
//!
//! The example below instantiates a color in the polar Oklch color space. It
//! then converts the color to Display P3 and tests whether it is in gamut—it
//! is. Next, it converts the color sRGB and tests whether it is in gamut—it is
//! not. Finally, it maps the color into sRGB's gamut. If you are reading this
//! on a wide-gamut screen, the color swatch below the code should show two
//! distinct shades of pink, with the left one considerably more intense.
//!
//! ```
//! # use prettypretty::{Color, ColorSpace};
//! let oklch = Color::oklch(0.716, 0.349, 335);
//! let p3 = oklch.to(ColorSpace::DisplayP3);
//! assert!(p3.in_gamut());
//!
//! let not_srgb = oklch.to(ColorSpace::Srgb);
//! assert!(!not_srgb.in_gamut());
//!
//! let srgb = not_srgb.map_to_gamut();
//! assert_eq!(srgb, Color::srgb(1, 0.15942348587138203, 0.9222706101768445));
//! ```
//! <style>
//! .color-swatch {
//!     display: flex;
//! }
//! .color-swatch > div {
//!     height: 4em;
//!     width: 4em;
//!     border: black 0.5pt solid;
//!     display: flex;
//!     align-items: center;
//!     justify-content: center;
//! }
//! .small.color-swatch > div {
//!     height: 1em;
//!     width: 1em;
//! }
//! </style>
//! <div class=color-swatch>
//! <div style="background-color: oklch(0.716 0.349 335);"></div>
//! <div style="background-color: color(srgb 1 0.15942 0.92227);"></div>
//! </div>
//!
//! ### Different Color Spaces for Different Tasks
//!
//! Instead of creating a color out of nothing, we could as easily modify an
//! existing color, for example, by pushing lightness, reducing chroma, or
//! nudging the hue. As it turns out, the perceptually uniform polar coordinates
//! of Oklch and Oklrch make them great color spaces for modifying colors.
//!
//! If we need to compare colors, however, then the Cartesian coordinates of
//! Oklab and Oklrab support a straight-forward Euclidian distance metric.
//!
//! Alas, it's back to sRGB for checking that thusly manipulated colors can
//! actually be displayed in terminals. If we are targeting other platforms,
//! such as the web, then Display P3 or even Rec. 2020 become options, too.
//!
//!
//! ## 2. Terminal Colors
//!
//! In contrast to high-resolution colors, terminal color formats from the 1970s
//! and 1980s may not even have coordinates, only integer index values. They are
//! represented through the following abstractions:
//!
//!   * [`EightBitColor`] combines [`AnsiColor`], [`EmbeddedRgb`], and
//!     [`GrayGradient`].
//!   * [`TrueColor`] represents 24-bit RGB colors, originally in the "device
//!     RGB" color space, nowadays sRGB.
//!
//! [`AnsiColor`] represents the 16 extended ANSI colors. They are eight base
//! colors—black, red, green, yellow, blue, magenta, cyan, and white—and their
//! bright variations—including bright black and bright white. ANSI colors have
//! names but no agreed-upon, intrinsic color values.
//!
//! [`EmbeddedRgb`] is a 6x6x6 RGB cube, i.e., every coordinate ranges from 0 to
//! 5, inclusive. Xterm's formula for converting to 24-bit RGB colors is widely
//! accepted. The color swatch below shows all 216 colors, with blue cycling
//! every column, green increasing every six columns, and red increasing every
//! row.
//!
//! <figure>
//! <div class="small color-swatch">
//! <div style="background-color: #000000;"></div>
//! <div style="background-color: #00005f;"></div>
//! <div style="background-color: #000087;"></div>
//! <div style="background-color: #0000af;"></div>
//! <div style="background-color: #0000d7;"></div>
//! <div style="background-color: #0000ff;"></div>
//! <div style="background-color: #005f00;"></div>
//! <div style="background-color: #005f5f;"></div>
//! <div style="background-color: #005f87;"></div>
//! <div style="background-color: #005faf;"></div>
//! <div style="background-color: #005fd7;"></div>
//! <div style="background-color: #005fff;"></div>
//! <div style="background-color: #008700;"></div>
//! <div style="background-color: #00875f;"></div>
//! <div style="background-color: #008787;"></div>
//! <div style="background-color: #0087af;"></div>
//! <div style="background-color: #0087d7;"></div>
//! <div style="background-color: #0087ff;"></div>
//! <div style="background-color: #00af00;"></div>
//! <div style="background-color: #00af5f;"></div>
//! <div style="background-color: #00af87;"></div>
//! <div style="background-color: #00afaf;"></div>
//! <div style="background-color: #00afd7;"></div>
//! <div style="background-color: #00afff;"></div>
//! <div style="background-color: #00d700;"></div>
//! <div style="background-color: #00d75f;"></div>
//! <div style="background-color: #00d787;"></div>
//! <div style="background-color: #00d7af;"></div>
//! <div style="background-color: #00d7d7;"></div>
//! <div style="background-color: #00d7ff;"></div>
//! <div style="background-color: #00ff00;"></div>
//! <div style="background-color: #00ff5f;"></div>
//! <div style="background-color: #00ff87;"></div>
//! <div style="background-color: #00ffaf;"></div>
//! <div style="background-color: #00ffd7;"></div>
//! <div style="background-color: #00ffff;"></div>
//! </div>
//! <div class="small color-swatch">
//! <div style="background-color: #5f0000;"></div>
//! <div style="background-color: #5f005f;"></div>
//! <div style="background-color: #5f0087;"></div>
//! <div style="background-color: #5f00af;"></div>
//! <div style="background-color: #5f00d7;"></div>
//! <div style="background-color: #5f00ff;"></div>
//! <div style="background-color: #5f5f00;"></div>
//! <div style="background-color: #5f5f5f;"></div>
//! <div style="background-color: #5f5f87;"></div>
//! <div style="background-color: #5f5faf;"></div>
//! <div style="background-color: #5f5fd7;"></div>
//! <div style="background-color: #5f5fff;"></div>
//! <div style="background-color: #5f8700;"></div>
//! <div style="background-color: #5f875f;"></div>
//! <div style="background-color: #5f8787;"></div>
//! <div style="background-color: #5f87af;"></div>
//! <div style="background-color: #5f87d7;"></div>
//! <div style="background-color: #5f87ff;"></div>
//! <div style="background-color: #5faf00;"></div>
//! <div style="background-color: #5faf5f;"></div>
//! <div style="background-color: #5faf87;"></div>
//! <div style="background-color: #5fafaf;"></div>
//! <div style="background-color: #5fafd7;"></div>
//! <div style="background-color: #5fafff;"></div>
//! <div style="background-color: #5fd700;"></div>
//! <div style="background-color: #5fd75f;"></div>
//! <div style="background-color: #5fd787;"></div>
//! <div style="background-color: #5fd7af;"></div>
//! <div style="background-color: #5fd7d7;"></div>
//! <div style="background-color: #5fd7ff;"></div>
//! <div style="background-color: #5fff00;"></div>
//! <div style="background-color: #5fff5f;"></div>
//! <div style="background-color: #5fff87;"></div>
//! <div style="background-color: #5fffaf;"></div>
//! <div style="background-color: #5fffd7;"></div>
//! <div style="background-color: #5fffff;"></div>
//! </div>
//! <div class="small color-swatch">
//! <div style="background-color: #870000;"></div>
//! <div style="background-color: #87005f;"></div>
//! <div style="background-color: #870087;"></div>
//! <div style="background-color: #8700af;"></div>
//! <div style="background-color: #8700d7;"></div>
//! <div style="background-color: #8700ff;"></div>
//! <div style="background-color: #875f00;"></div>
//! <div style="background-color: #875f5f;"></div>
//! <div style="background-color: #875f87;"></div>
//! <div style="background-color: #875faf;"></div>
//! <div style="background-color: #875fd7;"></div>
//! <div style="background-color: #875fff;"></div>
//! <div style="background-color: #878700;"></div>
//! <div style="background-color: #87875f;"></div>
//! <div style="background-color: #878787;"></div>
//! <div style="background-color: #8787af;"></div>
//! <div style="background-color: #8787d7;"></div>
//! <div style="background-color: #8787ff;"></div>
//! <div style="background-color: #87af00;"></div>
//! <div style="background-color: #87af5f;"></div>
//! <div style="background-color: #87af87;"></div>
//! <div style="background-color: #87afaf;"></div>
//! <div style="background-color: #87afd7;"></div>
//! <div style="background-color: #87afff;"></div>
//! <div style="background-color: #87d700;"></div>
//! <div style="background-color: #87d75f;"></div>
//! <div style="background-color: #87d787;"></div>
//! <div style="background-color: #87d7af;"></div>
//! <div style="background-color: #87d7d7;"></div>
//! <div style="background-color: #87d7ff;"></div>
//! <div style="background-color: #87ff00;"></div>
//! <div style="background-color: #87ff5f;"></div>
//! <div style="background-color: #87ff87;"></div>
//! <div style="background-color: #87ffaf;"></div>
//! <div style="background-color: #87ffd7;"></div>
//! <div style="background-color: #87ffff;"></div>
//! </div>
//! <div class="small color-swatch">
//! <div style="background-color: #af0000;"></div>
//! <div style="background-color: #af005f;"></div>
//! <div style="background-color: #af0087;"></div>
//! <div style="background-color: #af00af;"></div>
//! <div style="background-color: #af00d7;"></div>
//! <div style="background-color: #af00ff;"></div>
//! <div style="background-color: #af5f00;"></div>
//! <div style="background-color: #af5f5f;"></div>
//! <div style="background-color: #af5f87;"></div>
//! <div style="background-color: #af5faf;"></div>
//! <div style="background-color: #af5fd7;"></div>
//! <div style="background-color: #af5fff;"></div>
//! <div style="background-color: #af8700;"></div>
//! <div style="background-color: #af875f;"></div>
//! <div style="background-color: #af8787;"></div>
//! <div style="background-color: #af87af;"></div>
//! <div style="background-color: #af87d7;"></div>
//! <div style="background-color: #af87ff;"></div>
//! <div style="background-color: #afaf00;"></div>
//! <div style="background-color: #afaf5f;"></div>
//! <div style="background-color: #afaf87;"></div>
//! <div style="background-color: #afafaf;"></div>
//! <div style="background-color: #afafd7;"></div>
//! <div style="background-color: #afafff;"></div>
//! <div style="background-color: #afd700;"></div>
//! <div style="background-color: #afd75f;"></div>
//! <div style="background-color: #afd787;"></div>
//! <div style="background-color: #afd7af;"></div>
//! <div style="background-color: #afd7d7;"></div>
//! <div style="background-color: #afd7ff;"></div>
//! <div style="background-color: #afff00;"></div>
//! <div style="background-color: #afff5f;"></div>
//! <div style="background-color: #afff87;"></div>
//! <div style="background-color: #afffaf;"></div>
//! <div style="background-color: #afffd7;"></div>
//! <div style="background-color: #afffff;"></div>
//! </div>
//! <div class="small color-swatch">
//! <div style="background-color: #d70000;"></div>
//! <div style="background-color: #d7005f;"></div>
//! <div style="background-color: #d70087;"></div>
//! <div style="background-color: #d700af;"></div>
//! <div style="background-color: #d700d7;"></div>
//! <div style="background-color: #d700ff;"></div>
//! <div style="background-color: #d75f00;"></div>
//! <div style="background-color: #d75f5f;"></div>
//! <div style="background-color: #d75f87;"></div>
//! <div style="background-color: #d75faf;"></div>
//! <div style="background-color: #d75fd7;"></div>
//! <div style="background-color: #d75fff;"></div>
//! <div style="background-color: #d78700;"></div>
//! <div style="background-color: #d7875f;"></div>
//! <div style="background-color: #d78787;"></div>
//! <div style="background-color: #d787af;"></div>
//! <div style="background-color: #d787d7;"></div>
//! <div style="background-color: #d787ff;"></div>
//! <div style="background-color: #d7af00;"></div>
//! <div style="background-color: #d7af5f;"></div>
//! <div style="background-color: #d7af87;"></div>
//! <div style="background-color: #d7afaf;"></div>
//! <div style="background-color: #d7afd7;"></div>
//! <div style="background-color: #d7afff;"></div>
//! <div style="background-color: #d7d700;"></div>
//! <div style="background-color: #d7d75f;"></div>
//! <div style="background-color: #d7d787;"></div>
//! <div style="background-color: #d7d7af;"></div>
//! <div style="background-color: #d7d7d7;"></div>
//! <div style="background-color: #d7d7ff;"></div>
//! <div style="background-color: #d7ff00;"></div>
//! <div style="background-color: #d7ff5f;"></div>
//! <div style="background-color: #d7ff87;"></div>
//! <div style="background-color: #d7ffaf;"></div>
//! <div style="background-color: #d7ffd7;"></div>
//! <div style="background-color: #d7ffff;"></div>
//! </div>
//! <div class="small color-swatch">
//! <div style="background-color: #ff0000;"></div>
//! <div style="background-color: #ff005f;"></div>
//! <div style="background-color: #ff0087;"></div>
//! <div style="background-color: #ff00af;"></div>
//! <div style="background-color: #ff00d7;"></div>
//! <div style="background-color: #ff00ff;"></div>
//! <div style="background-color: #ff5f00;"></div>
//! <div style="background-color: #ff5f5f;"></div>
//! <div style="background-color: #ff5f87;"></div>
//! <div style="background-color: #ff5faf;"></div>
//! <div style="background-color: #ff5fd7;"></div>
//! <div style="background-color: #ff5fff;"></div>
//! <div style="background-color: #ff8700;"></div>
//! <div style="background-color: #ff875f;"></div>
//! <div style="background-color: #ff8787;"></div>
//! <div style="background-color: #ff87af;"></div>
//! <div style="background-color: #ff87d7;"></div>
//! <div style="background-color: #ff87ff;"></div>
//! <div style="background-color: #ffaf00;"></div>
//! <div style="background-color: #ffaf5f;"></div>
//! <div style="background-color: #ffaf87;"></div>
//! <div style="background-color: #ffafaf;"></div>
//! <div style="background-color: #ffafd7;"></div>
//! <div style="background-color: #ffafff;"></div>
//! <div style="background-color: #ffd700;"></div>
//! <div style="background-color: #ffd75f;"></div>
//! <div style="background-color: #ffd787;"></div>
//! <div style="background-color: #ffd7af;"></div>
//! <div style="background-color: #ffd7d7;"></div>
//! <div style="background-color: #ffd7ff;"></div>
//! <div style="background-color: #ffff00;"></div>
//! <div style="background-color: #ffff5f;"></div>
//! <div style="background-color: #ffff87;"></div>
//! <div style="background-color: #ffffaf;"></div>
//! <div style="background-color: #ffffd7;"></div>
//! <div style="background-color: #ffffff;"></div>
//! </div>
//! </figure>
//!
//! [`GrayGradient`] represents a 24-step gradient from almost black to almost
//! white. As for the embedded RGB cube, Xterm's formula for converting to
//! 24-bit RGB grays is widely accepted. The color swatch below illustrates the
//! gray gradient.
//!
//! <figure>
//! <div class="small color-swatch">
//! <div style="background-color: #121212;"></div>
//! <div style="background-color: #1c1c1c;"></div>
//! <div style="background-color: #262626;"></div>
//! <div style="background-color: #303030;"></div>
//! <div style="background-color: #3a3a3a;"></div>
//! <div style="background-color: #444444;"></div>
//! <div style="background-color: #4e4e4e;"></div>
//! <div style="background-color: #585858;"></div>
//! <div style="background-color: #626262;"></div>
//! <div style="background-color: #6c6c6c;"></div>
//! <div style="background-color: #767676;"></div>
//! <div style="background-color: #808080;"></div>
//! <div style="background-color: #8a8a8a;"></div>
//! <div style="background-color: #949494;"></div>
//! <div style="background-color: #9e9e9e;"></div>
//! <div style="background-color: #a8a8a8;"></div>
//! <div style="background-color: #b2b2b2;"></div>
//! <div style="background-color: #bcbcbc;"></div>
//! <div style="background-color: #c6c6c6;"></div>
//! <div style="background-color: #d0d0d0;"></div>
//! <div style="background-color: #dadada;"></div>
//! <div style="background-color: #e4e4e4;"></div>
//! <div style="background-color: #eeeeee;"></div>
//! <div style="background-color: #f8f8f8;"></div>
//! </div>
//! </figure>
//!
//! By combining ANSI, embedded RGB, and gray gradient colors, [`EightBitColor`]
//! covers the entire 8-bit code space. As a result, conversion from `u8` to
//! `EightBitColor` is infallible, whereas it is fallible for the three
//! component colors.
//!
//! [`TrueColor`] was a misnomer even when 24-bit video cards first came out.
//! Nowadays, the ready availability of wide-gamut and high-dynamic-range (HDR)
//! displays only underlines that true color is anything but true. But it *is*
//! the historically accurate term and lives on in this crate thanks to a mix of
//! ironic detachment and nostalgia.
//!
//! The example code below illustrates how [`AnsiColor`], [`EmbeddedRgb`],
//! [`GrayGradient`], and [`EightBitColor`] abstract over the underlying 8-bit
//! index space while also providing convenient access to RGB coordinates and
//! gray levels. Embedded RGB and gray gradient colors also nicely convert to
//! true colors, but ANSI and therefore 8-bit colors do not.
//!
//! ```
//! # use prettypretty::{AnsiColor, EightBitColor, EmbeddedRgb};
//! # use prettypretty::{GrayGradient, TrueColor};
//! let red = AnsiColor::BrightRed;
//! assert_eq!(u8::from(red), 9);
//! // Is TrueColor the equivalent of #f00, #f55, #e60000, #e74856, or what?
//!
//! let purple = EmbeddedRgb::new(3, 1, 4).unwrap();
//! let index = 16 + 3 * 36 + 1 * 6 + 4 * 1;
//! assert_eq!(index, 134);
//! assert_eq!(u8::from(purple), index);
//! assert_eq!(TrueColor::from(purple), TrueColor::new(175, 95, 215));
//!
//! let gray = GrayGradient::new(18).unwrap();
//! let index = 232 + 18;
//! assert_eq!(index, 250);
//! assert_eq!(gray.level(), 18);
//! assert_eq!(u8::from(gray), index);
//! assert_eq!(TrueColor::from(gray), TrueColor::new(188, 188, 188));
//!
//! let green = EightBitColor::from(71);
//! assert!(matches!(green, EightBitColor::Rgb(_)));
//! if let EightBitColor::Rgb(also_green) = green {
//!     assert_eq!(also_green[0], 1);
//!     assert_eq!(also_green[1], 3);
//!     assert_eq!(also_green[2], 1);
//!     assert_eq!(TrueColor::from(also_green), TrueColor::new(95, 175, 95));
//! } else {
//!     unreachable!("green is an embedded RGB color")
//! }
//! ```
//! <div class=color-swatch>
//! <div style="background: repeating-linear-gradient(45deg, #fff, #fff 10px, #fdd 10px, #fdd 20px);">
//! <span style="font-size: 2.5em;">?</span>
//! </div>
//! <div style="background-color: #af5fd7;"></div>
//! <div style="background-color: #bcbcbc;"></div>
//! <div style="background-color: #5faf5f;"></div>
//! </div>
//!
//!
//! ## 3. Integration of High-Resolution and Terminal Colors
//!
//! To apply 2020s color science to terminal colors, we need to be able to
//! convert them to high-resolution colors and back again:
//!
//!   * [`Theme`] provides high-resolution color values for the 16 extended ANSI
//!     colors and terminal defaults.
//!   * [`ColorMatcher`] stores high-resolution color values for all
//!     8-bit terminal colors to find closest matching color.
//!
//! Terminal emulators address ANSI colors' lack of intrinsic color values by
//! making colors configurable through [color
//! themes](https://gogh-co.github.io/Gogh/). This crate takes the exact same
//! approach. Though applications shouldn't require configuration and, as
//! described in the next section, use ANSI escape codes to query the terminal
//! for its current color theme instead. As the code example below illustrates,
//! with such a color [`Theme`], converting ANSI colors to high-resolution
//! colors becomes as simple as an index expression.
//!
//! Conversion in the other direction, from high-resolution colors to terminal
//! colors, requires two different strategies, depending on the targeted
//! terminal color format's resolution. When targeting 24-bit color, the
//! conversion from floating point to integer representations does incur loss of
//! resolution. But the important part is to convert and gamut-map the source
//! color to sRGB first. When targeting 8-bit and ANSI colors, there are so few
//! candidates that searching for the closest match becomes practical.
//! [`ColorMatcher`] collects and stores the necessary color values.
//!
//! The example below illustrates the use of color theme and matcher for
//! conversion between ANSI colors and high-resolution colors.
//!
//! ```
//! # use prettypretty::{AnsiColor, Color, ColorFormatError, ColorMatcher, DEFAULT_THEME};
//! # use prettypretty::{EightBitColor, EmbeddedRgb, GrayGradient, OkVersion, TrueColor};
//! # use std::str::FromStr;
//! let red = &DEFAULT_THEME[AnsiColor::BrightRed];
//! assert_eq!(red, &Color::srgb(1, 0.333333333333333, 0.333333333333333));
//!
//! let matcher = ColorMatcher::new(&DEFAULT_THEME, OkVersion::Revised);
//! let yellow = Color::from_str("#FFE06C")?;
//! let bright_yellow = matcher.to_ansi(&yellow);
//! assert_eq!(u8::from(bright_yellow), 11);
//! # Ok::<(), ColorFormatError>(())
//! ```
//! <div class=color-swatch>
//! <div style="background-color: #f55;"></div>
//! <div style="background-color: #ffe06c;"></div>
//! <div style="background-color: #ffff55;"></div>
//! </div>
//!
//!
//! ## 4. Features
//!
//! This crate has two features. While mutually exclusive use is not enforced,
//! it probably is the meaningful use. After all, each feature enables a very
//! different interface to the same functionality:
//!
//!   - **`color-object`** enables the high-level, nicely encapsulated interface
//!     summarized so far. This feature is enabled by default and best serves
//!     all users of this crate's functionality.
//!   - **`core-functions`** enables a lower-level interface based on functions
//!     that take color coordinates. Hence it is simple enough to be exposed
//!     through a C-based FFI and can thereby be reused from other programming
//!     languages. In other words, it is the right interface when implementing a
//!     higher-level color abstraction for another programming language, such as
//!     Python.
//!
//! In fact, that is just how prettypretty's Python implementation reuses the
//! Rust core, which allowed me to get rid of the Python implementation of the
//! same functionality.
//!
//!
//! ## 5. BYOIO: Bring Your Own (Terminal) I/O
//!
//! Unlike the Python version, the Rust version of prettypretty does not (yet?)
//! include its own facilities for styled text or terminal I/O. Instead, it is
//! designed to be a lightweight addition that focuses on color management only.
//! To use this crate, an application must create its own instances of [`Theme`]
//! and [`ColorMatcher`]. While this crate contains one default theme,
//! surprisingly called [`DEFAULT_THEME`], that theme is suitable for tests but
//! no more.
//!
//! To fill in an accurate terminal theme, the application should use the ANSI
//! escape sequences
//! ```text
//! "{OSC}{10..=11};?{ST}"
//! ```
//! and
//! ```text
//! "{OSC}4;{0..=15};?{ST}"
//! ```
//! to query the terminal for its two default and 16 extended ANSI colors. The
//! responses are ANSI escape sequences with the exact same prefix as requests,
//! *before* the question mark, followed by the color in X Windows `rgb:`
//! format, followed by ST. Once you stripped the prefix and suffix from a
//! response, you can use the `FromStr` trait to parse the X Windows color
//! format into a color object.
//!
//! As usual, OSC stands for the character sequence `\x1b]` (escape, closing
//! square bracket) and ST stands for the character sequence `\x1b\\` (escape,
//! backslash). Some terminals answer with `\x0b` (bell) instead of ST.
//!
//!
//! ## Postscript
//!
//! As already illustrated above, most code examples come with their own color
//! swatches, which show the color values mentioned in the code. Where possible,
//! swatches use the exact same color spaces as the code (sRGB, Display P3,
//! Oklab, or Oklch). Otherwise, they fall back on an equivalent color in a
//! comparable color space (Oklrab and Oklrch).
//!
//! Implementing this crate's color support was a breeze. In part, that was
//! because I had built a prototype and a package in Python before and hence
//! knew what I was going for. In part, that was because I copied many of the
//! nitty-gritty color algorithms and conversion matrices from the most
//! excellent [Color.js](https://colorjs.io) by [Lea
//! Verou](http://lea.verou.me/) and [Chris Lilley](https://svgees.us/). Without
//! their work, I could not have gotten as far as quickly. Thank you!

mod collect;
mod color;
mod parser;
mod term_color;
mod util;

pub use color::core::{ColorSpace, InterpolationStrategy, DEFAULT_INTERPOLATION};

#[cfg(feature = "core-functions")]
pub use color::core::{
    clip, convert, delta_e_ok, find_closest, from_24_bit, in_gamut, interpolate, map_to_gamut,
    normalize, normalize_eq, prepare_to_interpolate, scale_lightness, to_24_bit, to_contrast,
    to_contrast_luminance, P3_CONTRAST, SRGB_CONTRAST,
};

#[cfg(feature = "core-functions")]
pub use parser::parse;

#[cfg(feature = "color-object")]
pub use collect::{ColorMatcher, Theme, DEFAULT_THEME};
#[cfg(feature = "color-object")]
pub use color::{Color, Interpolator, OkVersion};
#[cfg(feature = "color-object")]
pub use term_color::{AnsiColor, EightBitColor, EmbeddedRgb, GrayGradient, Layer, TrueColor};
#[cfg(feature = "color-object")]
pub use util::{ColorFormatError, Error, OutOfBoundsError};
