#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use std::cell::RefCell;
use std::sync::Arc;

use super::{format::Format, DefaultColor, Fidelity, Layer, TerminalColor};
use crate::{trans::Translator, Color};

/// A style token represents the atomic units of styles, including colors and
/// other text formats.
///
/// Like design tokens contribute to a larger design system, [`StyleToken`]s
/// contribute to [`Style`]s. Following the builder pattern, a [`Stylist`]
/// fluently assembles styles.
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
    // ...
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl StyleToken {
    /// Determine whether the style token represents a color.
    pub fn is_color(&self) -> bool {
        matches!(
            self,
            Self::Foreground(_)
                | Self::Background(_)
                | Self::HiResForeground(_)
                | Self::HiResBackground(_)
        )
    }

    /// Determine the terminal fidelity necessary for rendering this style token
    /// as is.
    ///
    /// This method necessarily returns a fidelity higher than
    /// [`Fidelity::Plain`].
    pub fn fidelity(&self) -> Fidelity {
        match self {
            Self::Reset() => Fidelity::NoColor,
            Self::Format(_) => Fidelity::NoColor,
            Self::Foreground(color) => (*color).into(),
            Self::Background(color) => (*color).into(),
            Self::HiResForeground(_) => Fidelity::Full,
            Self::HiResBackground(_) => Fidelity::Full,
        }
    }

    /// Cap this style token at the given fidelity.
    ///
    /// If the fidelity is [`Fidelity::Plain`], this method returns `None`. If
    /// the fidelity is higher, this method adjusts colors using the given
    /// translator.
    pub fn cap(&self, fidelity: Fidelity, translator: &Translator) -> Option<Self> {
        match self {
            StyleToken::Reset() => {
                if matches!(fidelity, Fidelity::Plain) {
                    None
                } else {
                    Some(StyleToken::Reset())
                }
            }
            StyleToken::Format(format) => format.cap(fidelity).map(StyleToken::Format),
            StyleToken::Foreground(color) => {
                translator.cap(*color, fidelity).map(StyleToken::Foreground)
            }
            StyleToken::Background(color) => {
                translator.cap(*color, fidelity).map(StyleToken::Background)
            }
            StyleToken::HiResForeground(color) => translator
                .cap(TerminalColor::from(color), fidelity)
                .map(StyleToken::Foreground),
            StyleToken::HiResBackground(color) => translator
                .cap(TerminalColor::from(color), fidelity)
                .map(StyleToken::Background),
        }
    }

    /// Get the SGR parameters for this style token.
    pub fn sgr_parameters(&self) -> Vec<u8> {
        match self {
            StyleToken::Reset() => vec![0],
            StyleToken::Format(format) => format.sgr_parameters(),
            StyleToken::Foreground(color) => color.sgr_parameters(Layer::Foreground),
            StyleToken::Background(color) => color.sgr_parameters(Layer::Background),
            StyleToken::HiResForeground(color) => {
                TerminalColor::from(color).sgr_parameters(Layer::Foreground)
            }
            StyleToken::HiResBackground(color) => {
                TerminalColor::from(color).sgr_parameters(Layer::Background)
            }
        }
    }

    #[cfg(feature = "pyffi")]
    pub fn __invert__(&self) -> Option<Self> {
        !*self
    }
}

impl std::ops::Not for StyleToken {
    type Output = Option<Self>;

    /// Negate this style token.
    ///
    /// This method returns a style token to restore the terminal's default
    /// appearance from this style token or `None` if no updates are required.
    fn not(self) -> Self::Output {
        match self {
            Self::Reset() => None,
            Self::Format(format) => Some(Self::Format(!format)),
            Self::Foreground(TerminalColor::Default { .. }) => None,
            Self::Foreground(_) | Self::HiResForeground(_) => {
                Some(Self::Foreground(TerminalColor::Default {
                    color: DefaultColor::Foreground,
                }))
            }
            Self::Background(TerminalColor::Default { .. }) => None,
            Self::Background(_) | Self::HiResBackground(_) => {
                Some(Self::Background(TerminalColor::Default {
                    color: DefaultColor::Background,
                }))
            }
        }
    }
}

// -------------------------------------------------------------------------------------

/// Create a new, empty [`Stylist`].
///
/// This convenience function is equivalent to `Stylist::new()`.
#[cfg_attr(feature = "pyffi", pyfunction)]
pub fn stylist() -> Stylist {
    Stylist::new()
}

/// A stylist is a builder for fluently assembling styles from style tokens.
///
///
/// # Uniqueness and Order of Style Tokens
///
/// This struct makes *no* guarantees about the order of style tokens in the
/// built style, except that, if the style includes a reset token, that reset
/// token is the first one. Otherwise, the implementation of this struct is free
/// to reorder tokens as it sees fit. It does, however, guarantee that the
/// resulting style contains at most one reset token, format token, foreground
/// color, and background color each.
#[cfg_attr(feature = "pyffi", pyclass(eq, module = "prettypretty.color.style"))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stylist {
    tokens: RefCell<Vec<StyleToken>>,
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

impl Stylist {
    /// Create a new style builder.
    pub const fn new() -> Self {
        Self {
            tokens: RefCell::new(Vec::new()),
        }
    }

    /// Create a new style builder with a reset token as its first token.
    pub fn with_reset() -> Self {
        Self {
            tokens: RefCell::new(vec![StyleToken::Reset()]),
        }
    }

    /// Retrieve or create a format to modify.
    ///
    /// This method is intentionally private!
    ///
    /// If any style token is a format, this method removes the format from this
    /// style builder and returns it. Otherwise, it creates a new, empty format.
    /// In either case, the expectation is that the caller updates the format
    /// and adds it back onto this style builder. This method may reorder this
    /// style builder's tokens.
    fn latest_format(&self, tokens: &mut Vec<StyleToken>) -> Format {
        for index in 0..tokens.len() {
            if let StyleToken::Format(format) = tokens[index] {
                tokens.swap_remove(index);
                return format;
            }
        }

        Format::new()
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

    /// Remove an existing foreground style token.
    ///
    /// This method is intentionally private!
    ///
    /// This method removes an existing foreground style token, whether
    /// high-resolution or otherwise, from this style builder. The expectation
    /// is that the caller adds a different foreground style token onto this
    /// style builder. This method may reorder this style builder's tokens.
    fn remove_foreground(&self, tokens: &mut Vec<StyleToken>) {
        for index in 0..tokens.len() {
            match tokens[index] {
                StyleToken::Foreground(_) | StyleToken::HiResForeground(_) => {
                    tokens.swap_remove(index);
                    return;
                }
                _ => (),
            }
        }
    }

    /// Remove an existing background style token.
    ///
    /// This method is intentionally private!
    ///
    /// This method removes an existing background style token, whether
    /// high-resolution or otherwise, from this style builder. The expectation
    /// is that the caller adds a different background style token onto this
    /// style builder. This method may reorder this style builder's tokens.
    fn remove_background(&self, tokens: &mut Vec<StyleToken>) {
        for index in 0..tokens.len() {
            match tokens[index] {
                StyleToken::Foreground(_) | StyleToken::HiResForeground(_) => {
                    tokens.swap_remove(index);
                    return;
                }
                _ => (),
            }
        }
    }

    /// Push a foreground color onto this style builder.
    pub fn foreground(&self, color: impl Into<TerminalColor>) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        self.remove_foreground(&mut tokens);
        tokens.push(StyleToken::Foreground(color.into()));
        self
    }

    /// Push a background color onto this style builder.
    pub fn background(&self, color: impl Into<TerminalColor>) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        self.remove_background(&mut tokens);
        tokens.push(StyleToken::Background(color.into()));
        self
    }

    /// Push a high-resolution foreground color onto this style builder.
    pub fn hires_foreground(&self, color: Color) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        self.remove_foreground(&mut tokens);
        tokens.push(StyleToken::HiResForeground(color));
        self
    }

    /// Push a high-resolution background color oto this style builder.
    pub fn hires_background(&self, color: Color) -> &Self {
        let mut tokens = self.tokens.borrow_mut();
        self.remove_background(&mut tokens);
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Style {
    tokens: Arc<Vec<StyleToken>>,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Style {
    /// Create a new style builder.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn builder() -> Stylist {
        Stylist::new()
    }

    /// Determine whether this style is empty.
    pub fn is_empty(&self) -> bool {
        self.tokens.len() == 0
    }

    /// Get an iterator over the style's tokens.
    pub fn tokens(&self) -> TokenIterator {
        TokenIterator {
            tokens: self.tokens.clone(),
            index: 0,
        }
    }

    /// Determine whether this style includes color.
    pub fn has_color(&self) -> bool {
        for token in self.tokens() {
            if token.is_color() {
                return true;
            }
        }
        false
    }

    /// Determine this style's fidelity, which is the maximum fidelity of all
    /// style tokens.
    pub fn fidelity(&self) -> Fidelity {
        self.tokens
            .iter()
            .map(StyleToken::fidelity)
            .max()
            .unwrap_or(Fidelity::Plain)
    }

    /// Cap this style to the given fidelity.
    ///
    /// If the fidelity is [`Fidelity::Plain`], this method strips the style of
    /// all tokens that require ANSI escape sequences for implementation (which
    /// currently is all). Otherwise, it adjusts colors with the given
    /// translator.
    pub fn cap(&self, fidelity: Fidelity, translator: &Translator) -> Self {
        let tokens = self
            .tokens()
            .filter_map(|t| t.cap(fidelity, translator))
            .collect();

        Style {
            tokens: Arc::new(tokens),
        }
    }

    /// Get the SGR parameters for this style.
    pub fn sgr_parameters(&self) -> Vec<u8> {
        let mut parameters = Vec::new();
        for token in self.tokens() {
            parameters.append(&mut token.sgr_parameters())
        }

        parameters
    }

    /// Invert this style. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __invert__(&self) -> Self {
        !self
    }
}

#[cfg(not(feature = "pyffi"))]
impl Style {
    /// Create a new style builder.
    pub fn builder() -> Stylist {
        Stylist::new()
    }
}

impl std::ops::Not for Style {
    type Output = Self;

    /// Negate this style.
    ///
    /// This method returns the style to restore the terminal's default
    /// appearance from this style, which may be empty.
    fn not(self) -> Self::Output {
        let tokens = self.tokens().filter_map(|t| !t).collect();

        Style {
            tokens: Arc::new(tokens),
        }
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
