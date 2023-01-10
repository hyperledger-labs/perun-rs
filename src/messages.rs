mod funding_request;
mod update;
mod watch_request;

pub use funding_request::LedgerChannelFundingRequest;
pub use update::LedgerChannelUpdateAccepted;
pub use watch_request::LedgerChannelWatchRequest;

use core::fmt::Debug;

use crate::{
    abiencode::types::Hash,
    channel::{
        LedgerChannelProposal, LedgerChannelProposalAcc, LedgerChannelUpdate,
        LedgerChannelWatchUpdate,
    },
};
use alloc::string::String;

#[derive(Debug)]
pub enum ConversionError {
    ParticipantSizeMissmatch,
    AssetSizeMissmatch,
    ByteLengthMissmatch,
    ExptectedSome,
    StateChannelsNotSupported,
}

/// Messages sent to the Watcher service.
#[derive(Debug)]
pub enum WatcherRequestMessage {
    /// Ask the Watcher to start watching the blockchain for disputes.
    /// Acknowledged with [WatcherMessage::Ack] containing `version == 0`.
    WatchRequest(LedgerChannelWatchRequest),
    /// Notify the Watcher of a new state. This could be combined with
    /// [WatcherMessage::WatchRequest], the only difference is that
    /// [WatcherMessage::Update] does not necessary need the parameters.
    /// Acknowledged with [WatcherMessage::Ack].
    Update(LedgerChannelWatchUpdate),
    /// Ask the Watcher to initialize a dispute on-chain, with the given state.
    /// It currently does not contain the parameters for reducing the amount of
    /// communication needed. Adding it might be useful to make the watcher less
    /// stateful.
    StartDispute(LedgerChannelWatchUpdate),
}

/// Messages sent from the Watcher service.
#[derive(Debug)]
pub enum WatcherReplyMessage {
    /// Reply from the Watcher that a state has been received and will be used
    /// in a dispute case.
    Ack { id: Hash, version: u64 },
    /// Ask the Watcher to initialize a dispute on-chain, with the given state.
    /// It currently does not contain the parameters for reducing the amount of
    /// communication needed. Adding it might be useful to make the watcher less
    /// stateful.
    DisputeAck { id: Hash },
    /// Used by the Watcher to notify the device of the existence of an on-chain
    /// dispute. This way the device knows that it does not/should not continue
    /// updating the channel.
    DisputeNotification { id: Hash },
}

/// Messages sent to the Funder service.
#[derive(Debug)]
pub enum FunderRequestMessage {
    FundingRequest(LedgerChannelFundingRequest),
}

/// Messages sent from the Funder service.
#[derive(Debug)]
pub enum FunderReplyMessage {
    Funded { id: Hash },
}

/// Messages sent between participants of a channel.
#[derive(Debug)]
pub enum ParticipantMessage {
    Auth,
    ChannelProposal(LedgerChannelProposal),
    ProposalAccepted(LedgerChannelProposalAcc),
    ProposalRejected {
        id: Hash,
        reason: String,
    },
    ChannelUpdate(LedgerChannelUpdate),
    ChannelUpdateAccepted(LedgerChannelUpdateAccepted),
    ChannelUpdateRejected {
        id: Hash,
        version: u64,
        reason: String,
    },
}
