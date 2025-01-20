use std::io::Result;

/// Trait to determine whether a status code is an error.
pub(crate) trait IsError {
    /// Determine if this value is an error.
    fn is_error(&self) -> bool;
}

#[cfg(target_family = "unix")]
macro_rules! is_error {
    ($source:ty) => {
        impl IsError for $source {
            #[inline]
            fn is_error(&self) -> bool {
                *self == -1
            }
        }
    };
}

#[cfg(target_family = "windows")]
macro_rules! is_error {
    ($source:ty) => {
        impl IsError for $source {
            #[inline]
            fn is_error(&self) -> bool {
                *self == 0
            }
        }
    };
}

is_error!(i32);
is_error!(isize);
#[cfg(target_family = "windows")]
is_error!(u32);

/// Trait to convert a status code into a Rust result.
pub(crate) trait IntoResult {
    /// The target type.
    type Target;

    /// Convert this status code into a Rust result.
    fn into_result(self) -> Result<Self::Target>;
}

macro_rules! into_result {
    ($source:ty, $target:ty) => {
        impl IntoResult for $source {
            type Target = $target;

            fn into_result(self) -> Result<Self::Target> {
                if self.is_error() {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(self as Self::Target)
                }
            }
        }
    };
}

into_result!(i32, u32);
into_result!(isize, usize);
#[cfg(target_family = "windows")]
into_result!(u32, u32);

// -------------------------------------------------------------------------------------

/// A newtype to display a string as an identifier.
struct Ident<'a>(&'a str);

impl<'a> Ident<'a> {
    pub fn new(name: &&'a str) -> Self {
        Self(name)
    }
}

impl std::fmt::Debug for Ident<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// A newtype to display a vector of strings as a list of identifiers.
pub(crate) struct IdentList<'a>(Vec<Ident<'a>>);

impl<'a> IdentList<'a> {
    pub fn new(names: Vec<&'a str>) -> Self {
        Self(names.iter().map(Ident::new).collect())
    }
}

impl std::fmt::Debug for IdentList<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.iter()).finish()
    }
}
