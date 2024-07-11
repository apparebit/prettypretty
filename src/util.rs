/// A trait to abstract over environment variable access.
///
/// The standard library is a bit spartan when it comes to environment variable
/// access. So this trait makes up for it yet still keeps things simple by only
/// requiring the implementation of one method.
pub(crate) trait Environment {
    /// Try reading the environment variable as an OS string.
    fn read_os(&self, key: &str) -> Option<std::ffi::OsString>;

    /// Try reading the environment variable as a string.
    #[inline]
    fn read(&self, key: &str) -> Result<String, std::env::VarError> {
        match self.read_os(key) {
            Some(s) => s.into_string().map_err(std::env::VarError::NotUnicode),
            None => Err(std::env::VarError::NotPresent),
        }
    }

    /// Determine whether the environment variable is defined.
    #[inline]
    fn is_defined(&self, key: &str) -> bool {
        self.read_os(key).is_some()
    }

    /// Determine whether the environment variable is defined with a non-empty value.
    #[inline]
    fn is_non_empty(&self, key: &str) -> bool {
        let value = self.read_os(key);
        value.is_some() && value.unwrap().len() > 0
    }

    /// Determine whether the environment variable has the given value.
    #[inline]
    fn has_value(&self, key: &str, expected_value: &str) -> bool {
        let actual = self.read_os(key);
        actual.is_some() && actual.unwrap() == expected_value
    }
}

#[derive(Debug, Default)]
pub(crate) struct Env();

impl Environment for Env {
    #[inline]
    fn read_os(&self, key: &str) -> Option<std::ffi::OsString> {
        std::env::var_os(key)
    }
}

#[cfg(test)]
mod test {
    use super::Environment;
    use std::collections::HashMap;

    pub(crate) struct FakeEnv {
        bindings: HashMap<String, String>,
    }

    impl FakeEnv {
        /// Create a new fake environment.
        pub(crate) fn new() -> FakeEnv {
            FakeEnv {
                bindings: HashMap::new(),
            }
        }

        /// Set the fake environment variable.
        pub(crate) fn set(&mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> &mut Self {
            self.bindings
                .insert(key.as_ref().to_string(), value.as_ref().to_string());
            self
        }
    }

    impl Environment for FakeEnv {
        fn read_os(&self, key: &str) -> Option<std::ffi::OsString> {
            self.bindings.get(key).map(|v| v.into())
        }
    }
}

#[cfg(test)]
pub(crate) use test::FakeEnv;
