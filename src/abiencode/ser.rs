//! Serialize Rust types (including structs) like Solidity's `abi.encode(...)`.
//!
//! Optimized for low-memory devices that do not need/want the entire serialized
//! value in memory at the same time. For example because it is only needed in
//! this format for hashing or signing.
//!
//! # Important
//! This Serializer goes over the objects multiple times at runtime. The length
//! of dynamically-sized types (like `Vec<T>` MUST-NOT change during the
//! serialization). When using custom [Serialize] trait implementations or when
//! using `#[serde(with = "...")]` the borrow checker CAN-NOT ensure this
//! behavior, for example when generating a random `bytes` value inside of
//! [Serialize::serialize()].
//!
//! # Difference to other Serializers
//! To abi encode a dynamic length value (e.g. a struct/tuple) we have to know
//! first write the static sized parts (Head), including the offset for any
//! dynamic length field, before we can write the fields that have a dynamic
//! length (e.g. `uint256[]` or `bytes`).
//!
//! While we could buffer the output by storing it in memory until everything is
//! processed (if there is any dynamic length field the entire data structure
//! has to be buffered because the first byte may be known last), this would not
//! be ideal, especially on embedded devices. Mainly because abi encoded values
//! are really space in-efficient: A lot of offsets and everything is padded to
//! 256 bits.
//!
//! Implementing it as suggested by the Solidity docs is not ideal for embedded
//! devices either, because it still requires the entire serialized value to be
//! stored in memory.
//!
//! Instead we go over the data structure multiple times:
//! 1. [Pass::HeadSize]: Collect the length of the static-length part (Head) for
//!    each type. This can be done at compile-time but currently is not (most
//!    likely). Additionally, this Pass collects information on whether a type
//!    is dynamic or not, which the next Pass needs to know.
//! 2. [Pass::Head]: Write the static part. For all types containing
//!    dynamic-length information (including structs with dynamic fields), only
//!    an offset to where the dynamic part will be is written in this Pass.
//! 3. [Pass::TailSize]: Collect the length of the dynamic-length part (Tail)
//!    for each type we have to write an offset for (called from the Head pass).
//!    This cannot be done at compile-time and must be separate from the Head
//!    pass, since the Head pass for dynamic structs and lists (sequences) has
//!    to be written inside its Tail.
//! 4. [Pass::Tail]: Write the dynamic part. Structs containing dynamic-length
//!    types execute [Pass::Head] from here instead of from [Pass::Head],
//!    otherwise the data for staticly-sized types would be written in the wrong
//!    place.
//!
//! # Potential Improvements
//! - Make sure that computing [Pass::HeadSize] happens only once, at
//!   compile-time
//! - Make sure [Pass::TailSize] is sufficiently inlined, as it needs to be
//!   known in all parents (structs/lists/sequences) to be able to write
//!   offsets. In some situations it may not even be needed, I don't know if
//!   those are optimized away by the compiler.
//! - Look at which part the compiler actually optimizes away and what is
//!   inlined. We may have to implement the passes differently if the compiler
//!   doesn't inline them due to the match statement.

use super::error::{Error, Result};
use serde::{
    ser::{
        self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Serialize,
};

/// Type name used for marking a struct as fake-dynamic (dynamic but
/// transparent).
///
/// See [DynamicMarker] for why we need this. The characters have no special
/// meaning, they have just been chosen in a way that normal Rust types will
/// never have this name.
const MARK_DYNAMIC_NAME: &str = ":$&_DYNAMIC";

// Mark the struct this is serialized in as dynamic, even though all of its
// fields are not, without causing an additional indirection.

/// PhantomData type to mark a struct/tuple as dynamic, even if its fields
/// content are not dynamic.
///
/// Due to limitations of the [serde::Serializer] trait we cannot represent the
/// solidity types `bytes` and `bytes32` at the same time. `bytes32` and other
/// fixed-size bytes must be able to write a 32 byte slot to the serializer.
/// Therefore, [Serializer.serialize_bytes][Serializer#method.serialize_bytes]
/// (usually used for writing bytes) cannot be used for `bytes`, which require
/// that the type is marked as dynamic. We have to represent one of those types
/// differently:
///
/// The `bytes` type is serialized as a tuple (see [as_bytes][super::as_bytes]):
/// - A [DynamicMarker] to force the struct to be dynamic but at the same time
///   transparent (i.e. don't put it's content in the Tail and write an offset
///   in Head).
/// - The length (number of bytes without padding)
/// - The data, padded to the [SLOT_SIZE]
///
/// This allows us to use
/// [Serializer.serialize_bytes][Serializer#method.serialize_bytes] to write
/// arbitrary slots of bytes padded to [SLOT_SIZE].
///
/// Alternatively we could have stored fixed-size bytes like `bytes32` as a
/// length-1 tuple of `U256` values, but this would not be trivial, as we would
/// not be able to use
/// [Serializer.serialize_bytes][Serializer#method.serialize_bytes] and would
/// have to convert bytes to a number and back every time.
///
/// # Important
/// Be careful when using this Type directly. When used wrong the resulting
/// serialized bytes may not represent anything in Solidity. When possible use
/// [as_bytes][super::as_bytes] or [as_dyn_array][super::as_dyn_array] instead.
pub struct DynamicMarker;
impl Serialize for DynamicMarker {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_unit_struct(MARK_DYNAMIC_NAME)
    }
}

/// Flag to prints additional information on non-data slots via stdout.
///
/// Useful when the result differs from the expected value. After returning a
/// slot that does not contain the data itself, the [Serializer] prints a line
/// via stdout to explain the meaning of the last byte.
///
/// This may be moved to a feature flag at some point.
#[cfg(feature = "std")]
const DO_EXPLAIN: bool = false;

/// Flag to print even more information (which methods on [Serializer] have been
/// called).
///
/// This could probably be done with a debugger, too: Put a tracepoint on every
/// method on the [Serializer]. This is just a convenience feature as an
/// extension to [DO_EXPLAIN] that was really useful while debugging the
/// [Serializer].
///
/// This may be removed completely in the future.
#[cfg(feature = "std")]
const DO_TRACE: bool = false;

/// Helper macro to print slot explanations.
///
/// Does nothing in a no_std environment or when [DO_EXPLAIN] is `false`.
macro_rules! explain {
    ($($arg:tt)*) => {
        #[cfg(feature = "std")]
        if DO_EXPLAIN {
            println!(
                "^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ {}",
                format!($($arg)*)
            );
        }
    };
}

/// Helper function to print called methods on the [Serializer].
///
/// Does nothing in a no_std environment or when [DO_TRACE] is `false`.
#[inline]
fn trace(_method: &str, _pass: &Pass) {
    #[cfg(feature = "std")]
    if DO_TRACE {
        match _pass {
            Pass::HeadSize { .. } => {}
            // Pass::TailSize(_) => {}
            _ => {
                println!("TRACE: {}({:?})", _method, _pass);
            }
        }
    }
}

/// Implement this trait to use [to_writer()].
///
/// [Writer::write()] is called whenever the [Serializer] has serialized new
/// data. Currently this always writes slots of 32 bytes, but this may change in
/// the future.
///
/// We are not using the [std::io::Write] trait because it is not available in a
/// no_std environment. One option to make it compatible with [std::io::Write]
/// would be to auto-implement this trait for everything implementing
/// [std::io::Write], but this requires storing the [std::io::Error] in [Error]
/// and a few other changes.
pub trait Writer {
    fn write(&mut self, slot: &[u8]);
}

/// Implementation of [Writer] that does nothing when receiving serialized data.
///
/// Currently used during the size computation ([Pass::HeadSize] and
/// [Pass::TailSize]).
struct NoWriter;

impl Writer for NoWriter {
    fn write(&mut self, _: &[u8]) {
        panic!("do not write to a NoWriter!");
    }
}

#[derive(Debug)]
enum Pass {
    // Don't serialize, just calculate length of Head and whether the type is
    // dynamic. We need the head_size to calculate offsets for dynamic types and
    // we need to know if the type is dynamic to beginn with Pass::Head. Both
    // could be computed at compile time. is_dynamic is stored outside of Pass
    // because it is needed by all Passes.
    HeadSize {
        size: usize,
        is_dynamic: bool,
        is_fake_dynamic: bool,
    },
    Head {
        offset: usize,
    }, // First pass: Write the static part (stores the offset for the next dynamic value)
    TailSize(usize),
    Tail, // Second pass: Write the dynamic part
}

pub struct Serializer<'a, W>
where
    W: Writer,
{
    writer: &'a mut W,
    pass: Pass,
}

pub fn to_writer<T, W>(value: &T, writer: &mut W) -> Result<()>
where
    T: Serialize,
    W: Writer,
{
    to_writer_internal(value, writer, true)
}

#[cfg(test)]
pub fn to_fnargs_writer<T, W>(value: &T, writer: &mut W) -> Result<()>
where
    T: Serialize,
    W: Writer,
{
    to_writer_internal(value, writer, false)
}

fn to_writer_internal<T, W>(value: &T, writer: &mut W, include_outer_struct: bool) -> Result<()>
where
    T: Serialize,
    W: Writer,
{
    let (head_size, is_dynamic, _) = compute_size(&value)?;

    let mut serializer = Serializer {
        writer,
        pass: Pass::Head { offset: head_size },
    };

    if is_dynamic && include_outer_struct {
        serializer.write_right_aligned(SLOT_SIZE.to_be_bytes())
    }

    value.serialize(&mut serializer)?;
    if is_dynamic {
        serializer.pass = Pass::Tail;
        value.serialize(&mut serializer)?;
    }
    Ok(())
}

fn compute_size<T>(value: &T) -> Result<(usize, bool, bool)>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        writer: &mut NoWriter,
        pass: Pass::HeadSize {
            size: 0,
            is_dynamic: false,
            is_fake_dynamic: false,
        },
    };
    value.serialize(&mut serializer)?;

    if let Pass::HeadSize {
        size,
        is_dynamic,
        is_fake_dynamic,
    } = serializer.pass
    {
        Ok((size, is_dynamic, is_fake_dynamic))
    } else {
        unreachable!(
            "This should never happen if the serializer does not modify its own pass variable!"
        )
    }
}

const SLOT_SIZE: usize = 32; // bytes

impl<'a, W> Serializer<'a, W>
where
    W: Writer,
{
    // Panics if N>SLOT_SIZE
    fn write_left_aligned_slice(&mut self, v: &[u8]) {
        let mut bytes: [u8; SLOT_SIZE] = Default::default();
        bytes[..v.len()].copy_from_slice(v);
        self.writer.write(bytes.as_slice());
    }

    // Panics if N>SLOT_SIZE
    fn write_right_aligned<const N: usize>(&mut self, v: [u8; N]) {
        let mut bytes: [u8; SLOT_SIZE] = Default::default();
        bytes[SLOT_SIZE - N..].copy_from_slice(v.as_slice());
        self.writer.write(bytes.as_slice())
    }

    // Panics if N>SLOT_SIZE
    fn write_signed<const N: usize>(&mut self, negative: bool, v: [u8; N]) {
        let filler = if negative { 0xff } else { 0x00 };
        let mut bytes: [u8; SLOT_SIZE] = [filler; SLOT_SIZE];
        bytes[SLOT_SIZE - N..].copy_from_slice(v.as_slice());
        self.writer.write(bytes.as_slice())
    }

    fn serialize<T>(&mut self, value: &T, pass: Pass) -> Result<()>
    where
        T: Serialize,
    {
        let mut serializer = Serializer {
            writer: self.writer,
            pass,
        };
        value.serialize(&mut serializer)?;
        Ok(())
    }

    fn get_tail_size<T>(value: &T) -> Result<usize>
    where
        T: Serialize,
    {
        let mut serializer = Serializer {
            writer: &mut NoWriter,
            pass: Pass::TailSize(0),
        };
        value.serialize(&mut serializer)?;
        // This can only panic if the serializer changes the pass variable.
        if let Pass::TailSize(tail_size) = serializer.pass {
            Ok(tail_size)
        } else {
            unreachable!(
                "This should never happen because the serializer does not modify its own pass variable variant"
            )
        }
    }

    // Multiple Serde types need the same behavior: Write the entire type in
    // Pass::Head if the type is static and write the Head in Pass::Tail if it
    // isn't.
    //
    // This helper function does exactly that to avoid code duplication by
    // making it easier to execute this behavior. The "tuple" here refers to
    // tuples in the abi specs, which are basically structs in Rust. It is also
    // used in Sequences (Lists).
    //
    // Set offset_reduction=0 unless there is something (like the length in
    // sequences) that has to be written in Pass::Head (and thus has an effect
    // on the total size of the encoded value), but does not count towards the
    // offset.
    fn serialize_tuple_element<T: ?Sized>(
        &mut self,
        name: Option<&'static str>,
        value: &T,
        offset_reduction: usize,
    ) -> Result<()>
    where
        T: Serialize,
    {
        match self.pass {
            Pass::HeadSize {
                ref mut size,
                ref mut is_dynamic,
                ..
            } => {
                let (element_size, is_dyn, is_fake_dynamic) = compute_size(&value)?;
                // Unfortunately we can't use mutable references in the match
                // statement because compute_size requires a reference, too.
                // TODO: Make compute_size not use self or value and ideally
                // compute it at compile time.

                *size += if is_dyn && !is_fake_dynamic {
                    SLOT_SIZE
                } else {
                    element_size
                };

                *is_dynamic |= is_dyn || is_fake_dynamic;
                Ok(())
            }
            Pass::Head { offset } => {
                let (field_head_size, is_dyn, is_fake_dynamic) = compute_size(&value)?;
                if is_dyn && !is_fake_dynamic {
                    // The length (only used in Serde Sequences = Solidity dynamic
                    // length arrays) is part of the Head pass (as it is written
                    // there), but according to the specs and testing in Solidity it
                    // is not included when calculating the offset for any dynamic
                    // fields/elements. Because this is the only difference between
                    // sequence elements and struct fields the offset_reduction
                    // param allows using this function for both.
                    self.write_right_aligned((offset - offset_reduction).to_be_bytes());
                    match name {
                        Some(_name) => {
                            explain!("{} offset (HEAD)", _name);
                        }
                        None => {
                            explain!("element/field offset (HEAD)");
                        }
                    };

                    self.pass = Pass::Head {
                        offset: offset + field_head_size + Self::get_tail_size(&value)?,
                    };
                    Ok(())
                } else {
                    // The element is not dynamic (nor faked-dynamic) => it does
                    // not have a tail => It never writes offsets => The value
                    // for offsets doesn't matter. => We can't test if this is
                    // correct.
                    //
                    // There isn't a good way to detect when it is read either,
                    // so we just fail as loudly as we can: In debug builds we
                    // panic due to an integer overflow whenever offset is
                    // changed and when it is read it will be immediately
                    // noticable from the output (when printed/examined). In
                    // release builds this will write hex 0xFFFFFFFFFFFFFFFF as
                    // the offset, which is unlikely to occur normally (still
                    // possible as a U256 of course).
                    self.serialize(&value, Pass::Head { offset: usize::MAX })
                }
            }
            Pass::TailSize(size) => {
                let (field_head_size, is_dyn, is_fake_dynamic) = compute_size(&value)?;
                let field_tail_size = Self::get_tail_size(&value)?;
                self.pass = Pass::TailSize(
                    size + if is_dyn && !is_fake_dynamic {
                        field_head_size
                    } else {
                        0
                    } + field_tail_size,
                );
                Ok(())
            }
            Pass::Tail => {
                let (field_head_size, is_dyn, is_fake_dynamic) = compute_size(&value)?;
                if is_dyn && !is_fake_dynamic {
                    // This offset might be counter intuitive (I've thought
                    // about it wrong multiple times). It does NOT have an
                    // affect on the sequence this element is part of but
                    // instead on all children of the element. As in the
                    // to_writer function we have to give the Serializer the
                    // head size of the value it should serialize, otherwise it
                    // does not know where the dynamic part (Tail) begins and
                    // thus cannot write offsets in Pass::Head.
                    self.serialize(
                        &value,
                        Pass::Head {
                            offset: field_head_size,
                        },
                    )?;
                    self.serialize(&value, Pass::Tail)
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl<'a, 'b, W> ser::Serializer for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_u8(if v { 1 } else { 0 })
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        trace("serialize_i8", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_signed(v < 0, v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        trace("serialize_i16", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_signed(v < 0, v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        trace("serialize_i32", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_signed(v < 0, v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        trace("serialize_i64", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_signed(v < 0, v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        trace("serialize_i128", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_right_aligned(v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        trace("serialize_u8", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_right_aligned(v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        trace("serialize_u16", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_right_aligned(v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        trace("serialize_u32", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_right_aligned(v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        trace("serialize_u64", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_right_aligned(v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        trace("serialize_u128", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => *size += SLOT_SIZE,
            Pass::Head { .. } => self.write_right_aligned(v.to_be_bytes()),
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        };
        Ok(())
    }

    fn serialize_f32(self, _: f32) -> Result<()> {
        trace("serialize_f32", &self.pass);
        Err(Error::TypeNotRepresentable("f32"))
    }

    fn serialize_f64(self, _: f64) -> Result<()> {
        trace("serialize_f64", &self.pass);
        Err(Error::TypeNotRepresentable("f64"))
    }

    fn serialize_char(self, _: char) -> Result<()> {
        trace("serialize_char", &self.pass);
        Err(Error::TypeNotYetSupported("char"))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        trace("serialize_str", &self.pass);
        // This is basically the same as for dynamic length bytes, except that
        // we don't need serialize_str for anything else and thus do not have to
        // rely on DynamicMarker and an additional tuple.
        match self.pass {
            Pass::HeadSize {
                ref mut is_dynamic, ..
            } => {
                *is_dynamic = true;
            }
            Pass::Head { .. } => {}
            Pass::TailSize(ref mut size) => {
                // Calculate the amount of slots the dynamic part needs to
                // forward the offset. The length is in bytes.
                let r = v.len() % SLOT_SIZE;
                //                        length + chunks        + rem
                let tail_size = SLOT_SIZE + (v.len() - r) + (if r == 0 { 0 } else { SLOT_SIZE });

                *size += tail_size;
            }
            Pass::Tail => {
                self.write_right_aligned(v.len().to_be_bytes());
                explain!("str size (TAIL)");

                let iter = v.as_bytes().chunks_exact(SLOT_SIZE);
                let rem = iter.remainder();
                for chunk in iter {
                    self.writer.write(chunk);
                }
                if rem.len() > 0 {
                    self.write_left_aligned_slice(rem);
                }
            }
        };
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        trace("serialize_bytes", &self.pass);
        match self.pass {
            Pass::HeadSize { ref mut size, .. } => {
                let r = v.len() % SLOT_SIZE;
                //                   size + chunks        + rem
                *size += (v.len() - r) + (if r == 0 { 0 } else { SLOT_SIZE });
            }
            Pass::Head { .. } => {
                let iter = v.chunks_exact(SLOT_SIZE);
                let rem = iter.remainder();
                for chunk in iter {
                    self.writer.write(chunk);
                }
                if rem.len() > 0 {
                    self.write_left_aligned_slice(rem);
                }
            }
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        }
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        trace("serialize_none", &self.pass);
        Err(Error::TypeNotRepresentable("none"))
    }

    fn serialize_some<T: ?Sized>(self, _: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace("serialize_some", &self.pass);
        Err(Error::TypeNotRepresentable("some"))
    }

    fn serialize_unit(self) -> Result<()> {
        trace("serialize_unit", &self.pass);
        Err(Error::TypeNotRepresentable("unit"))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        if name == MARK_DYNAMIC_NAME {
            trace("serialize_unit_struct (mark dynamic)", &self.pass);
            match self.pass {
                Pass::HeadSize {
                    ref mut is_fake_dynamic,
                    ..
                } => {
                    *is_fake_dynamic = true;
                }
                Pass::Head { .. } => {}
                Pass::TailSize(_) => {}
                Pass::Tail => {}
            }
            Ok(())
        } else {
            trace("serialize_unit_struct", &self.pass);
            Err(Error::TypeNotRepresentable("unit struct"))
        }
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<()> {
        trace("serialize_unit_variant", &self.pass);
        Err(Error::TypeNotRepresentable("unit variant (enum)"))
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace("serialize_newtype_struct", &self.pass);
        self.serialize_tuple_element(Some(name), value, 0)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        trace("serialize_newtype_variant", &self.pass);
        Err(Error::TypeNotRepresentable("newtype variant (enum)"))
    }

    fn serialize_seq(self, size: Option<usize>) -> Result<Self::SerializeSeq> {
        trace("serialize_seq", &self.pass);
        match self.pass {
            Pass::HeadSize {
                ref mut size,
                ref mut is_dynamic,
                ..
            } => {
                *is_dynamic = true;
                *size += SLOT_SIZE;
            }
            Pass::Head { .. } => {
                self.write_right_aligned(size.unwrap().to_be_bytes());
                explain!("seq size (TAIL)");
            }
            Pass::TailSize(_) => {}
            Pass::Tail => {}
        }
        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple> {
        trace("serialize_tuple", &self.pass);
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        trace("serialize_tuple_struct", &self.pass);
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        trace("serialize_tuple_variant", &self.pass);
        Err(Error::TypeNotRepresentable("struct variant"))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap> {
        trace("serialize_map", &self.pass);
        Err(Error::TypeNotRepresentable("map"))
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct> {
        trace("serialize_struct", &self.pass);
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        trace("serialize_struct_variant", &self.pass);
        Err(Error::TypeNotRepresentable("struct variant"))
    }

    #[cfg(not(feature = "std"))]
    fn collect_str<T: ?Sized>(self, _value: &T) -> Result<()>
    where
        T: core::fmt::Display,
    {
        trace("collect_str", &self.pass);
        unimplemented!()
    }
}

impl<'a, 'b, W> SerializeSeq for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace("Seq: serialize_element", &self.pass);
        // The sequence length (written in Pass::Head) is not part of the offset
        // calculation for sequence elements, see comment inside of
        // serialize_tuple_element.
        self.serialize_tuple_element(None, value, SLOT_SIZE)
    }

    fn end(self) -> Result<()> {
        trace("Seq: end", &self.pass);
        Ok(())
    }
}

impl<'a, 'b, W> SerializeTuple for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace("Tuple: serialize_element", &self.pass);
        self.serialize_tuple_element(None, value, 0)
    }

    fn end(self) -> Result<()> {
        trace("Tuple: end", &self.pass);
        Ok(())
    }
}

impl<'a, 'b, W> SerializeTupleStruct for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace("TupleStruct: serialize_field", &self.pass);
        self.serialize_tuple_element(None, value, 0)
    }

    fn end(self) -> Result<()> {
        trace("TupleStruct: end", &self.pass);
        Ok(())
    }
}

impl<'a, 'b, W> SerializeTupleVariant for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        unreachable!("Because serialize_tuple_variant never returns Ok")
    }

    fn end(self) -> Result<()> {
        unreachable!("Because serialize_tuple_variant never returns Ok")
    }
}

impl<'a, 'b, W> SerializeMap for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();

    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<()>
    where
        T: Serialize,
    {
        unreachable!("Because serialize_map never returns Ok")
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        unreachable!("Because serialize_map never returns Ok")
    }

    fn end(self) -> Result<()> {
        unreachable!("Because serialize_map never returns Ok")
    }
}

impl<'a, 'b, W> SerializeStruct for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace("Struct: serialize_field", &self.pass);
        self.serialize_tuple_element(Some(name), value, 0)
    }

    fn end(self) -> Result<()> {
        trace("Struct: end", &self.pass);
        Ok(())
    }
}

impl<'a, 'b, W> SerializeStructVariant for &'a mut Serializer<'b, W>
where
    W: Writer,
{
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        unreachable!("Because serialize_struct_variant never returns Ok")
    }

    fn end(self) -> Result<()> {
        unreachable!("Because serialize_struct_variant never returns Ok")
    }
}
