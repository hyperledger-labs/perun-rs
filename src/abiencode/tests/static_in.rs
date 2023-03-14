use super::*;

#[derive(Serialize, Debug)]
struct Container<T> {
    x: u64,
    y: T,
    z: u64,
}

#[test]
fn fixed_in_fixed() {
    /*
    ```solidity
        struct StaticInFixedarrayInFixedarrayData {
            uint64[3][2] a;
            bytes b;
        }
        function StaticInFixedarrayInFixedarray() public pure returns(bytes memory) {
            StaticInFixedarrayInFixedarrayData memory d;
            d.a[0][0] = 0xaa;
            d.a[0][1] = 0xbb;
            d.a[0][2] = 0xcc;
            d.a[1][0] = 0xdd;
            d.a[1][1] = 0xee;
            d.a[1][2] = 0xff;
            d.b = "\x11\x22\x33\x44\x55";
            return abi.encode(d);
        }
    ```
    */

    #[derive(Serialize, Debug)]
    struct StaticInFixedarrayInDynarray {
        a: [[u64; 3]; 2],
        #[serde(with = "as_bytes")]
        b: [u8; 5],
    }

    let d = StaticInFixedarrayInDynarray {
        a: [[0xaa, 0xbb, 0xcc], [0xdd, 0xee, 0xff]],
        b: [0x11, 0x22, 0x33, 0x44, 0x55],
    };

    let expected = "
    0000000000000000000000000000000000000000000000000000000000000020 // d offset
        00000000000000000000000000000000000000000000000000000000000000aa // d.a[0][0]
        00000000000000000000000000000000000000000000000000000000000000bb // d.a[0][1]
        00000000000000000000000000000000000000000000000000000000000000cc // d.a[0][2]
        00000000000000000000000000000000000000000000000000000000000000dd // d.a[1][0]
        00000000000000000000000000000000000000000000000000000000000000ee // d.a[1][1]
        00000000000000000000000000000000000000000000000000000000000000ff // d.a[1][2]
        00000000000000000000000000000000000000000000000000000000000000e0 // d.b offset
            0000000000000000000000000000000000000000000000000000000000000005 // d.b length
            1122334455000000000000000000000000000000000000000000000000000000 // d.b
        ";
    serialize_and_compare(&d, expected);
}

#[test]
fn fixed_in_fixed_container() {
    /*
    ```solidity
        struct StaticInFixedarrayInFixedarrayData {
            uint64[3][2] a;
            bytes b;
        }
        struct StaticInFixedarrayInFixedarrayContainerData {
            uint64 x;
            StaticInFixedarrayInFixedarrayData y;
            uint64 z;
        }
        function StaticInFixedarrayInFixedarrayContainer() public pure returns(bytes memory) {
            StaticInFixedarrayInFixedarrayContainerData memory d;
            d.x = 0x1111;
            d.y.a[0][0] = 0xaa;
            d.y.a[0][1] = 0xbb;
            d.y.a[0][2] = 0xcc;
            d.y.a[1][0] = 0xdd;
            d.y.a[1][1] = 0xee;
            d.y.a[1][2] = 0xff;
            d.y.b = "\x11\x22\x33\x44\x55";
            d.z = 0x2222;
            return abi.encode(d);
        }
    ```
    */

    #[derive(Serialize, Debug)]
    struct StaticInFixedarrayInDynarray {
        a: [[u64; 3]; 2],
        #[serde(with = "as_bytes")]
        b: [u8; 5],
    }

    let y = StaticInFixedarrayInDynarray {
        a: [[0xaa, 0xbb, 0xcc], [0xdd, 0xee, 0xff]],
        b: [0x11, 0x22, 0x33, 0x44, 0x55],
    };
    let d = Container {
        x: 0x1111,
        y,
        z: 0x2222,
    };

    let expected = "
    0000000000000000000000000000000000000000000000000000000000000020 // d offset
        0000000000000000000000000000000000000000000000000000000000001111 // d.x
        0000000000000000000000000000000000000000000000000000000000000060 // d.y offset
        0000000000000000000000000000000000000000000000000000000000002222 // d.z
            00000000000000000000000000000000000000000000000000000000000000aa // d.y.a[0][0]
            00000000000000000000000000000000000000000000000000000000000000bb // d.y.a[0][1]
            00000000000000000000000000000000000000000000000000000000000000cc // d.y.a[0][2]
            00000000000000000000000000000000000000000000000000000000000000dd // d.y.a[1][0]
            00000000000000000000000000000000000000000000000000000000000000ee // d.y.a[1][1]
            00000000000000000000000000000000000000000000000000000000000000ff // d.y.a[1][2]
            00000000000000000000000000000000000000000000000000000000000000e0 // d.y.b offset
                0000000000000000000000000000000000000000000000000000000000000005 // d.y.b length
                1122334455000000000000000000000000000000000000000000000000000000 // d.y.b
        ";
    serialize_and_compare(&d, expected);
}

#[test]
fn fixed_in_dyn() {
    /*
    ```solidity
        struct StaticInFixedarrayInDynarrayData {
            uint64[3][] a;
            bytes b;
        }
        function StaticInFixedarrayInDynarray() public pure returns(bytes memory) {
            StaticInFixedarrayInDynarrayData memory d;
            d.a = new uint64[3][](2);
            d.a[0][0] = 0xaa;
            d.a[0][1] = 0xbb;
            d.a[0][2] = 0xcc;
            d.a[1][0] = 0xdd;
            d.a[1][1] = 0xee;
            d.a[1][2] = 0xff;
            d.b = "\x11\x22\x33\x44\x55";
            return abi.encode(d);
        }
    ```
    */
    #[derive(Serialize, Debug)]
    struct StaticInFixedarrayInDynarray {
        #[serde(with = "as_dyn_array")]
        a: [[u64; 3]; 2],
        #[serde(with = "as_bytes")]
        b: [u8; 5],
    }

    let d = StaticInFixedarrayInDynarray {
        a: [[0xaa, 0xbb, 0xcc], [0xdd, 0xee, 0xff]],
        b: [0x11, 0x22, 0x33, 0x44, 0x55],
    };

    let expected = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset            H dynamic_struct
    0000000000000000000000000000000000000000000000000000000000000040 // d.a offset      T   H field/seq
    0000000000000000000000000000000000000000000000000000000000000120 // d.b offset      T   H field/bytes
        0000000000000000000000000000000000000000000000000000000000000002 // d.a length  T   T field/seq
        00000000000000000000000000000000000000000000000000000000000000aa // d.a[0][0]   T       H ./elem/u64
        00000000000000000000000000000000000000000000000000000000000000bb // d.a[0][1]   T       H      ./u64
        00000000000000000000000000000000000000000000000000000000000000cc // d.a[0][2]   T       H      ./u64
        00000000000000000000000000000000000000000000000000000000000000dd // d.a[1][0]   T       H ./elem/u64
        00000000000000000000000000000000000000000000000000000000000000ee // d.a[1][1]   T       H      ./u64
        00000000000000000000000000000000000000000000000000000000000000ff // d.a[1][2]   T       H      ./u64
        0000000000000000000000000000000000000000000000000000000000000005 // d.b length  T   T field/bytes
        1122334455000000000000000000000000000000000000000000000000000000 // d.b         T   T           .
    ";
    serialize_and_compare(&d, expected);
}

#[test]
fn fixed_in_dyn_container() {
    /*
    ```solidity
        struct StaticInFixedArrayInDynArrayData {
            uint64[3][] a;
            bytes b;
        }
        struct StaticInFixedArrayInDynArrayContainerData {
            uint64 x;
            DynArrayOfFixedSizeStaticArraysData y;
            uint64 z;
        }
        function StaticInFixedArrayInDynArrayContainer() public pure returns(bytes memory) {
            StaticInFixedArrayInDynArrayContainerData memory d;
            d.x = 0x1111;
            d.y.a = new uint64[3][](2);
            d.y.a[0][0] = 0xaa;
            d.y.a[0][1] = 0xbb;
            d.y.a[0][2] = 0xcc;
            d.y.a[1][0] = 0xdd;
            d.y.a[1][1] = 0xee;
            d.y.a[1][2] = 0xff;
            d.y.b = "\x11\x22\x33\x44\x55";
            d.z = 0x2222;
            return abi.encode(d);
        }
    ```
    */

    #[derive(Serialize, Debug)]
    struct StaticInFixedArrayInDynArray {
        #[serde(with = "as_dyn_array")]
        a: [[u64; 3]; 2],
        #[serde(with = "as_bytes")]
        b: [u8; 5],
    }

    let y = StaticInFixedArrayInDynArray {
        a: [[0xaa, 0xbb, 0xcc], [0xdd, 0xee, 0xff]],
        b: [0x11, 0x22, 0x33, 0x44, 0x55],
    };
    let d = Container {
        x: 0x1111,
        y,
        z: 0x2222,
    };

    let expected = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    0000000000000000000000000000000000000000000000000000000000001111 // d.x
    0000000000000000000000000000000000000000000000000000000000000060 // d.y offset
    0000000000000000000000000000000000000000000000000000000000002222 // d.z
        0000000000000000000000000000000000000000000000000000000000000040 // d.y.a offset
        0000000000000000000000000000000000000000000000000000000000000120 // d.y.b offset
            0000000000000000000000000000000000000000000000000000000000000002 // d.y.a length
            00000000000000000000000000000000000000000000000000000000000000aa // d.y.a[0][0]
            00000000000000000000000000000000000000000000000000000000000000bb // d.y.a[0][1]
            00000000000000000000000000000000000000000000000000000000000000cc // d.y.a[0][2]
            00000000000000000000000000000000000000000000000000000000000000dd // d.y.a[1][0]
            00000000000000000000000000000000000000000000000000000000000000ee // d.y.a[1][1]
            00000000000000000000000000000000000000000000000000000000000000ff // d.y.a[1][2]
            0000000000000000000000000000000000000000000000000000000000000005 // d.y.b length
            1122334455000000000000000000000000000000000000000000000000000000 // d.b
    ";
    serialize_and_compare(&d, expected);
}

mod dyn_in_fixed {
    use super::*;
    fn run<T>(d: T)
    where
        T: Serialize,
    {
        /*
        ```solidity
            struct StaticInDynarrayInFixedarrayData {
                uint64[][2] a;
                bytes b;
            }
            function StaticInDynarrayInFixedarray() public pure returns(bytes memory) {
                StaticInDynarrayInFixedarrayData memory d;
                d.a[0] = new uint64[](3);
                d.a[0][0] = 0xaa;
                d.a[0][1] = 0xbb;
                d.a[0][2] = 0xcc;
                d.a[1] = new uint64[](3);
                d.a[1][0] = 0xdd;
                d.a[1][1] = 0xee;
                d.a[1][2] = 0xff;
                d.b = "\x11\x22\x33\x44\x55";
                return abi.encode(d);
            }
        ```
        */
        let expected: &str = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    0000000000000000000000000000000000000000000000000000000000000040 // d.a offset
    0000000000000000000000000000000000000000000000000000000000000180 // d.b offset
        0000000000000000000000000000000000000000000000000000000000000040 // d.a[0] offset
        00000000000000000000000000000000000000000000000000000000000000c0 // d.a[1] offset
            0000000000000000000000000000000000000000000000000000000000000003 // d.a[0] length
            00000000000000000000000000000000000000000000000000000000000000aa // d.a[0][0]
            00000000000000000000000000000000000000000000000000000000000000bb // d.a[0][1]
            00000000000000000000000000000000000000000000000000000000000000cc // d.a[0][2]

            0000000000000000000000000000000000000000000000000000000000000003 // d.a[1] length
            00000000000000000000000000000000000000000000000000000000000000dd // d.a[1][0]
            00000000000000000000000000000000000000000000000000000000000000ee // d.a[1][1]
            00000000000000000000000000000000000000000000000000000000000000ff // d.a[1][2]

        0000000000000000000000000000000000000000000000000000000000000005 // d.b length
        1122334455000000000000000000000000000000000000000000000000000000 // d.b
    ";
        serialize_and_compare(&d, expected);
    }

    #[test]
    fn normal() {
        // We need a separate type for Inner because we otherwise cannot apply the
        // `as_dyn_array` attribute. This type/struct does not exist in the solidity
        // code and thus must be marked with `#[serde(transparent)]`, otherwise it
        // will introduce an additional offset slot if the type is dynamic as is the
        // case here.
        #[derive(Serialize, Debug)]
        #[serde(transparent)]
        struct Inner {
            #[serde(with = "as_dyn_array")]
            a: [u64; 3],
        }
        #[derive(Serialize, Debug)]
        struct StaticInFixedarrayInDynarray {
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticInFixedarrayInDynarray {
            a: [
                Inner {
                    a: [0xaa, 0xbb, 0xcc],
                },
                Inner {
                    a: [0xdd, 0xee, 0xff],
                },
            ],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }

    #[test]
    fn tuple() {
        // We need a separate type for Inner because we otherwise cannot apply the
        // `as_dyn_array` attribute. This type/struct does not exist in the solidity
        // code and thus must be marked with `#[serde(transparent)]`, otherwise it
        // will introduce an additional offset slot if the type is dynamic as is the
        // case here.
        #[derive(Serialize, Debug)]
        #[serde(transparent)]
        struct Inner(#[serde(with = "as_dyn_array")] [u64; 3]);

        #[derive(Serialize, Debug)]
        struct StaticInFixedarrayInDynarray {
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticInFixedarrayInDynarray {
            a: [Inner([0xaa, 0xbb, 0xcc]), Inner([0xdd, 0xee, 0xff])],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }
}

mod dyn_in_dyn {
    use super::*;
    fn run<T>(d: T)
    where
        T: Serialize,
    {
        /*
        ```solidity
            struct StaticInDynarrayInDynarrayData {
                uint64[][] a;
                bytes b;
            }
            function StaticInDynarrayInDynarray() public pure returns(bytes memory) {
                StaticInDynarrayInDynarrayData memory d;
                d.a = new uint64[][](2);
                d.a[0] = new uint64[](3);
                d.a[0][0] = 0xaa;
                d.a[0][1] = 0xbb;
                d.a[0][2] = 0xcc;
                d.a[1] = new uint64[](3);
                d.a[1][0] = 0xdd;
                d.a[1][1] = 0xee;
                d.a[1][2] = 0xff;
                d.b = "\x11\x22\x33\x44\x55";
                return abi.encode(d);
            }
        ```
        */
        let expected: &str = "
0000000000000000000000000000000000000000000000000000000000000020 // d offset
    0000000000000000000000000000000000000000000000000000000000000040 // d.a offset
    00000000000000000000000000000000000000000000000000000000000001a0 // d.b offset
        0000000000000000000000000000000000000000000000000000000000000002 // d.a length
            0000000000000000000000000000000000000000000000000000000000000040 // d.a[0] offset
            00000000000000000000000000000000000000000000000000000000000000c0 // d.a[1] offset
                0000000000000000000000000000000000000000000000000000000000000003 // d.a[0] length
                00000000000000000000000000000000000000000000000000000000000000aa // d.a[0][0]
                00000000000000000000000000000000000000000000000000000000000000bb // d.a[0][1]
                00000000000000000000000000000000000000000000000000000000000000cc // d.a[0][2]

                0000000000000000000000000000000000000000000000000000000000000003 // d.a[1] length
                00000000000000000000000000000000000000000000000000000000000000dd // d.a[1][0]
                00000000000000000000000000000000000000000000000000000000000000ee // d.a[1][1]
                00000000000000000000000000000000000000000000000000000000000000ff // d.a[1][2]

        0000000000000000000000000000000000000000000000000000000000000005 // d.b length
        1122334455000000000000000000000000000000000000000000000000000000 // d.b
";
        serialize_and_compare(&d, expected);
    }

    #[test]
    fn normal() {
        // We need a separate type for Inner because we otherwise cannot apply the
        // `as_dyn_array` attribute. This type/struct does not exist in the solidity
        // code and thus must be marked with `#[serde(transparent)]`, otherwise it
        // will introduce an additional offset slot if the type is dynamic as is the
        // case here.
        #[derive(Serialize, Debug)]
        #[serde(transparent)]
        struct Inner {
            #[serde(with = "as_dyn_array")]
            a: [u64; 3],
        }
        #[derive(Serialize, Debug)]
        struct StaticInFixedarrayInDynarray {
            #[serde(with = "as_dyn_array")]
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticInFixedarrayInDynarray {
            a: [
                Inner {
                    a: [0xaa, 0xbb, 0xcc],
                },
                Inner {
                    a: [0xdd, 0xee, 0xff],
                },
            ],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }

    #[test]
    fn tuple() {
        // We need a separate type for Inner because we otherwise cannot apply the
        // `as_dyn_array` attribute. This type/struct does not exist in the solidity
        // code and thus must be marked with `#[serde(transparent)]`, otherwise it
        // will introduce an additional offset slot if the type is dynamic as is the
        // case here.
        #[derive(Serialize, Debug)]
        #[serde(transparent)]
        struct Inner(#[serde(with = "as_dyn_array")] [u64; 3]);

        #[derive(Serialize, Debug)]
        struct StaticInFixedarrayInDynarray {
            #[serde(with = "as_dyn_array")]
            a: [Inner; 2],
            #[serde(with = "as_bytes")]
            b: [u8; 5],
        }

        let d = StaticInFixedarrayInDynarray {
            a: [Inner([0xaa, 0xbb, 0xcc]), Inner([0xdd, 0xee, 0xff])],
            b: [0x11, 0x22, 0x33, 0x44, 0x55],
        };

        run(d);
    }
}
