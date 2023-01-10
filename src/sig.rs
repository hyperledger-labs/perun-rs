//! Handles the creation and verification of (Ethereum) Signatures.
//!
//! The Modules (and their respective dependency) can be enabled/disabled with
//! the equally named feature flags. [Error][k256::Error] and
//! [Signer][k256::Signer] are re-exported from the selected Module. If both
//! feature flags are present, [secp256k1] is used because [k256] is marked as
//! the default in cargo.toml.

use crate::abiencode::types::Hash;
use sha3::{Digest, Keccak256};

#[cfg(test)]
#[cfg(feature = "std")]
mod tests;

// Import the requested implementation(s), as well as the dummy fallback to make
// sure it always compiles, too, even if the feature flags are set.
#[doc(hidden)]
mod dummy;
#[cfg(feature = "k256")]
#[cfg_attr(docsrs, doc(cfg(feature = "k256")))]
pub mod k256;
#[cfg(feature = "secp256k1")]
#[cfg_attr(docsrs, doc(cfg(feature = "secp256k1")))]
pub mod secp256k1;

// Complain if no signing implementation is set, while hiding all the errors
// resulting from that by using the dummy implementation.
#[cfg(not(any(feature = "secp256k1", feature = "k256")))]
compile_error!(
    "Signature dependency needed, use one of the following feature flags: 'secp256k1', 'k256'"
);
#[cfg(not(any(feature = "secp256k1", feature = "k256")))]
pub use self::dummy::{Error, Signer};

// Only use k256 (part of default) if the secp256k1 feature flag is not set. The
// application may enable both feature flags, this logic chooses secp256k1 in
// this case (thus ignoring k256 which is enabled by default).
#[cfg(all(not(feature = "secp256k1"), feature = "k256"))]
pub use self::k256::{Error, Signer};
#[cfg(feature = "secp256k1")]
#[doc(hidden)]
pub use self::secp256k1::{Error, Signer};

/// Helper function for the Signers.
///
/// Add the `\x19Ethereum Signed Message\n<length>` prefix to hash. This is the
/// format expected by the Solidity contracts.
fn hash_to_eth_signed_msg_hash(hash: Hash) -> Hash {
    // Packed encoding => We can't use the serializer
    let mut hasher = Keccak256::new();
    hasher.update(b"\x19Ethereum Signed Message:\n32");
    hasher.update(hash.0);
    Hash(hasher.finalize().into())
}
