#![cfg_attr(not(feature = "std"), no_std)]

mod abiencode {
    mod error;
    mod hashing;
    mod ser;

    pub mod as_bytes;
    pub mod as_dyn_array;
    pub mod types;

    pub use error::{Error, Result};
    pub use hashing::to_hash;
    pub use ser::{to_writer, Serializer, Writer};

    #[cfg(test)]
    pub mod tests;
}
pub mod sig;

pub mod channel;
mod client;
pub mod wire;

pub use abiencode::types::Hash;
pub use client::PerunClient;

// TODO: This probably shouldn't be public, but the example currently needs it,
// since the encoding layer doesn't do decoding, yet.
pub mod perunwire {
    include!(concat!(env!("OUT_DIR"), "/perunwire.rs"));
}
