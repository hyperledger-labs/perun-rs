use super::*;

mod fixed {
    use super::*;

    fn run<T>(d: T)
    where
        T: Serialize,
    {
        /*
        ```solidity
            struct StaticstructInFixedarrayInnerData {
                uint64 v;
            }
            struct StaticstructInFixedarrayData {
                StaticstructInFixedarrayInnerData[2] a;
                bytes b;
            }
            function StaticstructInFixedarray() public pure returns(bytes memory) {
                StaticstructInFixedarrayData memory d;
                d.a[0].v = 0xaa;
                d.a[1].v = 0xbb;
                d.b = "\x11\x22\x33\x44\x55";
                return abi.encode(d);
            }
        ```
        */

        let expected = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    00000000000000000000000000000000000000000000000000000000000000aa // d.a[0].v
    00000000000000000000000000000000000000000000000000000000000000bb // d.a[1].v
    0000000000000000000000000000000000000000000000000000000000000060 // d.b offset
        0000000000000000000000000000000000000000000000000000000000000005 // d.b length
        1122334455000000000000000000000000000000000000000000000000000000 // d.b
    ";
        serialize_and_compare(&d, expected);
    }

    #[test]
    fn normal() {
        // Not transparent because the struct exists on the solidity side, too
        #[derive(Serialize, Debug)]
        struct Inner {
            v: u64,
        }

        #[derive(Serialize, Debug)]
        struct StaticstructInFixedarray {
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticstructInFixedarray {
            a: [Inner { v: 0xaa }, Inner { v: 0xbb }],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }

    #[test]
    fn tuple() {
        // Not transparent because the struct exists on the solidity side, too
        #[derive(Serialize, Debug)]
        struct Inner(u64);

        #[derive(Serialize, Debug)]
        struct StaticstructInFixedarray {
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticstructInFixedarray {
            a: [Inner(0xaa), Inner(0xbb)],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }
}

mod dynamic {
    use super::*;

    fn run<T>(d: T)
    where
        T: Serialize,
    {
        /*
        ```solidity
            struct StaticstructInDynarrayInnerData {
                uint64 v;
            }
            struct StaticstructInDynarrayData {
                StaticstructInDynarrayInnerData[] a;
                bytes b;
            }
            function StaticstructInDynarray() public pure returns(bytes memory) {
                StaticstructInDynarrayData memory d;
                d.a = new StaticstructInDynarrayInnerData[](2);
                d.a[0].v = 0xaa;
                d.a[1].v = 0xbb;
                d.b = "\x11\x22\x33\x44\x55";
                return abi.encode(d);
            }
        ```
        */

        let expected = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    0000000000000000000000000000000000000000000000000000000000000040 // d.a offset
    00000000000000000000000000000000000000000000000000000000000000a0 // d.b offset
        0000000000000000000000000000000000000000000000000000000000000002 // d.a length
        00000000000000000000000000000000000000000000000000000000000000aa // d.a[0]
        00000000000000000000000000000000000000000000000000000000000000bb // d.a[1]

        0000000000000000000000000000000000000000000000000000000000000005 // d.b length
        1122334455000000000000000000000000000000000000000000000000000000 // d.b
        ";
        serialize_and_compare(&d, expected);
    }

    #[test]
    fn normal() {
        // Not transparent because the struct exists on the solidity side, too
        #[derive(Serialize, Debug)]
        struct Inner {
            v: u64,
        }

        #[derive(Serialize, Debug)]
        struct StaticstructInFixedarray {
            #[serde(with = "as_dyn_array")]
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticstructInFixedarray {
            a: [Inner { v: 0xaa }, Inner { v: 0xbb }],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }

    #[test]
    fn tuple() {
        // Not transparent because the struct exists on the solidity side, too
        #[derive(Serialize, Debug)]
        struct Inner(u64);

        #[derive(Serialize, Debug)]
        struct StaticstructInFixedarray {
            #[serde(with = "as_dyn_array")]
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticstructInFixedarray {
            a: [Inner(0xaa), Inner(0xbb)],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }
}

// TODO: dyn_in_fixedarray (ExampleE?)
// TODO: dyn_in_dynarray
