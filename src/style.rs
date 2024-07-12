#![allow(dead_code)]

use crate::{Layer, TerminalColor};

pub trait TextAttribute: Copy + Default + PartialEq {
    /// Determine whether the variant is the default.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::style::*;
    /// assert!(Reverse::NotReversed.is_default());
    /// assert!(Reverse::default().is_default());
    /// assert!(!Reverse::Reversed.is_default());
    /// ```
    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Negate this text attribute.
    ///
    /// This method determines the update necessary for restoring the text
    /// attribute to its default state. Of course, the only attribute value that
    /// can restore the default state is the default value. Hence, if this text
    /// attribute is *not* the default, this method returns the default wrapped
    /// in `Some`. Otherwise, it returns `None`.
    ///
    /// # Examples
    ///
    /// Negating some `attribute` is the same as subtracting the attribute from
    /// the default value, i.e., `T::default().subtract(attribute)`.
    ///
    /// ```
    /// # use prettypretty::style::*;
    /// assert_eq!(Blink::Blinking.negate(), Some(Blink::NotBlinking));
    /// assert_eq!(Blink::default().subtract(Some(Blink::Blinking)), Some(Blink::NotBlinking));
    /// assert_eq!(Blink::NotBlinking.negate(), None);
    /// assert_eq!(Blink::default().subtract(Some(Blink::NotBlinking)), None);
    /// ```
    fn negate(&self) -> Option<Self> {
        if !self.is_default() {
            Some(Self::default())
        } else {
            None
        }
    }

    /// Subtract another text attribute from this one.
    ///
    /// This method determines the update necessary for setting the text
    /// attribute to this value if it last was set to the other value. Of
    /// course, the only attribute value that can do so is this text attribute
    /// value. Hence, if this text attribute  That leads to the following four cases:Hence, if the previous attribute is `None`the this method returns this text attribute value wrapped in
    /// `Some` unless the two attribute values are the same, in which case it
    /// returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::style::*;
    /// assert_eq!(Weight::Bold.subtract(Some(Weight::Thin)), Some(Weight::Bold));
    /// assert_eq!(Weight::Regular.subtract(Some(Weight::Regular)), None);
    /// ```
    fn subtract(&self, other: Option<Self>) -> Option<Self> {
        match other {
            Some(other) => {
                if *self != other {
                    Some(*self)
                } else {
                    None
                }
            }
            None => {
                if !self.is_default() {
                    Some(*self)
                } else {
                    None
                }
            }
        }
    }
}

macro_rules! text_attributes {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident {
                $default_variant:ident = $default_value:expr ,
                $( $variant:ident = $value:expr ),+
                $(,)?
            }
        )*
    ) => {
        $(
            $( #[$attr] )*
            #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
            pub enum $name {
                #[default]
                $default_variant = $default_value,
                $( $variant = $value ),+
            }

            impl TextAttribute for $name {}
        )*
    }
}

text_attributes! {
    /// The font weight: Regular, bold, or thin.
    Weight {
        Regular = 22,
        Bold = 1,
        Thin = 2,
    }

    /// The font style: upright or italic.
    ///
    /// This text attribute effectively is the binary attribute for italic text.
    Slant {
        Upright = 23,
        Italic = 3,
    }

    /// The binary attribute for underlined text.
    Underline {
        NotUnderlined = 24,
        Underlined = 4,
    }

    /// The binary attribute for blinking text.
    Blink {
        NotBlinking = 25,
        Blinking = 5,
    }

    /// The binary attribute for reversed text.
    Reverse {
        NotReversed = 27,
        Reversed = 7,
    }

    /// The binary attribute for stricken text.
    Strike {
        NotStricken = 29,
        Stricken = 9,
    }
}

/// A terminal style.
///
/// A terminal style captures the visual appearance of terminal output,
/// including text attributes as well as foreground and background colors. There
/// are two ways of modelling terminal styles:
///
///  1. Effective styles: Each style instance captures *all attributes* of a
///     cell in the fixed-width grid being displayed on screen. Since this model
///     only recognizes complete descriptions, representing changes may require
///     two style instances, one for attributes to clear and one for attributes
///     to set.
///  2. Style changes: Each style instance captures only *changing attributes*.
///     This representation is far more inline with ANSI escape sequences, which
///     incrementally update terminal styles. However, computing the effective
///     style may require an arbitrary history of style changes.
///
/// This struct uses the second approach but also provides methods to
/// automatically determine styles that undo the previous style, incrementally
/// modify a style, or combine several other styles.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    weight: Option<Weight>,
    slant: Option<Slant>,
    underline: Option<Underline>,
    blink: Option<Blink>,
    reverse: Option<Reverse>,
    strike: Option<Strike>,
    foreground: Option<TerminalColor>,
    background: Option<TerminalColor>,
}

impl Style {
    /// Determine whether this style is empty.
    ///
    /// A style is empty if it has no text attributes or colors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::style::*;
    /// assert!(Style::default().is_empty());
    /// assert!(!Style::default().bold().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.weight.is_none()
            && self.slant.is_none()
            && self.underline.is_none()
            && self.blink.is_none()
            && self.reverse.is_none()
            && self.strike.is_none()
            && self.foreground.is_none()
            && self.background.is_none()
    }

    /// Determine the style for restoring default appearance.
    ///
    /// This method computes the style that takes a terminal with this style
    /// back to its default appearance.
    pub fn negate(&self) -> Self {
        fn negate(color: TerminalColor, default: TerminalColor) -> Option<TerminalColor> {
            if !color.is_default() {
                Some(default)
            } else {
                None
            }
        }

        Self {
            weight: self.weight.and_then(|a| a.negate()),
            slant: self.slant.and_then(|a| a.negate()),
            underline: self.underline.and_then(|a| a.negate()),
            blink: self.blink.and_then(|a| a.negate()),
            reverse: self.reverse.and_then(|a| a.negate()),
            strike: self.strike.and_then(|a| a.negate()),
            foreground: self
                .foreground
                .and_then(|c| negate(c, TerminalColor::FOREGROUND)),
            background: self
                .background
                .and_then(|c| negate(c, TerminalColor::BACKGROUND)),
        }
    }

    /// Determine the style change from the other to this style.
    ///
    /// This method returns the incremental style change that updates a terminal
    /// with the other style to this style.
    pub fn subtract(&self, other: &Self) -> Self {
        fn subtract(color: TerminalColor, other: Option<TerminalColor>) -> Option<TerminalColor> {
            match other {
                Some(other) => {
                    if color != other {
                        Some(color)
                    } else {
                        None
                    }
                }
                None => {
                    if !color.is_default() {
                        Some(color)
                    } else {
                        None
                    }
                }
            }
        }

        Self {
            weight: self.weight.unwrap_or_default().subtract(other.weight),
            slant: self.slant.unwrap_or_default().subtract(other.slant),
            underline: self.underline.unwrap_or_default().subtract(other.underline),
            blink: self.blink.unwrap_or_default().subtract(other.blink),
            reverse: self.reverse.unwrap_or_default().subtract(other.reverse),
            strike: self.strike.unwrap_or_default().subtract(other.strike),
            foreground: subtract(
                self.foreground.unwrap_or(TerminalColor::FOREGROUND),
                other.foreground,
            ),
            background: subtract(
                self.background.unwrap_or(TerminalColor::BACKGROUND),
                other.background,
            ),
        }
    }

    /// Determine the combined style.
    ///
    /// This method returns the style resulting from applying first the other
    /// and then this style to a terminal. Just as for style subtraction, the
    /// order of styles matters for style addition. In other words, style
    /// addition is not commutative.
    pub fn add(&self, other: &Self) -> Self {
        Self {
            weight: self.weight.or(other.weight),
            slant: self.slant.or(other.slant),
            underline: self.underline.or(other.underline),
            blink: self.blink.or(other.blink),
            reverse: self.reverse.or(other.reverse),
            strike: self.strike.or(other.strike),
            foreground: self.foreground.or(other.foreground),
            background: self.background.or(other.background),
        }
    }
    pub fn bold(&mut self) -> &mut Self {
        self.weight = Some(Weight::Bold);
        self
    }
    pub fn thin(&mut self) -> &mut Self {
        self.weight = Some(Weight::Thin);
        self
    }
    pub fn italic(&mut self) -> &mut Self {
        self.slant = Some(Slant::Italic);
        self
    }
    pub fn underlined(&mut self) -> &mut Self {
        self.underline = Some(Underline::Underlined);
        self
    }
    pub fn blink(&mut self) -> &mut Self {
        self.blink = Some(Blink::Blinking);
        self
    }
    pub fn reverse(&mut self) -> &mut Self {
        self.reverse = Some(Reverse::Reversed);
        self
    }
    pub fn strike(&mut self) -> &mut Self {
        self.strike = Some(Strike::Stricken);
        self
    }

    /// Get the SGR parameters corresponding to this style.
    pub fn sgr_parameters(&self) -> Vec<u8> {
        let mut parameters = Vec::new();

        if let Some(weight) = self.weight {
            parameters.push(weight as u8);
        }
        if let Some(slant) = self.slant {
            parameters.push(slant as u8);
        }
        if let Some(underline) = self.underline {
            parameters.push(underline as u8);
        }
        if let Some(blink) = self.blink {
            parameters.push(blink as u8);
        }
        if let Some(reverse) = self.reverse {
            parameters.push(reverse as u8);
        }
        if let Some(strike) = self.strike {
            parameters.push(strike as u8);
        }
        if let Some(foreground) = self.foreground {
            parameters.append(&mut foreground.sgr_parameters(Layer::Foreground));
        }
        if let Some(background) = self.background {
            parameters.append(&mut background.sgr_parameters(Layer::Background));
        }

        parameters
    }

    /// Get the SGR ANSI escape sequence corresponding to this style.
    pub fn sgr(&self) -> String {
        use std::fmt::Write;

        let mut sgr = String::new();

        let _ = write!(&mut sgr, "\x1b[");
        for (index, param) in self.sgr_parameters().into_iter().enumerate() {
            if index > 0 {
                let _ = write!(&mut sgr, ";{}", param);
            } else {
                let _ = write!(&mut sgr, "{}", param);
            }
        }
        let _ = write!(&mut sgr, "m");

        sgr
    }
}
