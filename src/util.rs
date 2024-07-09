/// A trait to abstract over environment variable access.
///
/// The standard library is a bit spartan when it comes to environment variable
/// access. So this trait makes up for it yet still keeps things simple by only
/// requiring the implementation of one method.
pub(crate) trait Environment {
    fn get_os(&self, key: &str) -> Option<std::ffi::OsString>;

    #[inline]
    fn get(&self, key: &str) -> Result<String, std::env::VarError> {
        match self.get_os(key) {
            Some(s) => s.into_string().map_err(std::env::VarError::NotUnicode),
            None => Err(std::env::VarError::NotPresent),
        }
    }

    #[inline]
    fn is_defined(&self, key: &str) -> bool {
        self.get_os(key).is_some()
    }

    #[inline]
    fn is_non_empty(&self, key: &str) -> bool {
        let value = self.get_os(key);
        value.is_some() && value.unwrap().len() > 0
    }

    #[inline]
    fn has_value(&self, key: &str, expected_value: &str) -> bool {
        let actual = self.get_os(key);
        actual.is_some() && actual.unwrap() == expected_value
    }
}

pub(crate) struct Env();

impl Environment for Env {
    #[inline]
    fn get_os(&self, key: &str) -> Option<std::ffi::OsString> {
        std::env::var_os(key)
    }
}

impl Default for Env {
    fn default() -> Self {
        Env()
    }
}
