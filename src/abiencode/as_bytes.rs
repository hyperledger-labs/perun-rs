//! Serialize any `&[u8]` as solidity `bytes` (dynamic length bytes).
//!
//! Without this, it would be serialized to a `uint8[]` of fixed or dynamic
//! length.
//!
//! # Example usage
//! ```ignore
//! # // We cannot run this test because abiencode is not public.
//! # use serde::Serialize;
//! # use perun::abiencode::as_bytes;
//!
//! #[derive(Serialize, Debug)]
//! pub struct Array {
//!     #[serde(with = "as_bytes")]
//!     pub data: [u8; 4],
//! }
//!
//! #[derive(Serialize, Debug)]
//! pub struct ConstGeneric<const N: usize> {
//!     #[serde(with = "as_bytes")]
//!     pub data: [u8; N],
//! }
//!
//! #[derive(Serialize, Debug)]
//! pub struct Vector {
//!     #[serde(with = "as_bytes")]
//!     pub data: Vec<u8>,
//! }
//! ```

use super::ser::DynamicMarker;
use serde::{ser::SerializeTuple, Serialize, Serializer};

/// Internal data structure allowing us to serialize the data using
/// `serialize_bytes`, which unfortunately cannot be specified when calling
/// `serialize_element`.
struct Bytes<'a>(&'a [u8]);

impl<'a> Serialize for Bytes<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.0)
    }
}

pub fn serialize<'a, S>(
    v: &[u8],
    serializer: S,
) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_tuple(3)?;
    s.serialize_element(&DynamicMarker)?; // Mark tuple as dynamic (needed for correct encoding)
    s.serialize_element(&v.len())?; // Write length (intentionally not included when writing the data)
    s.serialize_element(&Bytes(v))?; // Write data
    s.end()
}
