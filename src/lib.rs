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

/// Handles the creation and verification of (Ethereum) Signatures.
///
/// Layout and Content of this module will most likely change in the near future
/// when adding support for the `k256` library, which has support for no_std.
mod sig {
    #[cfg(feature = "secp256k1")]
    mod secp256k1;
    #[cfg(feature = "secp256k1")]
    pub use self::secp256k1::{eth_sign, recover_signer};
}

pub mod channel;
mod client;
pub mod wire;

pub use abiencode::types::Address;
pub use client::PerunClient;
