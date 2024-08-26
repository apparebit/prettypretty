//! Terminal colors and other stylistic flourishes of text enabled by ANSI
//! escapes.
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
//!
//! Additionally, this module defines [`Format`] for representing text styles
//! beyond colors, [`Style`] to unify *all* stylistic abstractions, and
//! [`RichText`] to combine styles with text.

mod color;
mod context;
mod format;

pub use color::{AnsiColor, DefaultColor, EmbeddedRgb, GrayGradient, TerminalColor, TrueColor};
pub use context::{Fidelity, Layer};
pub use format::{AllFormats, Format, FormatIterator, Formatting, RichText, Style};

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::context::fidelity_from_environment;
    use crate::error::OutOfBoundsError;
    use crate::style::{AnsiColor, EmbeddedRgb, Fidelity, GrayGradient, TerminalColor, TrueColor};
    use crate::util::FakeEnv;

    #[test]
    fn test_conversion() -> Result<(), OutOfBoundsError> {
        let magenta = AnsiColor::Magenta;
        assert_eq!(magenta as u8, 5);

        let green = EmbeddedRgb::new(0, 4, 0)?;
        assert_eq!(green.as_ref(), &[0, 4, 0]);
        assert_eq!(TrueColor::from(green), TrueColor::new(0, 215, 0));

        let gray = GrayGradient::new(12)?;
        assert_eq!(gray.level(), 12);
        assert_eq!(TrueColor::from(gray), TrueColor::new(128, 128, 128));

        let also_magenta = TerminalColor::Ansi {
            color: AnsiColor::Magenta,
        };
        let also_green = TerminalColor::Embedded { color: green };
        let also_gray = TerminalColor::Gray { color: gray };

        assert_eq!(also_magenta, TerminalColor::from(5));
        assert_eq!(also_green, TerminalColor::from(40));
        assert_eq!(also_gray, TerminalColor::from(244));

        assert!(<[u8; 3]>::try_from(also_magenta).is_err());
        assert_eq!(<[u8; 3]>::try_from(also_green), Ok([0_u8, 215, 0]));
        assert_eq!(<[u8; 3]>::try_from(also_gray), Ok([128_u8, 128, 128]));

        Ok(())
    }

    #[test]
    fn test_limits() -> Result<(), OutOfBoundsError> {
        let black_ansi = AnsiColor::try_from(0)?;
        assert_eq!(black_ansi, AnsiColor::Black);
        assert_eq!(u8::from(black_ansi), 0);
        let white_ansi = AnsiColor::try_from(15)?;
        assert_eq!(white_ansi, AnsiColor::BrightWhite);
        assert_eq!(u8::from(white_ansi), 15);

        let black_rgb = EmbeddedRgb::try_from(16)?;
        assert_eq!(*black_rgb.as_ref(), [0_u8, 0_u8, 0_u8]);
        assert_eq!(u8::from(black_rgb), 16);
        let white_rgb = EmbeddedRgb::try_from(231)?;
        assert_eq!(*white_rgb.as_ref(), [5_u8, 5_u8, 5_u8]);
        assert_eq!(u8::from(white_rgb), 231);

        let black_gray = GrayGradient::try_from(232)?;
        assert_eq!(black_gray.level(), 0);
        assert_eq!(u8::from(black_gray), 232);
        let white_gray = GrayGradient::try_from(255)?;
        assert_eq!(white_gray.level(), 23);
        assert_eq!(u8::from(white_gray), 255);

        Ok(())
    }

    #[test]
    fn test_fidelity() {
        let env = &mut FakeEnv::new();
        assert_eq!(fidelity_from_environment(env, true), Fidelity::Plain);
        env.set("TERM", "cygwin");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::Ansi);
        env.set("TERM_PROGRAM", "iTerm.app");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::EightBit);
        env.set("TERM_PROGRAM_VERSION", "3.5");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::Full);
        env.set("COLORTERM", "truecolor");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::Full);
        env.set("CI", "");
        env.set("APPVEYOR", "");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::Ansi);
        env.set("TF_BUILD", "");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::Ansi);
        env.set("NO_COLOR", "");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::Ansi);
        env.set("NO_COLOR", "1");
        assert_eq!(fidelity_from_environment(env, true), Fidelity::NoColor);
    }
}
