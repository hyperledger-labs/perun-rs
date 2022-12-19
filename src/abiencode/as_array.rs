//! Serialize any `&[T]` as solidity `T[N]` (fixed-size array).
//!
//! Useful to hide a `Vec<T>` as a fixed-size array or when using const
//! generics.
//!
//! The length of the fixed-size array is taken from the length of the slice and
//! may thus be unknown at compile time.
//!
//! # Example usage
//! ```ignore
//! # // We cannot run this test because abiencode is not public.
//! # use serde::Serialize;
//! # use perun::abiencode::as_array;
//!
//! #[derive(Serialize, Debug)]
//! pub struct ConstGeneric<const N: usize> {
//!     #[serde(with = "as_array")]
//!     pub data: [u32; N],
//! }
//!
//! #[derive(Serialize, Debug)]
//! pub struct Vector {
//!     #[serde(with = "as_array")]
//!     pub data: Vec<u32>,
//! }
//! ```

use serde::ser::{Serialize, SerializeTuple, Serializer};

pub fn serialize<S, T>(v: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    // Because specialization in rust is not stable yet, Serde does not implement
    // Serialize for arrays larger than 32 elements, nor for const generic arrays.
    // This allows serialization by using `#[serde(with = "as_array")]`, without
    // requiring a new type with a serialize function.
    let mut s = serializer.serialize_tuple(v.len())?;
    for e in v {
        s.serialize_element(e)?;
    }
    s.end()
}
