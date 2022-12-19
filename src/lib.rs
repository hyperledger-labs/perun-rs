#![cfg_attr(not(feature = "std"), no_std)]

mod abiencode {
    mod error;
    mod hashing;
    mod ser;

    pub mod as_array;
    pub mod as_bytes;
    pub mod as_dyn_array;
    pub mod types;

    pub use error::{Error, Result};
    pub use hashing::to_hash;
    pub use ser::{to_writer, Serializer, Writer};

    #[cfg(test)]
    pub mod tests;
}
