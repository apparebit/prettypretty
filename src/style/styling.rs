#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use std::cell::RefCell;

use super::{format::Format, Colorant, EmbeddedRgb, Fidelity, GrayGradient, Layer, TrueColor};
use crate::trans::Translator;

// ================================================================================================

/// The state shared between style builders and styles.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct StyleData {
    reset: bool,
    format: Option<Format>,
    foreground: Option<Colorant>,
    background: Option<Colorant>,
}

// ================================================================================================

/// Create a new stylist, i.e., style builder.
#[cfg_attr(feature = "pyffi", pyfunction)]
pub const fn stylist() -> Stylist {
    Stylist::new()
}

/// A stylist is a builder of styles.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.style"))]
#[derive(Clone, Debug, Default)]
pub struct Stylist {
    data: RefCell<StyleData>,
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl Stylist {
    /// Create a new style builder.
    #[new]
    pub fn py_new() -> Self {
        Self::new()
    }

    /// Create a new style builder that has a reset token as first token.
    #[pyo3(name = "with_reset")]
    #[staticmethod]
    pub fn py_with_reset() -> Self {
        Self::with_reset()
    }

    /// Add bold formatting to this style builder.
    #[pyo3(name = "bold")]
    #[inline]
    pub fn py_bold(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.bold();
        slf
    }

    /// Add thin formatting to this style builder.
    #[pyo3(name = "thin")]
    #[inline]
    pub fn py_thin(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.thin();
        slf
    }

    /// Add italic formatting to this style builder.
    #[pyo3(name = "italic")]
    #[inline]
    pub fn py_italic(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.italic();
        slf
    }

    /// Add underlined formatting to this style builder.
    #[pyo3(name = "underlined")]
    #[inline]
    pub fn py_underlined(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.underlined();
        slf
    }

    /// Add blinking formatting to this style builder.
    #[pyo3(name = "blinking")]
    #[inline]
    pub fn py_blinking(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.blinking();
        slf
    }

    /// Add reversed formatting to this style builder.
    #[pyo3(name = "reversed")]
    #[inline]
    pub fn py_reversed(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.reversed();
        slf
    }

    /// Add hidden formatting to this style builder.
    #[pyo3(name = "hidden")]
    #[inline]
    pub fn py_hidden(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.hidden();
        slf
    }

    /// Add stricken formatting to this style builder.
    #[pyo3(name = "stricken")]
    #[inline]
    pub fn py_stricken(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.stricken();
        slf
    }

    /// Add the embedded RGB foreground color to this style builder.
    #[pyo3(name = "embedded_rgb_fg")]
    pub fn py_embedded_rgb_fg(slf: PyRef<'_, Self>, r: u8, g: u8, b: u8) -> PyRef<'_, Self> {
        slf.embedded_rgb_fg(r, g, b);
        slf
    }

    /// Add the gray foreground gradient to this style builder.
    #[pyo3(name = "gray_fg")]
    pub fn py_gray_fg(slf: PyRef<'_, Self>, level: u8) -> PyRef<'_, Self> {
        slf.gray_fg(level);
        slf
    }

    /// Add the true RGB foreground color to this style builder.
    #[pyo3(name = "rgb_fg")]
    pub fn py_rgb_fg(slf: PyRef<'_, Self>, r: u8, g: u8, b: u8) -> PyRef<'_, Self> {
        slf.rgb_fg(r, g, b);
        slf
    }

    /// Add the foreground color to this style builder.
    #[pyo3(name = "fg")]
    pub fn py_fg(
        slf: PyRef<'_, Self>,
        #[pyo3(from_py_with = "crate::style::into_colorant")] colorant: Colorant,
    ) -> PyRef<'_, Self> {
        slf.fg(colorant);
        slf
    }

    // Add the foreground color to this style builder.
    #[pyo3(name = "foreground")]
    pub fn py_foreground(
        slf: PyRef<'_, Self>,
        #[pyo3(from_py_with = "crate::style::into_colorant")] colorant: Colorant,
    ) -> PyRef<'_, Self> {
        slf.foreground(colorant);
        slf
    }

    /// Add the embedded RGB background color to this style builder.
    #[pyo3(name = "embedded_rgb_bg")]
    pub fn py_embedded_rgb_bg(slf: PyRef<'_, Self>, r: u8, g: u8, b: u8) -> PyRef<'_, Self> {
        slf.embedded_rgb_bg(r, g, b);
        slf
    }

    /// Add the gray background gradient to this style builder.
    #[pyo3(name = "gray_bg")]
    pub fn py_gray_bg(slf: PyRef<'_, Self>, level: u8) -> PyRef<'_, Self> {
        slf.gray_bg(level);
        slf
    }

    /// Add the true RGB background color to this style builder.
    #[pyo3(name = "rgb_bg")]
    pub fn py_rgb_bg(slf: PyRef<'_, Self>, r: u8, g: u8, b: u8) -> PyRef<'_, Self> {
        slf.rgb_bg(r, g, b);
        slf
    }

    /// Add the background color to this style builder.
    #[pyo3(name = "bg")]
    pub fn py_bg(
        slf: PyRef<'_, Self>,
        #[pyo3(from_py_with = "crate::style::into_colorant")] colorant: Colorant,
    ) -> PyRef<'_, Self> {
        slf.bg(colorant);
        slf
    }

    /// Add the background color to this style builder.
    #[pyo3(name = "background")]
    pub fn py_background(
        slf: PyRef<'_, Self>,
        #[pyo3(from_py_with = "crate::style::into_colorant")] colorant: Colorant,
    ) -> PyRef<'_, Self> {
        slf.background(colorant);
        slf
    }

    /// Finish building and return a new style.
    ///
    /// This method moves the builder's data into the new style and leaves an
    /// empty builder behind.
    pub fn et_voila(&self) -> Style {
        Style {
            data: self.data.take(),
        }
    }

    /// Finish building and return a new style.
    ///
    /// This method moves the builder's data into the new style and leaves an
    /// empty builder behind.  Consider using [`Stylist::et_voila`] instead.
    pub fn build(&self) -> Style {
        Style {
            data: self.data.take(),
        }
    }

    /// Render a debug representation of this stylist. <i
    /// class=python-only>Python only!</i>
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl Stylist {
    /// Create a new style builder.
    pub const fn new() -> Self {
        Self {
            data: RefCell::new(StyleData {
                reset: false,
                format: None,
                foreground: None,
                background: None,
            }),
        }
    }

    /// Create a new style builder for a style that resets the terminal
    /// appearance.
    pub fn with_reset() -> Self {
        Self {
            data: RefCell::new(StyleData {
                reset: true,
                format: None,
                foreground: None,
                background: None,
            }),
        }
    }

    /// Add bold formatting to this style builder.
    pub fn bold(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().bold());
        self
    }

    /// Add thin formatting to this style builder.
    pub fn thin(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().thin());
        self
    }

    /// Add italic formatting to this style builder.
    pub fn italic(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().italic());
        self
    }

    /// Add underlined formatting to this style builder.
    pub fn underlined(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().underlined());
        self
    }

    /// Add blinking formatting to this style builder.
    pub fn blinking(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().blinking());
        self
    }

    /// Add reversed formatting to this style builder.
    pub fn reversed(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().reversed());
        self
    }

    /// Add hidden formatting to this style builder.
    pub fn hidden(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().hidden());
        self
    }

    /// Add stricken formatting to this style builder.
    pub fn stricken(&self) -> &Self {
        let mut data = self.data.borrow_mut();
        data.format = Some(data.format.unwrap_or_default().stricken());
        self
    }

    /// Add the embedded RGB foreground color to this style builder.
    ///
    /// If any of the given components is invalid, i.e., greater than 5, this
    /// method sets the foreground color to `None`.
    pub fn embedded_rgb_fg(&self, r: u8, g: u8, b: u8) -> &Self {
        let mut data = self.data.borrow_mut();
        data.foreground = EmbeddedRgb::new(r, g, b).map(|c| c.into()).ok();
        self
    }

    /// Add the gray foreground gradient to this style builder.
    ///
    /// If the given level is invalid, i.e., greater than 23, this method sets
    /// the foreground color to `None`.
    pub fn gray_fg(&self, level: u8) -> &Self {
        let mut data = self.data.borrow_mut();
        data.foreground = GrayGradient::new(level).map(|c| c.into()).ok();
        self
    }

    /// Add the true RGB foreground color to this style builder.
    pub fn rgb_fg(&self, r: u8, g: u8, b: u8) -> &Self {
        let mut data = self.data.borrow_mut();
        data.foreground = Some(TrueColor::new(r, g, b).into());
        self
    }

    /// Add the foreground color to this style builder.
    pub fn fg(&self, color: impl Into<Colorant>) -> &Self {
        let mut data = self.data.borrow_mut();
        data.foreground = Some(color.into());
        self
    }

    /// Add the foreground color to this style builder.
    pub fn foreground(&self, color: impl Into<Colorant>) -> &Self {
        let mut data = self.data.borrow_mut();
        data.foreground = Some(color.into());
        self
    }

    /// Add the embedded RGB background color to this style builder.
    ///
    /// If any of the given components is invalid, i.e., greater than 5, this
    /// method sets the background color to `None`.
    pub fn embedded_rgb_bg(&self, r: u8, g: u8, b: u8) -> &Self {
        let mut data = self.data.borrow_mut();
        data.background = EmbeddedRgb::new(r, g, b).map(|c| c.into()).ok();
        self
    }

    /// Add the gray background gradient to this style builder.
    ///
    /// If the given level is invalid, i.e., greater than 23, this method sets
    /// the background color to `None`.
    pub fn gray_bg(&self, level: u8) -> &Self {
        let mut data = self.data.borrow_mut();
        data.background = GrayGradient::new(level).map(|c| c.into()).ok();
        self
    }

    /// Add the true RGB background color to this style builder.
    pub fn rgb_bg(&self, r: u8, g: u8, b: u8) -> &Self {
        let mut data = self.data.borrow_mut();
        data.background = Some(TrueColor::new(r, g, b).into());
        self
    }

    /// Add the background color to this style builder.
    pub fn bg(&self, color: impl Into<Colorant>) -> &Self {
        let mut data = self.data.borrow_mut();
        data.background = Some(color.into());
        self
    }

    /// Add the background color to this style builder.
    pub fn background(&self, color: impl Into<Colorant>) -> &Self {
        let mut data = self.data.borrow_mut();
        data.background = Some(color.into());
        self
    }

    /// Finish building and return a new style.
    ///
    /// This method moves the builder's data into the new style and leaves an
    /// empty builder behind.
    #[cfg(not(feature = "pyffi"))]
    pub fn et_voila(&self) -> Style {
        Style {
            data: self.data.take(),
        }
    }

    /// Finish building and return a new style.
    ///
    /// This method moves the builder's data into the new style and leaves an
    /// empty builder behind.  Consider using [`Stylist::et_voila`] instead.
    #[cfg(not(feature = "pyffi"))]
    pub fn build(&self) -> Style {
        Style {
            data: self.data.take(),
        }
    }
}

// ================================================================================================

/// A terminal style: Reset, text format, and foreground/background colors.
///
/// All four components are optional. In fact, use of the appearance reset
/// should be exceptional.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    data: StyleData,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Style {
    /// Create a new style builder.
    ///
    /// Consider using [`stylist()`] instead.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn builder() -> Stylist {
        Stylist::new()
    }

    /// Determine whether this style is empty.
    pub fn is_empty(&self) -> bool {
        !self.data.reset
            && self.data.format.is_none()
            && self.data.foreground.is_none()
            && self.data.background.is_none()
    }

    /// Determine whether this style resets the terminal's appearance
    pub fn has_reset(&self) -> bool {
        self.data.reset
    }

    /// Get this style's format.
    pub fn format(&self) -> Option<Format> {
        self.data.format
    }

    /// Determine whether this style includes color.
    pub fn has_color(&self) -> bool {
        self.data.foreground.is_some() || self.data.background.is_some()
    }

    /// Determine this style's foreground color.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "foreground")]
    pub fn py_foreground(&self) -> Option<Colorant> {
        self.data.foreground.clone()
    }

    /// Determine this style's background color.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "background")]
    pub fn py_background(&self) -> Option<Colorant> {
        self.data.background.clone()
    }

    /// Determine this style's fidelity.
    ///
    /// This method computes the maximum fidelity of the fidelities implied by
    /// this style's reset flag, format, foreground color, and background color.
    pub fn fidelity(&self) -> Fidelity {
        *self
            .data
            .reset
            .then_some(Fidelity::NoColor)
            .iter()
            .chain(self.data.format.map(|_| Fidelity::NoColor).iter())
            .chain(self.data.foreground.as_ref().map(|c| c.into()).iter())
            .chain(self.data.background.as_ref().map(|c| c.into()).iter())
            .max()
            .unwrap_or(&Fidelity::Plain)
    }

    /// Cap this style to the given fidelity.
    pub fn cap(&self, fidelity: Fidelity, translator: &Translator) -> Self {
        let reset = if matches!(fidelity, Fidelity::Plain) {
            false
        } else {
            self.data.reset
        };

        let format = self.data.format.and_then(|f| f.cap(fidelity));

        let foreground = if let Some(ref colorant) = self.data.foreground {
            translator.cap_colorant(colorant, fidelity)
        } else {
            None
        };

        let background = if let Some(ref colorant) = self.data.background {
            translator.cap_colorant(colorant, fidelity)
        } else {
            None
        };

        Self {
            data: StyleData {
                reset,
                format,
                foreground,
                background,
            },
        }
    }

    /// Determine the number of SGR parameters required by this style.
    ///
    /// If this style includes a high-resolution color, this method returns
    /// `None`. Otherwise, it returns some number *n*, with
    /// 1&nbsp;<=&nbsp;*n*&nbsp;<=&nbsp;18.
    pub fn sgr_parameter_count(&self) -> Option<usize> {
        let mut count: usize = 0;

        if self.data.reset {
            count += 1;
        }

        if let Some(format) = self.data.format {
            count += format.attribute_count() as usize;
        }

        if let Some(color) = self.data.foreground.as_ref() {
            if let Some(number) = color.sgr_parameter_count() {
                count += number;
            } else {
                return None;
            }
        }

        if let Some(color) = self.data.background.as_ref() {
            if let Some(number) = color.sgr_parameter_count() {
                count += number;
            } else {
                return None;
            }
        }

        Some(count)
    }

    /// Get the SGR parameters for this style.
    pub fn sgr_parameters(&self) -> Vec<u8> {
        self.data
            .reset
            .then_some(0_u8)
            .into_iter()
            .chain(
                self.data
                    .format
                    .map(|f| f.sgr_parameters())
                    .into_iter()
                    .flatten(),
            )
            .chain(
                self.data
                    .foreground
                    .as_ref()
                    .and_then(|c| c.sgr_parameters(Layer::Foreground))
                    .into_iter()
                    .flatten(),
            )
            .chain(
                self.data
                    .background
                    .as_ref()
                    .and_then(|c| c.sgr_parameters(Layer::Background))
                    .into_iter()
                    .flatten(),
            )
            .collect()
    }

    /// Invert this style. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __invert__(&self) -> Self {
        !self
    }

    /// Render a debug representation of this style. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    /// Render this style as an ANSI SGR escape sequence. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl Style {
    /// Create a new style builder.
    ///
    /// Consider using [`stylist`] instead.
    #[cfg(not(feature = "pyffi"))]
    pub fn builder() -> Stylist {
        Stylist::new()
    }

    /// Determine this style's foreground color.
    pub fn foreground(&self) -> Option<&Colorant> {
        self.data.foreground.as_ref()
    }

    /// Determine this style's background color.
    pub fn background(&self) -> Option<&Colorant> {
        self.data.background.as_ref()
    }
}

impl std::ops::Not for &Style {
    type Output = Style;

    /// Negate this style.
    ///
    /// This method returns the style to restore the terminal's default
    /// appearance from this style, which may be empty.
    fn not(self) -> Self::Output {
        Style {
            data: StyleData {
                reset: false,
                format: self.data.format.map(|f| !f),
                foreground: self.data.foreground.as_ref().and_then(|c| !c),
                background: self.data.background.as_ref().and_then(|c| !c),
            },
        }
    }
}

impl std::ops::Not for Style {
    type Output = Style;

    fn not(self) -> Self::Output {
        !(&self)
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[")?;
        for (index, param) in self.sgr_parameters().iter().enumerate() {
            if 0 < index {
                f.write_str(";")?;
            }
            f.write_fmt(format_args!("{}", *param))?;
        }
        f.write_str("m")
    }
}
