use crate::channel::{LedgerChannelProposal, ProposedChannel};
use crate::sig::Signer;
use crate::wire::{MessageBus, ParticipantMessage};
use core::fmt::Debug;

// TODO: Add all the verification code (if data is correctly formed)

/// The main Perun object used to create new channels and configure
/// communication.
///
/// It contains information on how to access the private key for signing and how
/// to send information to the watcher and funder. Usually you only need one
/// PerunClient.
///
/// Note: An application will usually have only one MessageBux type, thus using
/// dynamic dispatch here doesn't make much sense.
#[derive(Debug)]
pub struct PerunClient<B: MessageBus> {
    pub(crate) bus: B,
    pub(crate) signer: Signer,
}

impl<B: MessageBus> PerunClient<B> {
    /// Creates a new [PerunClient] with the given [MessageBus].
    pub fn new(bus: B, signer: Signer) -> Self {
        PerunClient { bus, signer }
    }

    pub fn send_handshake_msg(&self) {
        self.bus.send_to_participants(ParticipantMessage::Auth);
    }

    /// Propose a new channel with the given parameters/proposal and send a
    /// message to all participants.
    pub fn propose_channel(&self, prop: LedgerChannelProposal) -> ProposedChannel<B> {
        let c = ProposedChannel::new(self, 0, prop);
        self.bus
            .send_to_participants(ParticipantMessage::ChannelProposal(prop));
        c
    }

    /// Call this when receiving a proposal message, then call `accept()` or
    /// `reject()` to send the response.
    pub fn handle_proposal(&self, prop: LedgerChannelProposal) -> ProposedChannel<B> {
        ProposedChannel::new(self, 1, prop)
    }
}
