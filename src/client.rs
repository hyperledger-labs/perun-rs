use crate::channel::ProposedChannel;
use crate::messages::{LedgerChannelProposal, ParticipantMessage};
use crate::sig::Signer;
use crate::wire::{BroadcastMessageBus, Identity, MessageBus};
use crate::Address;
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

    pub fn send_handshake_msg(&self, sender: &Identity, recipient: &Identity) {
        self.bus
            .send_to_participant(sender, recipient, ParticipantMessage::Auth);
    }

    /// Propose a new channel with the given parameters/proposal and send a
    /// message to all participants.
    pub fn propose_channel(
        &self,
        prop: LedgerChannelProposal,
        withdraw_receiver: Address,
    ) -> ProposedChannel<B> {
        // ProposedChannel::new cannot fail (panic or return an Error).
        // Therefore it does not make a difference weather we first create the
        // return object or first send out the messages (which currently use a
        // reference to avoid copying the Identifier too often, though that may
        // change in the future). This means we can save on a call to
        // prop.clone() by using the proposal for the return value as reference
        // for the peers needed to broadcast, which we would have had to clone
        // if we couldn't change the order of the lines below.
        //
        // Alternatively we could have added a second clone, add a livetime to
        // ParticipantMessage, change broadcast_to_participants to not use a
        // reference (which also requires a second clone), or read the proposal
        // back from the ProposedChannel.
        let msg = ParticipantMessage::ChannelProposal(prop.clone());
        self.bus.broadcast_to_participants(0, &prop.peers, msg);
        ProposedChannel::new(self, 0, withdraw_receiver, prop)
    }

    /// Call this when receiving a proposal message, then call `accept()` or
    /// `reject()` to send the response.
    pub fn handle_proposal(
        &self,
        prop: LedgerChannelProposal,
        withdraw_receiver: Address,
    ) -> ProposedChannel<B> {
        ProposedChannel::new(self, 1, withdraw_receiver, prop)
    }
}
