use super::*;

pub trait BytesContainer {
    fn gen(base: u8) -> Self;
}

#[derive(Serialize, Debug)]
pub struct BytesContainerViaTupleAttr(#[serde(with = "as_bytes")] [u8; 4]);
impl BytesContainer for BytesContainerViaTupleAttr {
    fn gen(base: u8) -> Self {
        return Self([0x01 | base, 0x02 | base, 0x03 | base, 0x04 | base]);
    }
}

#[derive(Serialize, Debug)]
pub struct BytesContainerViaNormalAttr {
    #[serde(with = "as_bytes")]
    value: [u8; 4],
}
impl BytesContainer for BytesContainerViaNormalAttr {
    fn gen(base: u8) -> Self {
        Self {
            value: [0x01 | base, 0x02 | base, 0x03 | base, 0x04 | base],
        }
    }
}

// Helper method to run two BytesContainer tests that can have two different
// Rust implementations for the same behavior.
fn run<T>()
where
    T: BytesContainer,
    T: Serialize,
{
    /*
    ```solidity
        struct BytesContainerData {
            bytes a;
        }
        function BytesContainer() public pure returns(bytes memory) {
            BytesContainerData memory d;
            d.a = "\xa1\xa2\xa3\xa4";
            return abi.encode(d);
        }
    ```
    */
    let d = T::gen(0xa0);

    let expected = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    0000000000000000000000000000000000000000000000000000000000000020 // d.a offset
        0000000000000000000000000000000000000000000000000000000000000004 // d.a length
        a1a2a3a400000000000000000000000000000000000000000000000000000000 // d.a
    ";
    serialize_and_compare(&d, expected);
}

#[test]
fn normal() {
    run::<BytesContainerViaNormalAttr>()
}

#[test]
fn tuple() {
    run::<BytesContainerViaTupleAttr>()
}
