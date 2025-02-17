/// A trait to abstract over environment variable access.
///
/// The standard library is a bit spartan when it comes to environment variable
/// access. So this trait makes up for it yet still keeps things simple by only
/// requiring the implementation of one method.
pub(crate) trait Environment {
    /// Try reading the environment variable as an OS string.
    fn read_os(&self, key: &str) -> Option<std::ffi::OsString>;

    /// Try reading the environment variable as a string.
    fn read(&self, key: &str) -> Result<String, std::env::VarError> {
        self.read_os(key).map_or_else(
            || Err(std::env::VarError::NotPresent),
            |s| s.into_string().map_err(std::env::VarError::NotUnicode),
        )
    }

    /// Determine whether the environment variable is defined.
    fn is_defined(&self, key: &str) -> bool {
        self.read_os(key).is_some()
    }

    /// Determine whether the environment variable is defined with a non-empty value.
    fn is_non_empty(&self, key: &str) -> bool {
        self.read_os(key).is_some_and(|v| !v.is_empty())
    }

    /// Determine whether the environment variable has the given value.
    fn has_value(&self, key: &str, expected_value: &str) -> bool {
        self.read_os(key).is_some_and(|v| v == expected_value)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Env();

impl Environment for Env {
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
