pub(crate) struct Env {}

impl Env {
    #[inline]
    pub(crate) fn is_defined(name: &str) -> bool {
        std::env::var_os(name).is_some()
    }

    #[inline]
    pub(crate) fn is_non_empty(name: &str) -> bool {
        let value = std::env::var_os(name);
        value.is_some() && value.unwrap().len() > 0
    }

    #[inline]
    pub(crate) fn has_value(name: &str, value: &str) -> bool {
        let actual = std::env::var_os(name);
        actual.is_some() && actual.unwrap() == value
    }
}
