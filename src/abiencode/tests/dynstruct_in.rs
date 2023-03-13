use super::*;
use bytescontainer::{BytesContainer, BytesContainerViaNormalAttr, BytesContainerViaTupleAttr};

mod fixed {
    use super::*;

    fn run<T>()
    where
        T: Serialize,
        T: BytesContainer,
    {
        /*
        ```solidity
            struct DynstructInFixedarrayInnerData {
                bytes v;
            }
            struct DynstructInFixedarrayData {
                DynstructInFixedarrayInnerData[2] a;
                bytes b;
            }
            function DynstructInFixedarray() public pure returns(bytes memory) {
                DynstructInFixedarrayData memory d;
                d.a[0].v = "\xa1\xa2\xa3\xa4";
                d.a[1].v = "\xb1\xb2\xb3\xb4";
                d.b = "\x11\x22\x33\x44\x55";
                return abi.encode(d);
            }
        ```
        */

        #[derive(Serialize, Debug)]
        struct DynstructInFixedarray<T> {
            a: [T; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = DynstructInFixedarray {
            a: [T::gen(0xa0), T::gen(0xb0)],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        let expected = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    0000000000000000000000000000000000000000000000000000000000000040 // d.a offset
    0000000000000000000000000000000000000000000000000000000000000140 // d.b offset
        0000000000000000000000000000000000000000000000000000000000000040 // d.a[0] offset
        00000000000000000000000000000000000000000000000000000000000000a0 // d.a[1] offset
            0000000000000000000000000000000000000000000000000000000000000020 // d.a[0].v offset
                0000000000000000000000000000000000000000000000000000000000000004 // d.a[0].v length
                a1a2a3a400000000000000000000000000000000000000000000000000000000 // d.a[0].v

            0000000000000000000000000000000000000000000000000000000000000020 // d.a[1].v offset
                0000000000000000000000000000000000000000000000000000000000000004 // d.a[1].v length
                b1b2b3b400000000000000000000000000000000000000000000000000000000 // d.a[1].v

        0000000000000000000000000000000000000000000000000000000000000005 // d.b length
        1122334455000000000000000000000000000000000000000000000000000000 // d.b
        ";
        serialize_and_compare(&d, expected);
    }

    #[test]
    fn normal() {
        run::<BytesContainerViaNormalAttr>();
    }

    #[test]
    fn tuple() {
        run::<BytesContainerViaTupleAttr>();
    }
}

mod dynamic {
    use super::*;

    fn run<T>()
    where
        T: Serialize,
        T: BytesContainer,
    {
        /*
        ```solidity
            struct DynstructInDynarrayInnerData {
                bytes v;
            }
            struct DynstructInDynarrayData {
                DynstructInDynarrayInnerData[] a;
                bytes b;
            }
            function DynstructInDynarray() public pure returns(bytes memory) {
                DynstructInDynarrayData memory d;
                d.a = new DynstructInDynarrayInnerData[](2);
                d.a[0].v = "\xa1\xa2\xa3\xa4";
                d.a[1].v = "\xb1\xb2\xb3\xb4";
                d.b = "\x11\x22\x33\x44\x55";
                return abi.encode(d);
            }
        ```
        */

        #[derive(Serialize, Debug)]
        struct DynstructInFixedarray<T>
        where
            T: Serialize,
        {
            #[serde(with = "as_dyn_array")]
            a: [T; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = DynstructInFixedarray {
            a: [T::gen(0xa0), T::gen(0xb0)],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        let expected = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    0000000000000000000000000000000000000000000000000000000000000040 // d.a offset
    0000000000000000000000000000000000000000000000000000000000000160 // d.b offset
        0000000000000000000000000000000000000000000000000000000000000002 // d.a length
        0000000000000000000000000000000000000000000000000000000000000040 // d.a[0] offset
        00000000000000000000000000000000000000000000000000000000000000a0 // d.a[1] offset
            0000000000000000000000000000000000000000000000000000000000000020 // d.a[0].v offset
                0000000000000000000000000000000000000000000000000000000000000004 // d.a[0].v length
                a1a2a3a400000000000000000000000000000000000000000000000000000000 // d.a[0].v
            0000000000000000000000000000000000000000000000000000000000000020 // d.a[1].v offset
                0000000000000000000000000000000000000000000000000000000000000004 // d.a[1].v length
                b1b2b3b400000000000000000000000000000000000000000000000000000000 // d.a[1].v

        0000000000000000000000000000000000000000000000000000000000000005 // d.b length
        1122334455000000000000000000000000000000000000000000000000000000 // d.b
        ";
        serialize_and_compare(&d, expected);
    }

    #[test]
    fn normal() {
        run::<BytesContainerViaNormalAttr>();
    }

    #[test]
    fn tuple() {
        run::<BytesContainerViaTupleAttr>();
    }
}
