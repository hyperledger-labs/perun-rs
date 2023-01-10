use crate::{
    abiencode::{self, as_bytes},
    Hash,
};
use rand::{rngs::StdRng, SeedableRng};
use serde::Serialize;
use uint::hex::ToHex;

fn data() -> Hash {
    /*
    ```solidity
    function verify_sig(address signer, bytes memory sig) public pure {
        bytes memory d;
        d = "\xa1\xa2\xa3\xa4";
        require(Sig.verify(abi.encode(d), sig, signer), "invalid signature");
    }
    ```
    */

    #[derive(Serialize, Debug)]
    #[serde(transparent)]
    struct Bytes {
        #[serde(with = "as_bytes")]
        value: [u8; 4],
    }
    let d = Bytes {
        value: [0xa1, 0xa2, 0xa3, 0xa4],
    };

    abiencode::to_hash(&d).unwrap()
}

macro_rules! make_compare_hardcoded {
    ($name:ident, $signer:ty, $address:literal, $expected_sig:literal) => {
        #[test]
        fn $name() {
            // This test may break in the future (e.g. if the dependency changes
            // internally), they exist to allow checking if a signature is valid
            // on-chain without needing a real blockchain or ganache.

            // Do not use that on any real device, this is just for testing.
            let mut rng = StdRng::seed_from_u64(0);
            let signer = <$signer>::new(&mut rng);
            let sig = signer.sign_eth(data());

            println!("Address: {}", signer.address().0.encode_hex::<String>());
            println!("Sig: 0x{}", sig.0.encode_hex::<String>());

            let address = $address;
            let expected_sig = $expected_sig;

            // Sanity checks for macro user input
            assert_eq!(address.len(), 20 * 2);
            assert_eq!(expected_sig.len(), 2 + 65 * 2);

            assert_eq!(signer.address().0.encode_hex::<String>(), address);
            assert_eq!(sig.0.encode_hex::<String>(), &expected_sig[2..]);
        }
    };
}

macro_rules! make_a_to_b {
    ($name:ident, $signer:ty, $verifier:ty) => {
        #[test]
        fn $name() {
            // Do not use that on any real device, this is just for testing.
            let mut rng = StdRng::seed_from_u64(0);
            let signer = <$signer>::new(&mut rng);
            let msg = data();
            let sig = signer.sign_eth(msg);

            println!("Address: {}", signer.address().0.encode_hex::<String>());
            println!("Sig: 0x{}", sig.0.encode_hex::<String>());

            let verifier = <$verifier>::new(&mut rng);
            let address = verifier.recover_signer(msg, sig).unwrap();

            assert_eq!(address, signer.address());
        }
    };
}

// Note that the outputs of the following do not necessary have to be equal,
// they may depend on how exactly each library uses the random number generator.
#[cfg(feature = "secp256k1")]
make_compare_hardcoded!(
    secp256k1_sign,
    super::secp256k1::Signer,
    "a9572220348b1080264e81c0779f77c144790cd6",
    "0xdb101ce5201d7a04b67bdfe5c50b910524c62c0900c85997fd187a7b4e56aa990f96e031be10092befa49e713218cb14c75a6f6a5aa4699d969f4348f873f0151b"
);

#[cfg(feature = "k256")]
make_compare_hardcoded!(
    k256_sign,
    super::k256::Signer,
    "a9572220348b1080264e81c0779f77c144790cd6",
    "0xdb101ce5201d7a04b67bdfe5c50b910524c62c0900c85997fd187a7b4e56aa990f96e031be10092befa49e713218cb14c75a6f6a5aa4699d969f4348f873f0151b"
);

#[cfg(feature = "secp256k1")]
make_a_to_b!(
    secp256k1_to_secp256k1,
    super::secp256k1::Signer,
    super::secp256k1::Signer
);

#[cfg(feature = "k256")]
make_a_to_b!(k256_to_k256, super::k256::Signer, super::k256::Signer);

#[cfg(all(feature = "secp256k1", feature = "k256"))]
make_a_to_b!(
    secp256k1_to_k256,
    super::secp256k1::Signer,
    super::k256::Signer
);

#[cfg(all(feature = "secp256k1", feature = "k256"))]
make_a_to_b!(
    k256_to_secp256k1,
    super::k256::Signer,
    super::secp256k1::Signer
);

// #[cfg(feature = "secp256k1")]
// fn secp256k1_sign() {
//     // This test may break in the future (e.g. if the dependency changes
//     // internally), they exist to allow checking if a signature is valid
//     // on-chain without needing a real blockchain or ganache.

//     // Do not use that on any real device, this is just for testing.
//     let mut rng = StdRng::seed_from_u64(0);
//     let signer = super::secp256k1::Signer::new(&mut rng);

//     let sig = signer.sign_eth(data());

//     assert_eq!(signer.address().0.encode_hex::<String>(), "");
//     assert_eq!(sig.0.encode_hex::<String>(), ""[2..]);
// }

// #[cfg(feature = "k256")]
// fn k256_sign() {
//     // This test may break in the future (e.g. if the dependency changes
//     // internally), they exist to allow checking if a signature is valid
//     // on-chain without needing a real blockchain or ganache.

//     // Do not use that on any real device, this is just for testing.
//     let mut rng = StdRng::seed_from_u64(0);
//     let signer = super::k256::Signer::new(&mut rng);

//     let sig = signer.sign_eth(data());

//     assert_eq!(signer.address().0.encode_hex::<String>(), "");
//     assert_eq!(sig.0.encode_hex::<String>(), ""[2..]);
// }
