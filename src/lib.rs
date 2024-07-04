#![doc(
    html_logo_url = "https://repository-images.githubusercontent.com/796446264/7483a099-9280-489e-b1b0-119497d8c2da"
)]

//! # Pretty 🌸 Pretty
//!
//! This library brings 2020s color science to 1970s terminals to help build
//! awesome looking and adaptable terminal user interfaces. It supports
//! high-resolution colors, accurate conversion between color spaces, finding
//! the closest matching color, gamut testing and mapping, and computing text
//! contrast.
//!
#![doc = include_str!("style.html")]
//!
//! ## 1. High-Resolution Colors
//!
//! High-resolution colors from the 2020s have floating point coordinates and
//! explicit color spaces:
//!
//!   * [`ColorSpace`] enumerates supported color spaces.
//!   * [`Color`] combines a color space and three floating point coordinates
//!     into a precise color representation.
//!
//! Much of prettypretty's functionality is accessible through [`Color`]'s
//! methods. That includes:
//!
//!   * Access to color space and coordinates [`space`](Color::space),
//!     [`as_ref`](Color::as_ref)
//!   * Testing for color [`is_gray`](Color::is_gray)
//!   * Conversion between color spaces [`to`](Color::to)
//!   * Gamut testing [`in_gamut`](Color::in_gamut), clipping
//!     [`clip`](Color::clip), and mapping [`to_gamut`](Color::to_gamut)
//!   * Lightening [`lighten`](Color::lighten) and darkening
//!     [`darken`](Color::darken)
//!   * Perceptual contrast [`contrast_again`](Color::contrast_against),
//!     [`use_black_text`](Color::use_black_text),
//!     [`use_black_background`](Color::use_black_background)
//!   * Color difference [`distance`](Color::distance),
//!     [`find_closest_ok`](Color::find_closest_ok),
//!     [`find_closest`](Color::find_closest)
//!   * Interpolation [`interpolate`](Color::interpolate)
//!
//! The example below illustrates how to use [`Color`]. First, it instantiates a
//! color in the polar Oklch color space, converts it to Display P3, and tests
//! whether it is in gamut—it is. Next, it converts the color to sRGB and tests
//! whether it is in gamut—it is not. Finally, it maps the color into sRGB's
//! gamut. If you are reading this on a wide-gamut screen, the color swatch
//! below the code should show two distinct shades of pink, with the left one
//! considerably more intense.
//!
//! ```
//! # use prettypretty::{Color, ColorSpace};
//! let oklch = Color::oklch(0.716, 0.349, 335.0);
//! let p3 = oklch.to(ColorSpace::DisplayP3);
//! assert!(p3.in_gamut());
//!
//! let not_srgb = oklch.to(ColorSpace::Srgb);
//! assert!(!not_srgb.in_gamut());
//!
//! let srgb = not_srgb.to_gamut();
//! assert_eq!(srgb, Color::srgb(1.0, 0.15942348587138203, 0.9222706101768445));
//! ```
//! <div class=color-swatch>
//! <div style="background-color: oklch(0.716 0.349 335);"></div>
//! <div style="background-color: color(srgb 1 0.15942 0.92227);"></div>
//! </div>
//!
//! ### Different Color Spaces for Different Tasks
//!
//! Instead of creating a color out of nothing (well, *numbers* ...), we could
//! as easily modify an existing color, for example, by pushing lightness,
//! reducing chroma, or shifting the hue. As it turns out, the perceptually
//! uniform polar coordinates of Oklch and Oklrch make for intuitive color
//! manipulation.
//!
//! If we need to compare colors, however, then the Cartesian coordinates of
//! Oklab and Oklrab support a straight-forward Euclidian distance metric.
//!
//! Alas, when working with the Oklab variations, it's easy to end up with
//! colors that are out of gamut for any particular color space or even outside
//! the spectrum of visible light. So we always should check whether colors are
//! in gamut and clip or gamut map those that aren't.
//!
//!
//! ## 2. Terminal Colors
//!
//! In contrast to high-resolution colors, which fit into a nicely uniform
//! representation with three coordinates, terminal color formats from the 1970s
//! and 1980s may not even have coordinates, only integer index values. ANSI
//! escape codes support four different kinds of colors:
//!
//!   * [`DefaultColor`], the default foreground and background colors.
//!   * [`AnsiColor`], the 16 extended ANSI colors.
//!   * 8-bit indexed colors, which comprise [`AnsiColor`], [`EmbeddedRgb`],
//!     and [`GrayGradient`].
//!   * [`TrueColor`], 24-bit RGB colors.
//!
//! Treating these color types uniformly requires one more:
//!
//!   * [`TerminalColor`] combines the different types of terminal colors into
//!     one coherent type.
//!
//! [`DefaultColor`] represents the default foreground and background
//! colors. They have their own ANSI escape codes and hence are distinct from
//! the ANSI colors. Typically, they can also be independently themed.
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
//! [`TrueColor`] represents 24-bit RGB colors. Even in the early 1990s, when
//! 24-bit graphic cards first became widely available, the term was a misnomer.
//! For example, Kodak's [Photo CD](https://en.wikipedia.org/wiki/Photo_CD) was
//! introduced at the same time and had a considerably wider gamut than the
//! device RGB of graphic cards. Alas, the term lives on. Terminal emulators
//! often advertise support for 16 million colors by setting the `COLORTERM`
//! environment variable to `truecolor`.
//!
//! Finally, [`TerminalColor`] combines the just listed types into a single
//! coherent type of terminal colors. It does *not* model that ANSI colors can
//! appear as themselves and as 8-bit indexed colors. This crate used to include
//! the corresponding wrapper type, but it offered too little functionality to
//! justify having a wrapper of a wrapper of a type. Since all wrapped colors
//! implement `Into<TerminalColor>`, there should be little need for manually
//! wrapping terminal colors in Rust code.
//!
//! The example code below illustrates how [`AnsiColor`], [`EmbeddedRgb`], and
//! [`GrayGradient`] abstract over the underlying 8-bit index space while also
//! providing convenient access to RGB coordinates and gray levels. Embedded RGB
//! and gray gradient colors also nicely convert to true as well as
//! high-resolutions colors, but ANSI colors do not.
//!
//! ```
//! # use prettypretty::{AnsiColor, Color, EmbeddedRgb, GrayGradient, TerminalColor, TrueColor};
//! # use prettypretty::OutOfBoundsError;
//! let red = AnsiColor::BrightRed;
//! assert_eq!(u8::from(red), 9);
//! // What's the color value of ANSI red? We don't know!
//!
//! let purple = EmbeddedRgb::new(3, 1, 4)?;
//! let index = 16 + 3 * 36 + 1 * 6 + 4 * 1;
//! assert_eq!(index, 134);
//! assert_eq!(u8::from(purple), index);
//! assert_eq!(TrueColor::from(purple), TrueColor::new(175, 95, 215));
//! assert_eq!(Color::from(purple), Color::from_24bit(175, 95, 215));
//!
//! let gray = GrayGradient::new(18)?;
//! let index = 232 + 18;
//! assert_eq!(index, 250);
//! assert_eq!(gray.level(), 18);
//! assert_eq!(u8::from(gray), index);
//! assert_eq!(TrueColor::from(gray), TrueColor::new(188, 188, 188));
//! assert_eq!(Color::from(gray), Color::from_24bit(188, 188, 188));
//!
//! let green = TerminalColor::from(71);
//! assert!(matches!(green, TerminalColor::Rgb6 { .. }));
//! if let TerminalColor::Rgb6 { color: also_green } = green {
//!     assert_eq!(also_green[0], 1);
//!     assert_eq!(also_green[1], 3);
//!     assert_eq!(also_green[2], 1);
//!     assert_eq!(TrueColor::from(also_green), TrueColor::new(95, 175, 95));
//!     assert_eq!(Color::from(also_green), Color::from_24bit(95, 175, 95));
//! } else {
//!     unreachable!("green is an embedded RGB color")
//! }
//! # Ok::<(), OutOfBoundsError>(())
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
//! easily translate between terminal and high-resolution colors:
//!
//!   * [`Sampler`] provides the logic and state for translating between
//!     terminal and high-resolution colors.
//!
//! The immediate challenge when tying to translate terminal colors to high
//! resolution colors is that default and ANSI colors are only abstract and have
//! no intrinsic values. Terminal emulators address this lack of color values by
//! making colors configurable through [color
//! themes](https://gogh-co.github.io/Gogh/). While prettypretty requires some
//! sort of mapping too, asking users to configure colors again is the wrong
//! approach. Instead prettypretty uses ANSI escape codes to query the terminal
//! for its current theme colors and then passes those 18 colors to
//! [`Sampler::new`].
//!
//! In other words, theme colors take care of translation from terminal to
//! high-resolution colors. The difficulty of translation in the other
//! direction, from high-resolution to terminal colors, very much depends on the
//! target colors:
//!
//!   * **24-bit colors**: In the best case, when the source color is in-gamut
//!     for sRGB and the target are 24-bit "true" colors, a loss of numeric
//!     resolution is the only concern. It probably is imperceptible as well.
//!     However, if the source color is out of sRGB gamut, even when still
//!     targeting 24-bit colors and using gamut-mapping, which [`Sampler`] does,
//!     the difference between source and target colors becomes clearly
//!     noticeable. It only becomes more glaring when targeting 8-bit or ANSI
//!     colors.
//!
//!   * **8-bit colors**: While accuracy necessarily suffers when targeting
//!     8-bit colors, the small number of colors makes brute force search
//!     feasible for finding the closest target color.
//!     [`Sampler::to_closest_8bit`] does just that, though it only considers
//!     embedded RGB and gray gradient colors. ANSI colors aren't usually
//!     assigned coordinates based on some formula and hence tend to stick out
//!     amongst embedded RGB and gray gradient colors.
//!
//!   * **ANSI colors**: [`Sampler::to_closest_ansi`] is the equivalent method
//!     for targeting ANSI colors. As that method's documentation illustrates,
//!     it may produce subpar results because there are too few potential colors
//!     to match. Hence, I developed a more suitable algorithm that not only
//!     uses color pragmatics, i.e., the coordinates of theme colors, but also
//!     color semantics, i.e., their intended appearance. The implementation in
//!     [`Sampler::to_ansi_hue_lightness`] first uses hue to find a pair of
//!     regular and bright colors and second uses lightness to pick the closer
//!     one. However, that won't work if the theme colors violate standard
//!     semantics. Hence [`Sampler::to_ansi`] transparently picks the best
//!     possible method.
//!
//! The example below illustrates the use of a sampler instance for translation
//! between ANSI and high-resolution colors. `VGA_COLORS` contains the theme
//! colors in the order expected by [`Sampler::new`]. That places the two
//! default colors before the ANSI colors, which explains the `+ 2` in the first
//! line's index expression. As long as your application sticks to using
//! [`Sampler`]'s interface, it won't have to adjust indexes like that.
//!
//! ```
//! # use prettypretty::{AnsiColor, Color, ColorFormatError, Layer, Sampler, VGA_COLORS};
//! # use prettypretty::OkVersion;
//! # use std::str::FromStr;
//! let red = &VGA_COLORS[AnsiColor::BrightRed as usize + 2];
//! assert_eq!(red, &Color::srgb(1.0, 0.333333333333333, 0.333333333333333));
//!
//! let sampler = Sampler::new(OkVersion::Revised, VGA_COLORS.clone());
//! let also_red = &sampler.resolve(AnsiColor::BrightRed);
//! assert_eq!(red, also_red);
//!
//! let yellow = Color::from_str("#FFE06C")?;
//! let bright_yellow = sampler.to_closest_ansi(&yellow);
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
//! This crate has two feature flags:
//!
//!   - **`f64`**: This feature is enabled by default. When disabled, the crate
//!     uses `f32`. In either case, the currently active floating point type is
//!     [`Float`] and the same-sized unsigned integer bits are [`Bits`].
//!   - **`pyffi`**: This feature is disabled by default. When enabled, this
//!     crate uses [PyO3](https://pyo3.rs/) and
//!     [Maturin](https://www.maturin.rs) to export an extension module for
//!     Python that makes this crate's Rust-based colors available in Python.
//!
//! The `pyffi` feature thus satisfies the need for easy scriptability. It helps
//! that PyO3's integration between Rust and Python goes well beyond what is
//! offered by other FFIs or stub generators. However, getting everything to
//! work was nonetheless painful. One major frustration is that annotations with
//! `#[new]` and `#[staticmethod]` don't respect `cfg_attr()` and thus requiring
//! the duplication of methods. But when the Rust version accepts `impl Into<T>`
//! arguments, then even the same `impl` block isn't enough and the code has to
//! be spread over one block for `feature="pyffi"`, one block for
//! `not(feature="pyffi")`, and sometimes even a third block for shared helper
//! methods.
//!
//! With exception of two methods on [`Color`], the Python version offers the
//! same functionality as the Rust version. Since Python does not support traits
//! such as `From` and `TryFrom`, prettypretty includes additional methods that
//! make the same functionality available. The documentation tags such
//! Python-only methods as <span class=python-only></span> and the few Rust-only
//! methods as <span class=rust-only></span>.
//!
//!
//! ## 5. BYOIO: Bring Your Own (Terminal) I/O
//!
//! Unlike the Python version, the Rust version of prettypretty does not (yet?)
//! include its own facilities for styled text or terminal I/O. Instead, it is
//! designed to be a lightweight addition that focuses on color management only.
//! To use this crate, an application should create its own instance of
//! [`Sampler`] with the colors of the current terminal theme.
//!
//! An application should use the ANSI escape sequences
//! ```text
//! "{OSC}{10..=11};?{ST}"
//! ```
//! and
//! ```text
//! "{OSC}4;{0..=15};?{ST}"
//! ```
//! to query the terminal for the two default and 16 extended ANSI colors. The
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
//! ## 6. Et Cetera
//!
//! Just like the above examples, most code blocks in the documentation come
//! with color swatches, which show the color values mentioned in the code.
//! Where possible, swatches use the exact same color spaces as the code (sRGB,
//! Display P3, Rec. 2020, Oklab, or Oklch). Otherwise, they fall back on an
//! equivalent color in a comparable color space (Oklrab and Oklrch).
//!
//! Implementing this crate's color support was a breeze. In part, that was
//! because I had built a prototype and a package in Python before and hence
//! knew what I was going for. In part, that was because I copied many of the
//! nitty-gritty color algorithms and conversion matrices from the most
//! excellent [Color.js](https://colorjs.io) by [Lea
//! Verou](http://lea.verou.me/) and [Chris Lilley](https://svgees.us/). Without
//! their work, I could not have gotten as far as quickly. Thank you! 🌸

/// The floating point type in use.
#[cfg(feature = "f64")]
pub type Float = f64;
/// The floating point type in use.
#[cfg(not(feature = "f64"))]
pub type Float = f32;

/// [`Float`]'s bits.
#[cfg(feature = "f64")]
pub type Bits = u64;
/// [`Float`]'s bits.
#[cfg(not(feature = "f64"))]
pub type Bits = u32;

mod collection;
mod core;
mod error;
mod object;
mod term_color;

pub use collection::{Sampler, ThemeEntry, ThemeEntryIterator, VGA_COLORS};
pub use core::{ColorFormatError, ColorSpace, HueInterpolation};
pub use error::OutOfBoundsError;
pub use object::{Color, Interpolator, OkVersion};
pub use term_color::{
    AnsiColor, DefaultColor, EmbeddedRgb, Fidelity, GrayGradient, Layer, TerminalColor, TrueColor,
};

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

/// Collect Python classes and functions implemented in Rust in the simulated
/// `color` module. <span class=python-only></span>
#[doc = include_str!("style.html")]
#[cfg(feature = "pyffi")]
#[pymodule]
pub fn color(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AnsiColor>()?;
    m.add_class::<Color>()?;
    m.add_class::<ColorSpace>()?;
    m.add_class::<DefaultColor>()?;
    m.add_class::<EmbeddedRgb>()?;
    m.add_class::<Fidelity>()?;
    m.add_class::<GrayGradient>()?;
    m.add_class::<HueInterpolation>()?;
    m.add_class::<Interpolator>()?;
    m.add_class::<Layer>()?;
    m.add_class::<OkVersion>()?;
    m.add_class::<Sampler>()?;
    m.add_class::<TerminalColor>()?;
    m.add_class::<ThemeEntry>()?;
    m.add_class::<ThemeEntryIterator>()?;
    m.add_class::<TrueColor>()?;
    Ok(())
}
