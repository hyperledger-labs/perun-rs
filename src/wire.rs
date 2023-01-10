mod encoding;

use core::fmt::Debug;

pub use encoding::ProtoBufEncodingLayer;

use crate::{
    abiencode::types::Hash,
    channel::{
        LedgerChannelFundingRequest, LedgerChannelProposal, LedgerChannelProposalAcc,
        LedgerChannelUpdate, LedgerChannelUpdateAccepted, LedgerChannelWatchRequest,
        LedgerChannelWatchUpdate,
    },
};
use alloc::string::String;

pub trait BytesBus: Debug {
    fn send_to_watcher(&self, msg: &[u8]);
    fn send_to_funder(&self, msg: &[u8]);
    fn send_to_participants(&self, msg: &[u8]);
}

/// Low-Level abstraction over the network configuration.
///
/// Might be moved into a byte based MessageBus or behind a `unstable` feature
/// flag.
pub trait MessageBus: Debug {
    fn send_to_watcher(&self, msg: WatcherMessage);
    fn send_to_funder(&self, msg: FunderMessage);
    fn send_to_participants(&self, msg: ParticipantMessage);
}

/// Messages sent to/from the Watcher service.
#[derive(Debug)]
pub enum WatcherMessage {
    /// Ask the Watcher to start watching the blockchain for disputes.
    /// Acknowledged with [WatcherMessage::Ack] containing `version == 0`.
    WatchRequest(LedgerChannelWatchRequest),
    /// Notify the Watcher of a new state. This could be combined with
    /// [WatcherMessage::WatchRequest], the only difference is that
    /// [WatcherMessage::Update] does not necessary need the parameters.
    /// Acknowledged with [WatcherMessage::Ack].
    Update(LedgerChannelWatchUpdate),
    /// Reply from the Watcher that a state has been received and will be used
    /// in a dispute case.
    Ack { id: Hash, version: u64 },
    /// Ask the Watcher to initialize a dispute on-chain, with the given state.
    /// It currently does not contain the parameters for reducing the amount of
    /// communication needed. Adding it might be useful to make the watcher less
    /// stateful.
    StartDispute(LedgerChannelWatchUpdate),
    /// Acknowledgement of [WatcherMessage::StartDispute]
    DisputeAck { id: Hash },
    /// Used by the Watcher to notify the device of the existence of an on-chain
    /// dispute. This way the device knows that it does not/should not continue
    /// updating the channel.
    DisputeNotification { id: Hash },
}

/// Messages sent to/from the Funder service.
#[derive(Debug)]
pub enum FunderMessage {
    FundingRequest(LedgerChannelFundingRequest),
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
