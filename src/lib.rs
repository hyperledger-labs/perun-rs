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

pub use abiencode::types::{Address, Hash};
pub use client::PerunClient;

// TODO: This probably shouldn't be public, but the example currently needs it,
// since the encoding layer doesn't do decoding, yet.
pub mod perunwire {
    // The message types are currently defined in two separate .proto files with
    // different package names. This makes sense (as of now), since they are
    // defined in different repositories. Using the same package names in the
    // protobuf files would probably work, but does not feel right, hence
    // different package names, one for each protobuf file.
    //
    // In Rust (and Go, too), this distinction doesn't make much sense unless
    // there is a name conflict. Instead of having one module per protobuf
    // package we're importing both into this package, similar to how you'd
    // re-export all types in a subpackage.

    include!(concat!(env!("OUT_DIR"), "/perunwire.rs"));
    include!(concat!(env!("OUT_DIR"), "/perunremote.rs"));
}
