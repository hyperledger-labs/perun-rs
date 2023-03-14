mod address;
mod bytes;
mod bytes_in;
mod bytescontainer;
mod dynstruct_in;
mod simple;
mod solidity_docs;
mod static_in;
mod staticstruct_in;
mod string;

use super::*;
use serde::Serialize;
use uint::hex::FromHex;

use core::fmt::Debug;

/*
Add the following before a `serialize_and_compare` call to print the entire serialized output.
```
    let mut writer = PrintWriter {};
    to_writer(&d, &mut writer).unwrap();

    println!("===========================================");
```
*/

/*
Python code to split output from remix into chunks of 32 bytes, the annotations
are done manually, as adding them automatically would require a custom known
good (explaining) serializer.
```python
s = "..."
print(*(s[i:i+64] for i in range(0, len(s), 64)), sep="\n")
```
*/
#[cfg(feature = "std")]
fn print_hex_slice(value: &[u8]) {
    print!("0x");
    for b in value {
        print!("{:02x?}", b);
    }
    println!();
}

#[cfg(feature = "std")]
struct PrintWriter;

#[cfg(feature = "std")]
impl Writer for PrintWriter {
    fn write(&mut self, slot: &[u8]) {
        print_hex_slice(slot);
    }
}

struct AssertWriter<'a, I>
where
    I: Iterator<Item = (&'a str, &'a str)>,
{
    expected_iter: I,
}

struct Slot<'a>(&'a [u8]);

impl<'a> Debug for Slot<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for b in self.0 {
            f.write_fmt(format_args!("{:02x}", b))?;
        }
        Ok(())
    }
}

impl<'a> PartialEq for Slot<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<'a, I> Writer for AssertWriter<'a, I>
where
    I: Iterator<Item = (&'a str, &'a str)>,
{
    fn write(&mut self, slot: &[u8]) {
        match self.expected_iter.next() {
            Some((expected, _line)) => {
                // Make sure the test expects something that makes sense
                assert_eq!(
                    expected.len(),
                    64,
                    "The expected input must be grouped into slots of 32 bytes as hex, without 0x."
                );
                assert_eq!(slot.len(), 32, "Each slot should have 32 bytes.");

                // Print current expected line to make debugging easier.
                #[cfg(feature = "std")]
                println!("{}", _line);

                let expected = <[u8; 32]>::from_hex(expected).unwrap();

                // We're wrapping both in Slot to make assert_eq! format both as
                // a hex string.
                assert_eq!(
                    Slot(slot),
                    Slot(expected.as_slice()),
                    "slot did not match the expected value"
                );
            }
            None => {
                panic!("Expected end of data, got {:?}", slot);
            }
        }
    }
}

macro_rules! expected_iter {
    // Mainly using a macro here because I didn't want to bother with the return
    // type.
    ( $expected:expr ) => {
        // Iterate over the expected content, extracting the slot information
        // (32-byte hex string at the beginning, skipping empty lines). You can add
        // additional comments (// is not required) after the slot to explain what a
        // slot does.
        $expected
            .split("\n")
            .filter(|&line| !line.trim().is_empty())
            .map(|line| {
                if line.len() < 64 {
                    panic!("expected line is too short, it must start with a 32 byte hex string!");
                };
                (
                    &line.trim()[..64], // Data to compare
                    line,               // Line to display
                )
            })
    };
}

pub fn serialize_and_compare_fnargs<T>(value: &T, expected: &str)
where
    T: Serialize,
{
    // encode the value. The AssetWriter compares the returned slots against the
    // iterator.
    let mut writer = AssertWriter {
        expected_iter: expected_iter!(expected),
    };
    ser::to_fnargs_writer(&value, &mut writer).unwrap();

    // Make sure we're not missing a slot.
    let next = writer.expected_iter.next();
    assert_eq!(next, None, "there are less slots than expected.");
}

pub fn serialize_and_compare<T>(value: &T, expected: &str)
where
    T: Serialize,
{
    // encode the value. The AssetWriter compares the returned slots against the
    // iterator.
    let mut writer = AssertWriter {
        expected_iter: expected_iter!(expected),
    };
    to_writer(&value, &mut writer).unwrap();

    // Make sure we're not missing a slot.
    let next = writer.expected_iter.next();
    assert_eq!(next, None, "there are less slots than expected.");
}

// More or less the same as BytesContainer, the only difference is that it is
// encoded in a flattened way (the container itself is not visible).
trait Bytes {
    fn gen(base: u8) -> Self;
}

#[derive(Serialize, Debug)]
#[serde(transparent)]
struct BytesViaTupleAttr(#[serde(with = "as_bytes")] [u8; 4]);
impl Bytes for BytesViaTupleAttr {
    fn gen(base: u8) -> Self {
        return Self([0x01 | base, 0x02 | base, 0x03 | base, 0x04 | base]);
    }
}

#[derive(Serialize, Debug)]
#[serde(transparent)]
struct BytesViaNormalAttr {
    #[serde(with = "as_bytes")]
    value: [u8; 4],
}
impl Bytes for BytesViaNormalAttr {
    fn gen(base: u8) -> Self {
        Self {
            value: [0x01 | base, 0x02 | base, 0x03 | base, 0x04 | base],
        }
    }
}
