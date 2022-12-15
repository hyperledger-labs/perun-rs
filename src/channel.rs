mod agreed_upon;
pub mod fixed_size_payment;
mod proposal;

use crate::abiencode::types::{Address, Bytes32, U256};
pub use agreed_upon::LedgerChannelUpdateAccepted;
use core::fmt::Debug;
pub use proposal::{LedgerChannelProposal, LedgerChannelProposalAcc, ProposedChannel};
use serde::Serialize;

/// ID (Index) of a participant in the channel.
///
/// `0` is the proposer of the channel.
pub type PartID = usize;

/// The nonce added by each participant.
///
/// They are combined into a single [U256] using SHA256.
pub type NonceShare = Bytes32;

/// Uniquely identifies an Asset by blockchain + AssetHolder.
#[derive(Serialize, Debug, Copy, Clone)]
pub struct Asset {
    pub chain_id: U256,
    pub holder: Address,
}
