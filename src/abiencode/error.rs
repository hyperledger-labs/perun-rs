use core::fmt::Display;

use serde::ser;

#[derive(Debug)]
pub enum Error {
    TypeNotRepresentable(&'static str),
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

pub type Result<T> = core::result::Result<T, Error>;
