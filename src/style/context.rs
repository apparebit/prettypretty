#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::TerminalColor;
use crate::util::{Env, Environment};

// ====================================================================================================================
// Layer and Fidelity

/// The targeted display layer: Foreground or background.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The foreground or text layer.
    Foreground,
    /// The background layer.
    Background,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Layer {
    /// Determine the offset for this layer.
    ///
    /// The offset is added to CSI parameter values for foreground colors.
    #[inline]
    pub fn offset(&self) -> u8 {
        match self {
            Self::Foreground => 0,
            Self::Background => 10,
        }
    }

    /// Return a humane description for this layer. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Foreground => f.write_str("Foreground"),
            Self::Background => f.write_str("Background"),
        }
    }
}

/// The stylistic fidelity of terminal output.
///
/// This enumeration captures levels of stylistic fidelity. It can describe the
/// capabilities of a terminal or runtime environment (such as CI) as well as
/// the preferences of a user (notably, `NoColor`).
///
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, ord, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fidelity {
    /// Plain text, no ANSI escape codes
    Plain,
    /// ANSI escape codes but no colors
    NoColor,
    /// ANSI and default colors only
    Ansi,
    /// 8-bit indexed colors including ANSI and default colors
    EightBit,
    /// Full fidelity including 24-bit RGB color.
    Full,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Fidelity {
    /// Determine the fidelity required for rendering the given terminal color.
    /// <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_color(color: TerminalColor) -> Self {
        color.into()
    }

    /// Determine the fidelity level for terminal output based on environment
    /// variables.
    ///
    /// This method determines fidelity based on heuristics about environment
    /// variables. Its primary sources are [NO_COLOR](https://no-color.org) and
    /// [FORCE_COLOR](https://force-color.org). Its secondary source is Chalk's
    /// [supports-color](https://github.com/chalk/supports-color/blob/main/index.js).
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_environment(has_tty: bool) -> Self {
        fidelity_from_environment(&Env::default(), has_tty)
    }

    /// Determine the fidelity level for terminal output based on environment
    /// variables.
    ///
    /// This method determines fidelity based on heuristics about environment
    /// variables. Its primary sources are [NO_COLOR](https://no-color.org) and
    /// [FORCE_COLOR](https://force-color.org). Its secondary source is Chalk's
    /// [supports-color](https://github.com/chalk/supports-color/blob/main/index.js).
    #[cfg(not(feature = "pyffi"))]
    pub fn from_environment(has_tty: bool) -> Self {
        fidelity_from_environment(&Env::default(), has_tty)
    }

    /// Determine whether this fidelity level suffices for rendering the
    /// terminal color.
    pub fn covers(&self, color: TerminalColor) -> bool {
        Fidelity::from(color) <= *self
    }

    /// Return a humane description for this fidelity. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

// While implementing this function, I was also writing helper functions to
// simplify environment access. So, when it came to testing this function, an
// answer offered itself: Mock the environment! Well, not really: I simply
// abstracted environment access behind a trait and use a different
// implementation for testing. That way, I continue to adhere to the first law
// of mocking: Mock people, not code! 😈
pub(crate) fn fidelity_from_environment(env: &impl Environment, has_tty: bool) -> Fidelity {
    if env.is_non_empty("NO_COLOR") {
        return Fidelity::NoColor;
    } else if env.is_non_empty("FORCE_COLOR") {
        return Fidelity::Ansi;
    } else if env.is_defined("TF_BUILD") || env.is_defined("AGENT_NAME") {
        // Supports-color states that this test must come before TTY test.
        return Fidelity::Ansi;
    } else if !has_tty {
        return Fidelity::Plain;
    } else if env.has_value("TERM", "dumb") {
        return Fidelity::Plain; // FIXME Check Windows version!
    } else if env.is_defined("CI") {
        if env.is_defined("GITHUB_ACTIONS") || env.is_defined("GITEA_ACTIONS") {
            return Fidelity::Full;
        }

        for ci in [
            "TRAVIS",
            "CIRCLECI",
            "APPVEYOR",
            "GITLAB_CI",
            "BUILDKITE",
            "DRONE",
        ] {
            if env.is_defined(ci) {
                return Fidelity::Ansi;
            }
        }

        if env.has_value("CI_NAME", "codeship") {
            return Fidelity::Ansi;
        }

        return Fidelity::Plain;
    }

    if let Ok(teamcity) = env.read("TEAMCITY_VERSION") {
        // Apparently, Teamcity 9.x and later support ANSI colors.
        let mut charity = teamcity.chars();
        let c1 = charity.next();
        let c2 = charity.next();

        if c1
            .filter(|c| *c == '9')
            .and(c2.filter(|c| *c == '.'))
            .or(c1
                .filter(|c| c.is_ascii_digit() && *c != '0')
                .and(c2.filter(|c| c.is_ascii_digit()))
                .and(charity.next().filter(|c| *c == '.')))
            .is_some()
        {
            return Fidelity::Ansi;
        }

        return Fidelity::Plain;
    } else if env.has_value("COLORTERM", "truecolor") || env.has_value("TERM", "xterm-kitty") {
        return Fidelity::Full;
    } else if env.has_value("TERM_PROGRAM", "Apple_Terminal") {
        return Fidelity::EightBit;
    } else if env.has_value("TERM_PROGRAM", "iTerm.app") {
        if let Ok(version) = env.read("TERM_PROGRAM_VERSION") {
            let mut charity = version.chars();
            if charity
                .next()
                .filter(|c| *c == '3')
                .and(charity.next().filter(|c| *c == '.'))
                .is_some()
            {
                return Fidelity::Full;
            }
        }
        return Fidelity::EightBit;
    }

    if let Ok(mut term) = env.read("TERM") {
        term.make_ascii_lowercase();

        if term.ends_with("-256") || term.ends_with("-256color") {
            return Fidelity::EightBit;
        } else if term.starts_with("screen")
            || term.starts_with("xterm")
            || term.starts_with("vt100")
            || term.starts_with("vt220")
            || term.starts_with("rxvt")
            || term == "color"
            || term == "ansi"
            || term == "cygwin"
            || term == "linux"
        {
            return Fidelity::Ansi;
        }
    } else if env.is_defined("COLORTERM") {
        return Fidelity::Ansi;
    }

    Fidelity::Plain
}

impl From<TerminalColor> for Fidelity {
    fn from(value: TerminalColor) -> Self {
        match value {
            TerminalColor::Default { .. } | TerminalColor::Ansi { .. } => Self::Ansi,
            TerminalColor::Embedded { .. } | TerminalColor::Gray { .. } => Self::EightBit,
            TerminalColor::Full { .. } => Self::Full,
        }
    }
}

impl std::fmt::Display for Fidelity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Plain => "plain text",
            Self::NoColor => "no colors",
            Self::Ansi => "ANSI colors",
            Self::EightBit => "8-bit colors",
            Self::Full => "24-bit colors",
        };

        f.write_str(s)
    }
}

#[cfg(test)]
mod test {
    use super::{fidelity_from_environment, Fidelity};
    use crate::util::FakeEnv;

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
