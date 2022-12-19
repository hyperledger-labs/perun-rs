use core::fmt::Debug;

use rand::{distributions::Standard, prelude::Distribution};
use serde::Serialize;
use uint::construct_uint;

#[cfg(feature = "secp256k1")]
use secp256k1::{PublicKey, ThirtyTwoByteHash};
#[cfg(feature = "secp256k1")]
use sha3::{Digest, Keccak256};

macro_rules! impl_hex_debug {
    ($T:ident) => {
        impl Debug for $T {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("0x")?;
                for b in self.0 {
                    f.write_fmt(format_args!("{:02x}", b))?;
                }
                Ok(())
            }
        }
    };
}

macro_rules! bytesN {
    ( $T:ident, $N:literal ) => {
        #[derive(PartialEq, Copy, Clone)]
        pub struct $T(pub [u8; $N]);

        impl Serialize for $T {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_bytes(&self.0)
            }
        }

        impl Distribution<$T> for Standard {
            fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> $T {
                $T(rng.gen())
            }
        }

        impl Default for $T {
            fn default() -> Self {
                Self([0; $N])
            }
        }

        impl_hex_debug!($T);
    };
}

bytesN!(Bytes1, 1);
bytesN!(Bytes2, 2);
bytesN!(Bytes3, 3);
bytesN!(Bytes4, 4);
bytesN!(Bytes5, 5);
bytesN!(Bytes6, 6);
bytesN!(Bytes7, 7);
bytesN!(Bytes8, 8);
bytesN!(Bytes9, 9);
bytesN!(Bytes10, 10);
bytesN!(Bytes11, 11);
bytesN!(Bytes12, 12);
bytesN!(Bytes13, 13);
bytesN!(Bytes14, 14);
bytesN!(Bytes15, 15);
bytesN!(Bytes16, 16);
bytesN!(Bytes17, 17);
bytesN!(Bytes18, 18);
bytesN!(Bytes19, 19);
bytesN!(Bytes20, 20);
bytesN!(Bytes21, 21);
bytesN!(Bytes22, 22);
bytesN!(Bytes23, 23);
bytesN!(Bytes24, 24);
bytesN!(Bytes25, 25);
bytesN!(Bytes26, 26);
bytesN!(Bytes27, 27);
bytesN!(Bytes28, 28);
bytesN!(Bytes29, 29);
bytesN!(Bytes30, 30);
bytesN!(Bytes31, 31);
bytesN!(Bytes32, 32);

bytesN!(Hash, 32);

#[cfg(feature = "secp256k1")]
impl ThirtyTwoByteHash for Hash {
    fn into_32(self) -> [u8; 32] {
        self.0
    }
}

bytesN!(Signature, 65);
impl Signature {
    pub fn new(rs: &[u8; 64], v: u8) -> Self {
        let mut sig: Signature = Signature([0; 65]);
        sig.0[..64].copy_from_slice(rs);
        sig.0[64] = v;
        sig
    }
}

// We could use primitive_types:U256 or ethereum_types::U256 here, too. Both
// have the ability to serde serialize, but unfrotunately to a hex string, which
// is not what we want. We could have wrapped it in out newtype struct like we
// did with the bytes, but then we would have to use `x.0.add` instead of
// `x.add` everywhere. Since both primitive_types and ethereum_types internally
// use construct_uint and don't add much functionality it is easier to just
// create our own types for now.
//
// Alternatively we could use the serde with attribute everywhere.
construct_uint! {
    pub struct U256(4);
}

impl Serialize for U256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = [0u8; 32];
        self.to_big_endian(&mut bytes);
        serializer.serialize_bytes(&bytes)
    }
}

impl Distribution<U256> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> U256 {
        let buf: [u8; 32] = rng.gen();
        U256::from_big_endian(&buf)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct Address(pub [u8; 20]);
impl_hex_debug!(Address);

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // For some onknown reason abi encoding has addresses right aligned
        // (like uints) instead of left aligned like bytes/bytesN.
        let mut bytes = [0u8; 32];
        bytes[32 - 20..].copy_from_slice(self.0.as_slice());
        serializer.serialize_bytes(&bytes)
    }
}

#[cfg(feature = "secp256k1")]
impl From<PublicKey> for Address {
    fn from(pk: PublicKey) -> Self {
        // See https://ethereum.stackexchange.com/questions/65233/goethereum-getting-public-key-from-private-key-hex-formatting

        // Throw away the first byte, which is not part of the public key. It is
        // added by serialize_uncompressed due to the encoding used.
        let hash: [u8; 32] = Keccak256::digest(&pk.serialize_uncompressed()[1..]).into();

        let mut addr = Address([0; 20]);
        addr.0.copy_from_slice(&hash[32 - 20..]);
        addr
    }
}

impl Distribution<Address> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Address {
        Address(rng.gen())
    }
}
