#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use std::cell::RefCell;
use std::sync::Arc;

use super::{format::Format, Fidelity, TerminalColor};
use crate::Color;

/// A style token represents the atomic units of styles, including colors and
/// other text formats.
///
/// Just like design tokens contribute to a larger design system,
/// [`StyleToken`]s contribute to [`Style`]s assembled with [`StyleBuilder`]s.
/// This naming scheme reserves the shortest, most generic term "style" for the
/// primary abstraction amongst the bunch.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StyleToken {
    Reset(),
    Format(Format),
    Foreground(TerminalColor),
    Background(TerminalColor),
    HiResForeground(Color),
    HiResBackground(Color),
    Link {
        text: String,
        href: String,
        id: Option<String>,
    },
    // ...
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl StyleToken {
    /// Determine the terminal fidelity necessary for rendering this style as
    /// is.
    ///
    /// This method necessarily return a fidelity higher than
    /// [`Fidelity::Plain`].
    pub fn fidelity(&self) -> Fidelity {
        match self {
            Self::Reset() => Fidelity::NoColor,
            Self::Format(_) => Fidelity::NoColor,
            Self::Foreground(color) => (*color).into(),
            Self::Background(color) => (*color).into(),
            Self::HiResForeground(_) => Fidelity::Full,
            Self::HiResBackground(_) => Fidelity::Full,
            Self::Link { .. } => Fidelity::NoColor,
        }
    }
}

// -------------------------------------------------------------------------------------

/// Create a new, empty [`StyleBuilder`].
///
/// A stylist creates styles, hence the function name.
#[cfg_attr(feature = "pyffi", pyfunction)]
pub fn stylist() -> StyleBuilder {
    StyleBuilder {
        tokens: RefCell::new(Vec::new()),
    }
}

/// A builder for fluently assembling styles from style tokens.
///
/// In departure from Rust conventions, new builders are created with the
/// [`stylist()`] function and new styles are forged with the
/// [`go`](StyleBuilder::go) method. To wit,
/// ```rust,ignore
/// let bold_style = stylist().bold().go();
/// ```
/// creates a bold style. These idiosyncratic choices are motivated by the
/// appearance of fluent style definitions, with the above example shorter and
/// more fluid than
/// ```rust,compile_fail
/// let bold_style = StyleBuilder::new().bold().build();
/// ```
#[cfg_attr(feature = "pyffi", pyclass(eq, module = "prettypretty.color.style"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleBuilder {
    tokens: RefCell<Vec<StyleToken>>,
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl StyleBuilder {
    /// Push a style reset onto this builder.
    #[pyo3(name = "reset")]
    #[inline]
    pub fn py_reset(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf.reset();
        slf
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

    /// Push a foreground color onto this style builder.
    #[pyo3(name = "foreground")]
    pub fn py_foreground(
        slf: PyRef<'_, Self>,
        #[pyo3(from_py_with = "crate::style::into_terminal_color")] color: TerminalColor,
    ) -> PyRef<'_, Self> {
        slf.foreground(color);
        slf
    }

    /// Push a background color onto this style builder.
    #[pyo3(name = "background")]
    pub fn py_background(
        slf: PyRef<'_, Self>,
        #[pyo3(from_py_with = "crate::style::into_terminal_color")] color: TerminalColor,
    ) -> PyRef<'_, Self> {
        slf.background(color);
        slf
    }

    /// Push a high-resolution foreground color onto this style builder.
    #[pyo3(name = "hires_foreground")]
    pub fn py_hires_foreground(slf: PyRef<'_, Self>, color: Color) -> PyRef<'_, Self> {
        slf.hires_foreground(color);
        slf
    }

    /// Push a high-resolution background color onto this style builder.
    #[pyo3(name = "hires_background")]
    pub fn py_hires_background(slf: PyRef<'_, Self>, color: Color) -> PyRef<'_, Self> {
        slf.hires_background(color);
        slf
    }

    /// Finish building and return a new style.
    ///
    /// This method creates a new style with this builder's style tokens. To
    /// avoid unnecessary allocations, it moves the list of tokens into the
    /// returned struct while leaving an empty vector behind.
    pub fn go(&self) -> Style {
        Style {
            tokens: Arc::new(self.tokens.take()),
        }
    }
}

impl StyleBuilder {
    /// Retrieve a format to modify.
    ///
    /// If the last style token is a format, this method removes the token from
    /// this builder and returns the unwrapped format. Otherwise, it creates a
    /// new format. In either case, the expectation is that the caller updates
    /// the format and adds it onto this style builder.
    fn latest_format(&self, tokens: &mut Vec<StyleToken>) -> Format {
        if let Some(StyleToken::Format(format)) = tokens.last() {
            let format = *format; // Copy format to appease borrow checker
            tokens.pop();
            format
        } else {
            Format::new()
        }
    }

    /// Push a style reset onto this builder.
    pub fn reset(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        tokens.push(StyleToken::Reset());
        self
    }

    /// Add bold formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style collection.
    pub fn bold(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let format = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(format.bold()));
        self
    }

    /// Add thin formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style builder.
    pub fn thin(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let formatting = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(formatting.thin()));
        self
    }

    /// Add italic formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style builder.
    pub fn italic(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let formatting = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(formatting.italic()));
        self
    }

    /// Add underlined formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style builder.
    pub fn underlined(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let formatting = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(formatting.underlined()));
        self
    }

    /// Add blinking formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style builder.
    pub fn blinking(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let formatting = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(formatting.blinking()));
        self
    }

    /// Add reversed formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style builder.
    pub fn reversed(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let formatting = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(formatting.reversed()));
        self
    }

    /// Add hidden formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style builder.
    pub fn hidden(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let formatting = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(formatting.hidden()));
        self
    }

    /// Add stricken formatting to this style builder.
    ///
    /// If the latest style is formatting, this method modifies the latest
    /// style. Otherwise, it pushes new formatting onto this style builder.
    pub fn stricken(&self) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        let formatting = self.latest_format(&mut tokens);
        tokens.push(StyleToken::Format(formatting.stricken()));
        self
    }

    /// Push a foreground color onto this style builder.
    pub fn foreground(&self, color: impl Into<TerminalColor>) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        tokens.push(StyleToken::Foreground(color.into()));
        self
    }

    /// Push a background color onto this style builder.
    pub fn background(&self, color: impl Into<TerminalColor>) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        tokens.push(StyleToken::Background(color.into()));
        self
    }

    /// Push a high-resolution foreground color onto this style builder.
    pub fn hires_foreground(&self, color: Color) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        tokens.push(StyleToken::HiResForeground(color));
        self
    }

    /// Push a high-resolution background color oto this style builder.
    pub fn hires_background(&self, color: Color) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        tokens.push(StyleToken::HiResBackground(color));
        self
    }

    /// Finish building and return a new style.
    ///
    /// This method creates a new style with this builder's style tokens. To
    /// avoid unnecessary allocations, it moves the list of tokens into the
    /// returned struct while leaving an empty vector behind.
    #[cfg(not(feature = "pyffi"))]
    pub fn go(&self) -> Style {
        Style {
            tokens: Arc::new(self.tokens.take()),
        }
    }
}

// -------------------------------------------------------------------------------------

/// A combination of styles.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    tokens: Arc<Vec<StyleToken>>,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Style {
    /// Get an iterator over the style's tokens.
    pub fn tokens(&self) -> TokenIterator {
        TokenIterator {
            tokens: self.tokens.clone(),
            index: 0,
        }
    }

    /// Determine the style collection's fidelity.
    ///
    /// The fidelity of this style collection is the maximum fidelity of the
    /// constituent styles.
    pub fn fidelity(&self) -> Fidelity {
        self.tokens
            .iter()
            .map(StyleToken::fidelity)
            .max()
            .unwrap_or(Fidelity::Plain)
    }
}

// -------------------------------------------------------------------------------------

/// An iterator over a style's tokens.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.style"))]
#[derive(Debug)]
pub struct TokenIterator {
    tokens: Arc<Vec<StyleToken>>,
    index: usize,
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl TokenIterator {
    /// Access this iterator. <i class=python-only>Python only!</i>
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next style token. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<StyleToken> {
        slf.next()
    }
}

impl Iterator for TokenIterator {
    type Item = StyleToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.tokens.len() <= self.index {
            None
        } else {
            self.index += 1;
            Some(self.tokens[self.index - 1].clone())
        }
    }
}

// -------------------------------------------------------------------------------------

/// The definition of rich text.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RichText {
    style: Style,
    text: String,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl RichText {
    pub fn fidelity(&self) -> Fidelity {
        self.style.fidelity()
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod test {
    use super::{stylist, StyleToken};
    use crate::style::{format::Format, AnsiColor, TerminalColor};

    #[test]
    fn test_stylist() {
        let bold_red = stylist()
            .bold()
            .underlined()
            .foreground(AnsiColor::Red)
            .go();
        for (index, token) in bold_red.tokens().enumerate() {
            match token {
                StyleToken::Format(format) => {
                    assert_eq!(index, 0);
                    assert_eq!(format, Format::new().bold().underlined());
                }
                StyleToken::Foreground(color) => {
                    assert_eq!(index, 1);
                    assert_eq!(
                        color,
                        TerminalColor::Ansi {
                            color: AnsiColor::Red
                        }
                    );
                }
                _ => panic!("unexpected style token {:?}", token),
            }
        }
    }
}
