use core::fmt::Debug;

use crate::channel::{LedgerChannelProposal, LedgerChannelProposalAcc};

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
pub enum WatcherMessage {}

/// Messages sent to/from the Funder service.
#[derive(Debug)]
pub enum FunderMessage {
    // params, (init_state), init_alloc
}

/// Messages sent between participants of a channel.
#[derive(Debug)]
pub enum ParticipantMessage {
    ChannelProposal(LedgerChannelProposal),
    ProposalAccepted(LedgerChannelProposalAcc),
    ProposalRejected,
}
