//! Handles the creation and verification of (Ethereum) Signatures.

use crate::abiencode::types::Hash;
use sha3::{Digest, Keccak256};

#[cfg(feature = "secp256k1")]
mod secp256k1;
#[cfg(feature = "secp256k1")]
pub use self::secp256k1::{Error, Signer};

#[cfg(any(not(feature = "secp256k1"), doc))]
mod dummy;
#[cfg(not(feature = "secp256k1"))]
pub use self::dummy::{Error, Signer};

/// Add the `\x19Ethereum Signed Message\n<length>` prefix to hash.
///
/// This is the format expected by the Solidity contracts.
fn hash_to_eth_signed_msg_hash(hash: Hash) -> Hash {
    // Packed encoding => We can't use the serializer
    let mut hasher = Keccak256::new();
    hasher.update(b"\x19Ethereum Signed Message:\n32");
    hasher.update(hash.0);
    Hash(hasher.finalize().into())
}
