//! Serialize any `&[T]` as solidity `T[]` (dnymic length array).
//!
//! Useful to hide a `[T; N]` as a dynamic length array with a compile-time
//! known length, for example in a no_std environment where `Vec<T>` is not
//! available or when it is not desireable or needed to have dynamic lengths on
//! the Rust side.
//!
//! # Example usage
//! ```ignore
//! # // We cannot run this test because abiencode is not public.
//! # use serde::Serialize;
//! # use perun::abiencode::as_array;
//!
//! #[derive(Serialize, Debug)]
//! pub struct Array {
//!     #[serde(with = "as_array")]
//!     pub data: [u8; 4],
//! }
//!
//! #[derive(Serialize, Debug)]
//! pub struct ConstGeneric<const N: usize> {
//!     #[serde(with = "as_array")]
//!     pub data: [u32; N],
//! }
//! ```

use serde::ser::{Serialize, SerializeSeq, Serializer};

pub fn serialize<S, T>(v: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut s = serializer.serialize_seq(Some(v.len()))?;
    for e in v {
        s.serialize_element(e)?;
    }
    s.end()
}
