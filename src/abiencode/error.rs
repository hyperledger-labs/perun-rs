//! Error type and Return values used by the Serialization.

use core::fmt::Display;

use serde::ser;

/// Represents all possible errors that can happen during Serialization.
///
/// Note that custom errors using [ser::Error::custom()] are not yet supported.
#[derive(Debug)]
pub enum Error {
    /// The struct contains a type that is not directly representable in
    /// Solidity types.
    ///
    /// For example floating point numbers, enums and maps. While we could
    /// default to some enum representation or automatically convert floats to
    /// `fixedNxM` we don't do this, as it could lead to loss of accuracy or
    /// force a specific representation on the Solidity side. Instead use the
    /// [serde_repr](https://github.com/dtolnay/serde-repr) crate as shown in
    /// the [Serde Overview](https://serde.rs/enum-number.html) for enums or
    /// implement a custom serialize method.
    TypeNotRepresentable(&'static str),
    /// Although the type is representable in Solidity (currently only used for
    /// `char`), the Serializer currently does not implement this functionality.
    TypeNotYetSupported(&'static str),
}

impl ser::Error for Error {
    fn custom<T>(_: T) -> Self
    where
        T: core::fmt::Display,
    {
        unimplemented!()
    }
}
#[cfg(feature = "std")]
impl ser::StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::TypeNotRepresentable(type_name) => {
                f.write_str("type is not representable in abi encoding: ")?;
                f.write_str(type_name)
            }
            Error::TypeNotYetSupported(type_name) => {
                f.write_str("type is not yet implemented: ")?;
                f.write_str(type_name)
            }
        }
    }
}

/// Alias for `Result` using the [Error] returned by the Serializer.
pub type Result<T> = core::result::Result<T, Error>;
