use core::fmt::Debug;

use crate::{
    abiencode::types::Hash,
    channel::{
        LedgerChannelFundingRequest, LedgerChannelProposal, LedgerChannelProposalAcc,
        LedgerChannelUpdateAccepted, LedgerChannelWatchRequest,
    },
};

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
    WatchRequest(LedgerChannelWatchRequest),
    WatchRequestAck { id: Hash },
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
    ChannelProposal(LedgerChannelProposal),
    ProposalAccepted(LedgerChannelProposalAcc),
    ProposalRejected,
    ChannelUpdateAccepted(LedgerChannelUpdateAccepted),
}
