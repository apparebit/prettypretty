#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::{Attribute, Fidelity, FormatUpdate, Layer};
use crate::termco::Colorant;
use crate::Translator;

/// A terminal style.
///
/// A terminal style comprises text formatting, a foreground color, and a
/// background color. All three are optional. If none are provided, the style
/// denotes the default appearance. Since instances are immutable, terminal
/// styles can be arbitrarily reused.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    format: FormatUpdate,
    foreground: Option<Colorant>,
    background: Option<Colorant>,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Style {
    /// Credate a new empty style. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn py_new() -> Self {
        Self::default()
    }

    /// Create a new style with added bold formatting.
    pub fn bold(&self) -> Self {
        Self {
            format: self.format + Attribute::Bold,
            ..self.clone()
        }
    }

    /// Create a new style with added thin formatting.
    pub fn thin(&self) -> Self {
        Self {
            format: self.format + Attribute::Thin,
            ..self.clone()
        }
    }

    /// Create a new style with added italic formatting.
    pub fn italic(&self) -> Self {
        Self {
            format: self.format + Attribute::Italic,
            ..self.clone()
        }
    }

    /// Create a new style with added underlined formatting.
    pub fn underlined(&self) -> Self {
        Self {
            format: self.format + Attribute::Underlined,
            ..self.clone()
        }
    }

    /// Create a new style with added blinking formatting.
    pub fn blinking(&self) -> Self {
        Self {
            format: self.format + Attribute::Blinking,
            ..self.clone()
        }
    }

    /// Create a new style with added reversed formatting.
    pub fn reversed(&self) -> Self {
        Self {
            format: self.format + Attribute::Reversed,
            ..self.clone()
        }
    }

    /// Create a new style with added hidden formatting.
    pub fn hidden(&self) -> Self {
        Self {
            format: self.format + Attribute::Hidden,
            ..self.clone()
        }
    }

    /// Create a new style with added stricken formatting.
    pub fn stricken(&self) -> Self {
        Self {
            format: self.format + Attribute::Stricken,
            ..self.clone()
        }
    }

    // Create a new style with the given foreground color.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "with_foreground")]
    pub fn py_with_foreground(
        &self,
        #[pyo3(from_py_with = "crate::termco::into_colorant")] colorant: Colorant,
    ) -> Self {
        self.with_foreground(colorant)
    }

    // Create a new style with the given background color.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "with_background")]
    pub fn py_with_background(
        &self,
        #[pyo3(from_py_with = "crate::termco::into_colorant")] colorant: Colorant,
    ) -> Self {
        self.with_background(colorant)
    }

    /// Determine this style's fidelity.
    ///
    /// This method computes the maximum fidelity of this style's format,
    /// foreground color, and background color.
    pub fn fidelity(&self) -> Fidelity {
        *(!self.format.is_empty())
            .then_some(Fidelity::NoColor)
            .iter()
            .chain(self.foreground.as_ref().map(|c| c.into()).iter())
            .chain(self.background.as_ref().map(|c| c.into()).iter())
            .max()
            .unwrap_or(&Fidelity::Plain)
    }

    /// Cap this style to the given fidelity.
    pub fn cap(&self, fidelity: Fidelity, translator: &Translator) -> Self {
        let format = self.format.cap(fidelity);

        let foreground = if let Some(ref colorant) = self.foreground {
            translator.cap_colorant(colorant, fidelity)
        } else {
            None
        };

        let background = if let Some(ref colorant) = self.background {
            translator.cap_colorant(colorant, fidelity)
        } else {
            None
        };

        Self {
            format,
            foreground,
            background,
        }
    }

    /// Determine whether this style is the default style.
    pub fn is_default(&self) -> bool {
        self.format.is_empty() && self.foreground.is_none() && self.background.is_none()
    }

    /// Get this style's formatting.
    pub fn format(&self) -> FormatUpdate {
        self.format
    }

    /// Get this style's foreground color.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "foreground")]
    pub fn py_foreground(&self) -> Option<Colorant> {
        self.foreground().cloned()
    }

    /// Get this style's background color.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "background")]
    pub fn py_background(&self) -> Option<Colorant> {
        self.background().cloned()
    }

    /// Negate this style. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __neg__(&self) -> Self {
        -self
    }

    /// Get this style's debug representation. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    /// Get this style's string representation. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl Style {
    /// Create a new style with the given foreground color.
    pub fn with_foreground(&self, color: impl Into<Colorant>) -> Self {
        Self {
            foreground: Some(color.into()),
            ..self.clone()
        }
    }

    /// Create a new style with the given background color.
    pub fn with_background(&self, color: impl Into<Colorant>) -> Self {
        Self {
            format: self.format,
            foreground: self.foreground.clone(),
            background: Some(color.into()),
        }
    }

    /// Get this style's foreground colorant.
    pub fn foreground(&self) -> Option<&Colorant> {
        self.foreground.as_ref()
    }

    /// Get this style's background colorant.
    pub fn background(&self) -> Option<&Colorant> {
        self.background.as_ref()
    }
}

impl std::ops::Neg for &Style {
    type Output = Style;

    fn neg(self) -> Self::Output {
        Style {
            format: -self.format,
            foreground: self.foreground.as_ref().and_then(|c| -c),
            background: self.background.as_ref().and_then(|c| -c),
        }
    }
}

impl std::ops::Neg for Style {
    type Output = Style;

    fn neg(self) -> Self::Output {
        -(&self)
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_default() {
            return Ok(());
        }

        let mut first = true;
        macro_rules! maybe_emit_semicolon {
            () => {
                if first {
                    #[allow(unused_assignments)]
                    {
                        first = false;
                    }
                } else {
                    f.write_str(";")?;
                }
            };
        }

        f.write_str("\x1b[")?;
        for attr in self.format.disable().attributes() {
            maybe_emit_semicolon!();
            write!(f, "{}", attr.disable_sgr())?;
        }
        for attr in self.format.enable().attributes() {
            maybe_emit_semicolon!();
            write!(f, "{}", attr.enable_sgr())?;
        }
        if let Some(ref colorant) = self.foreground {
            maybe_emit_semicolon!();
            colorant.write_sgr_params(Layer::Foreground, f)?;
        }
        if let Some(ref colorant) = self.background {
            maybe_emit_semicolon!();
            colorant.write_sgr_params(Layer::Background, f)?;
        }
        f.write_str("m")
    }
}

// ----------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use crate::termco::EmbeddedRgb;

    #[test]
    fn test_style() {
        use super::Attribute::*;

        let style = Style::default();
        assert_eq!(style.format(), FormatUpdate::default());
        assert_eq!(style.foreground(), None);
        assert_eq!(style.background(), None);
        assert_eq!(format!("{}", style), "");
        let negated = -&style;
        assert_eq!(negated.format(), FormatUpdate::default());
        assert_eq!(negated.foreground(), None);
        assert_eq!(negated.background(), None);
        assert_eq!(format!("{}", negated), "");

        let style = style.bold().underlined();
        assert_eq!(style.format(), (Bold + Underlined).into());
        assert_eq!(style.foreground(), None);
        assert_eq!(style.background(), None);
        assert_eq!(format!("{}", style), "\x1b[1;4m");
        let negated = -&style;
        assert_eq!(negated.format(), -(Bold + Underlined));
        assert_eq!(negated.foreground(), None);
        assert_eq!(negated.background(), None);
        assert_eq!(format!("{}", negated), "\x1b[22;24m");

        let style = style.with_foreground(EmbeddedRgb::new(5, 3, 1).unwrap());
        assert_eq!(style.format(), (Bold + Underlined).into());
        assert_eq!(
            style.foreground(),
            Some(&Colorant::Embedded(EmbeddedRgb::new(5, 3, 1).unwrap()))
        );
        assert_eq!(style.background(), None);
        assert_eq!(format!("{}", style), "\x1b[1;4;38;5;215m");
        let negated = -style;
        assert_eq!(negated.format(), -(Bold + Underlined));
        assert_eq!(negated.foreground(), Some(&Colorant::Default()));
        assert_eq!(negated.background(), None);
        assert_eq!(format!("{}", negated), "\x1b[22;24;39m");
    }
}
