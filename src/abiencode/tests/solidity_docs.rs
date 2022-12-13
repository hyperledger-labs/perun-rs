use super::*;
use types::U256;

// The following tests come from the solidity documentation:
// https://docs.soliditylang.org/en/v0.8.17/abi-spec.html#examples
//
// The code below intentionally does NOT include the method ID (4 bytes), as
// that is not representable in the serializer and can be added afterwards.
//
// The solidity docs explains function arguments, which does not include the
// 0x20 offset of structs, therefore we use serialize_and_compare_fnargs.

/*
contract Foo {
    function bar(bytes3[2] memory) public pure {}
    function baz(uint32 x, bool y) public pure returns (bool r) { r = x > 32 || y; }
    function sam(bytes memory, bool, uint[] memory) public pure {}
}
*/
#[test]
fn foo_baz() {
    #[derive(Serialize, Debug)]
    struct Baz(u32, bool);

    let d = Baz(69, true);

    let expected = "
0000000000000000000000000000000000000000000000000000000000000045
0000000000000000000000000000000000000000000000000000000000000001
    ";
    serialize_and_compare(&d, expected);
}

#[test]
fn foo_baz_return() {
    serialize_and_compare_fnargs(
        &true,
        "0000000000000000000000000000000000000000000000000000000000000001",
    );
    serialize_and_compare_fnargs(
        &false,
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
}

mod foo_bar {
    use crate::abiencode::types::Bytes3;

    use super::*;

    fn run<T>(d: &T)
    where
        T: Serialize,
    {
        let expected = "
6162630000000000000000000000000000000000000000000000000000000000
6465660000000000000000000000000000000000000000000000000000000000
    ";
        serialize_and_compare_fnargs(&d, expected);
    }

    #[test]
    fn via_attr() {
        // This one is more an example of how it could be done, Using the `Bytes3`
        // type is easier to read.

        #[derive(Serialize, Debug)]
        struct Bar([Bytes3; 2]);

        let d = Bar([Bytes3(*b"abc"), Bytes3(*b"def")]);
        run(&d);
    }

    #[test]
    fn via_type() {
        use types::Bytes3;

        #[derive(Serialize, Debug)]
        struct Bar([Bytes3; 2]);

        let d = Bar([Bytes3(*b"abc"), Bytes3(*b"def")]);
        run(&d);
    }
}

mod foo_sam {

    use super::*;

    fn run<T>(d: &T)
    where
        T: Serialize,
    {
        let expected = "
0000000000000000000000000000000000000000000000000000000000000060 // [0] (bytes) offset
0000000000000000000000000000000000000000000000000000000000000001 // [1] (bool)
00000000000000000000000000000000000000000000000000000000000000a0 // [2] (uint[]) offset
    0000000000000000000000000000000000000000000000000000000000000004 // [0] (bytes) length
    6461766500000000000000000000000000000000000000000000000000000000 // [0] (bytes)

    0000000000000000000000000000000000000000000000000000000000000003 // [2] (uint[]) length
    0000000000000000000000000000000000000000000000000000000000000001 // [2][0]
    0000000000000000000000000000000000000000000000000000000000000002 // [2][1]
    0000000000000000000000000000000000000000000000000000000000000003 // [2][2]
        ";
        serialize_and_compare_fnargs(&d, expected);
    }

    #[test]
    fn via_array() {
        // Technically we can have more than 4 bytes, however I've choosen to
        // represent it in Rust as 4 bytes instead of using a vector (see
        // `via_vectors`). Additionally I've decided to (as an example) represent
        // the uint (256) as u64, see `via_u256` for a test/example without this
        // arbitrary restriction on the rust side and `no_restrictions` for a
        // variant without any restrictions compared to the solidity
        // representation.
        #[derive(Serialize, Debug)]
        struct Sam(
            #[serde(with = "as_bytes")] [u8; 4],
            bool,
            #[serde(with = "as_dyn_array")] [u64; 3],
        );

        let d = Sam(*b"dave", true, [1, 2, 3]);
        run(&d);
    }

    #[cfg(feature = "std")]
    #[test]
    fn via_vectors() {
        #[derive(Serialize, Debug)]
        struct Sam(#[serde(with = "as_bytes")] Vec<u8>, bool, Vec<u64>);

        let d = Sam(vec![0x64, 0x61, 0x76, 0x65], true, vec![1, 2, 3]);
        run(&d);
    }

    #[test]
    fn via_array_u128() {
        #[derive(Serialize, Debug)]
        struct Sam(
            #[serde(with = "as_bytes")] [u8; 4],
            bool,
            #[serde(with = "as_dyn_array")] [u128; 3],
        );

        let d = Sam(*b"dave", true, [1, 2, 3]);
        run(&d);
    }

    #[test]
    fn via_array_u256() {
        #[derive(Serialize, Debug)]
        struct Sam(
            #[serde(with = "as_bytes")] [u8; 4],
            bool,
            #[serde(with = "as_dyn_array")] [U256; 3],
        );

        let d = Sam(*b"dave", true, [1.into(), 2.into(), 3.into()]);
        run(&d);
    }

    #[cfg(feature = "std")]
    #[test]
    fn no_restrictions() {
        #[derive(Serialize, Debug)]
        struct Sam(#[serde(with = "as_bytes")] Vec<u8>, bool, Vec<U256>);
        let d = Sam(
            vec![0x64, 0x61, 0x76, 0x65],
            true,
            vec![1.into(), 2.into(), 3.into()],
        );
        run(&d);
    }
}

#[cfg(feature = "std")]
#[test]
fn dynamictypes_f() {
    use types::Bytes10;

    /*
    f(uint256,uint32[],bytes10,bytes)
    */

    #[derive(Serialize, Debug)]
    struct Data(U256, Vec<u32>, Bytes10, #[serde(with = "as_bytes")] Vec<u8>);

    let d = Data(
        0x123.into(),
        vec![0x456, 0x789],
        Bytes10(*b"1234567890"),
        "Hello, world!".into(),
    );

    let expected = "
0000000000000000000000000000000000000000000000000000000000000123 // [0]
0000000000000000000000000000000000000000000000000000000000000080 // [1] offset
3132333435363738393000000000000000000000000000000000000000000000 // [2]
00000000000000000000000000000000000000000000000000000000000000e0 // [3] offset
    0000000000000000000000000000000000000000000000000000000000000002 // [1] length
    0000000000000000000000000000000000000000000000000000000000000456 // [1][0]
    0000000000000000000000000000000000000000000000000000000000000789 // [1][1]

    000000000000000000000000000000000000000000000000000000000000000d // [3] length
    48656c6c6f2c20776f726c642100000000000000000000000000000000000000 // [3]
    ";

    serialize_and_compare_fnargs(&d, expected);
}

#[cfg(feature = "std")]
#[test]
fn dynamictypes_g() {
    /*
    g(uint256[][],string[])
    */

    // For simplicity we're just storing string literals, thus avoiding having
    // to deal with livetimes for this one test case. Using `String` would be
    // possible, too, if we implement a custom serialize method or enable the
    // feature `serde/std`.
    #[derive(Serialize, Debug)]
    struct Data(Vec<Vec<U256>>, Vec<&'static str>);

    let d = Data(
        vec![vec![1.into(), 2.into()], vec![3.into()]],
        vec!["one", "two", "three"],
    );

    let expected = "
0000000000000000000000000000000000000000000000000000000000000040 // offset of [[1, 2], [3]]
0000000000000000000000000000000000000000000000000000000000000140 // offset of [\"one\", \"two\", \"three\"]
    0000000000000000000000000000000000000000000000000000000000000002 // count for [[1, 2], [3]]
    0000000000000000000000000000000000000000000000000000000000000040 // offset of [1, 2]
    00000000000000000000000000000000000000000000000000000000000000a0 // offset of [3]
        0000000000000000000000000000000000000000000000000000000000000002 // count for [1, 2]
        0000000000000000000000000000000000000000000000000000000000000001 // encoding of 1
        0000000000000000000000000000000000000000000000000000000000000002 // encoding of 2

        0000000000000000000000000000000000000000000000000000000000000001 // count for [3]
        0000000000000000000000000000000000000000000000000000000000000003 // encoding of 3

    0000000000000000000000000000000000000000000000000000000000000003 // count for [\"one\", \"two\", \"three\"]
    0000000000000000000000000000000000000000000000000000000000000060 // offset for \"one\"
    00000000000000000000000000000000000000000000000000000000000000a0 // offset for \"two\"
    00000000000000000000000000000000000000000000000000000000000000e0 // offset for \"three\"
        0000000000000000000000000000000000000000000000000000000000000003 // count for \"one\"
        6f6e650000000000000000000000000000000000000000000000000000000000 // encoding of \"one\"

        0000000000000000000000000000000000000000000000000000000000000003 // count for \"two\"
        74776f0000000000000000000000000000000000000000000000000000000000 // encoding of \"two\"

        0000000000000000000000000000000000000000000000000000000000000005 // count for \"three\"
        7468726565000000000000000000000000000000000000000000000000000000 // encoding of \"three\"
    ";

    serialize_and_compare_fnargs(&d, expected);
}
