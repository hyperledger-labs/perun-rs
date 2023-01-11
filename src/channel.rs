mod active;
mod agreed_upon;
mod channel_update;
pub mod fixed_size_payment;
mod proposal;
mod signed;

use crate::abiencode::types::{Address, Bytes32, U256};
use serde::Serialize;

// TODO: Re-export all other types, too
pub use channel_update::ChannelUpdate;

// Re-exported within this crate because the PerunClient has to be able to
// create a new ProposedChannel, but the proposal module is private.
pub(crate) use proposal::ProposedChannel;

// Re-exported because it is part of the low-level channel API
pub use crate::messages::LedgerChannelProposal;

/// ID (Index) of a participant in the channel.
///
/// `0` is the proposer of the channel.
pub type PartID = usize;

/// The nonce added by each participant.
///
/// They are combined into a single [U256] using SHA256.
pub type NonceShare = Bytes32;

/// Uniquely identifies an Asset by blockchain + AssetHolder.
#[derive(Serialize, Debug, Copy, Clone, Default)]
pub struct Asset {
    pub chain_id: U256,
    pub holder: Address,
}
