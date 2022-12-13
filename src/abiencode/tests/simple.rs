use super::*;

fn run<T>()
where
    T: Bytes,
    T: Serialize,
{
    /*
    ```solidity
        function Bytes() public pure returns(bytes memory) {
            bytes memory d;
            d = "\xa1\xa2\xa3\xa4";
            return abi.encode(d);
        }
    ```
    */
    let d = BytesViaNormalAttr::gen(0xa0);

    let expected = "
    0000000000000000000000000000000000000000000000000000000000000020
    0000000000000000000000000000000000000000000000000000000000000004
    a1a2a3a400000000000000000000000000000000000000000000000000000000
    ";
    serialize_and_compare(&d, expected);
}

#[test]
fn bytes_normal_attr() {
    run::<BytesViaNormalAttr>()
}
#[test]
fn bytes_tuple_attr() {
    run::<BytesViaTupleAttr>()
}

#[test]
fn u64() {
    /*
    ```solidity
        function u64() public pure returns(bytes memory) {
            uint64 d = 0x1337000012341111;
            return abi.encode(d);
        }
    ```
    */

    let d: u64 = 0x1337000012341111;

    let expected = "
    0000000000000000000000000000000000000000000000001337000012341111
    ";

    serialize_and_compare(&d, expected)
}

mod bytes_zerolen {
    use super::*;

    fn run<T>(d: T)
    where
        T: Serialize,
    {
        /*
        ```solidity
            function BytesZero() public pure returns(bytes memory) {
                bytes memory d;
                d = "";
                return abi.encode(d);
            }
        ```
        */

        let expected = "
        0000000000000000000000000000000000000000000000000000000000000020
        0000000000000000000000000000000000000000000000000000000000000000
        ";
        serialize_and_compare(&d, expected);
    }

    #[test]
    fn fixedarray() {
        #[derive(Serialize, Debug)]
        #[serde(transparent)]
        struct BytesZero {
            #[serde(with = "as_bytes")]
            value: [u8; 0],
        }
        run(BytesZero { value: [] })
    }

    #[cfg(feature = "std")]
    #[test]
    fn vector() {
        #[derive(Serialize, Debug)]
        #[serde(transparent)]
        struct BytesZero {
            #[serde(with = "as_bytes")]
            value: Vec<u8>,
        }
        run(BytesZero { value: vec![] })
    }
}
