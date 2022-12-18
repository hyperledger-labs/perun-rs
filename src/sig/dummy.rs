//! Dummy Signer that always panics. Fallback if no signer feature flag is
//! selected.
//!
//! Used during development to avoid compiler errors while no no_std compatible
//! signing library is added.

// TODO: Remove this once we've added k256 for no_std.

use crate::abiencode::types::{Address, Hash, Signature};

#[derive(Debug)]
pub struct Error {}

#[derive(Debug)]
pub struct Signer {}

impl Signer {
    pub fn sign_eth(&self, _msg: Hash) -> Signature {
        unimplemented!()
    }

    pub fn recover_signer(&self, _hash: Hash, _eth_sig: Signature) -> Result<Address, Error> {
        unimplemented!()
    }
}
