use super::types::Address;
use super::*;
use uint::hex::FromHex;

#[test]
fn in_function_args() {
    /*
    ```solidity
        function Address() public pure returns(bytes memory) {
            address d = 0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5;
            return abi.encode(d);
        }
    ```
    */

    // Random address from etherscan, do not use!
    let addr = "95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5";
    let addr = Address(<[u8; 20]>::from_hex(addr).unwrap());

    let expected = "
00000000000000000000000095222290dd7278aa3ddd389cc1e1d165cc4bafe5
    ";

    serialize_and_compare_fnargs(&addr, expected)
}

#[test]
fn in_container() {
    /*
    ```solidity
        struct AddressContainerData {
            address a;
        }
        function AddressContainer() public pure returns(bytes memory) {
            AddressContainerData memory d;
            d.a = 0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5;
            return abi.encode(d);
        }
    ```
    */

    // Random address from etherscan, do not use!
    let addr = "95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5";
    let addr = Address(<[u8; 20]>::from_hex(addr).unwrap());

    #[derive(Serialize, Debug)]
    struct AddrContainer {
        a: Address,
    }

    let d = AddrContainer { a: addr };

    // Not a dynamic type => No 0x0000..0020 added in the beginning.
    let expected = "
00000000000000000000000095222290dd7278aa3ddd389cc1e1d165cc4bafe5
    ";

    serialize_and_compare(&d, expected)
}
