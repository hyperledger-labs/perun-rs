//! Low-level API for the Proposal phase.
//!
//! Currently, this can only handle channels with one asset and two
//! participants. In the future we'll likely generalize it to work with
//! arbitrary channel sizes and potentially even arbitrary ways to represent the
//! data in rust (e.g. using `Vec<T>` vs no-heap `Vec<T>` vs `fixed-size<A,P>`).

use super::{
    agreed_upon::AgreedUponChannel,
    fixed_size_payment::{self, Params, State},
    NonceShare, PartID,
};
use crate::{
    abiencode::{
        self,
        types::{Address, Hash, U256},
    },
    wire::{MessageBus, ParticipantMessage},
    PerunClient,
};
use sha3::{Digest, Sha3_256};

type Allocation = fixed_size_payment::Allocation<1, 2>;
type Balances = fixed_size_payment::Balances<1, 2>;

/// Channel configuration (also exchanged over the network)
#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelProposal {
    pub proposal_id: Hash,
    pub challenge_duration: u64,
    pub nonce_share: NonceShare,
    pub init_bals: Allocation,
    pub funding_agreement: Balances,
    pub participant: Address,
}

/// Message sent when a participant accepts the proposed channel.
#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelProposalAcc {
    nonce_share: NonceShare,
    participant: Address,
}

/// Error returned when the proposal was already accepted by a participant.
#[derive(Debug)]
pub struct AlreadyAcceptedError;

/// Error returned when the transition from ProposedChannel -> AgreedUponChannel failed.
#[derive(Debug)]
pub enum BuildError {
    AbiEncodeError(abiencode::Error),
    MissingAccResponse(PartID),
}
impl From<abiencode::Error> for BuildError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}

/// Represents a channel that was proposed, but not accepted by all
/// participants.
///
/// Use `build()` or `try_into()` to get an [AgreedUponChannel], to sign the
/// initial state and exchange those signatures.
#[derive(Debug)]
pub struct ProposedChannel<'a, B: MessageBus> {
    /// Who are we in this channel (0 is the channel proposer).
    part_id: PartID,
    /// Reference to the PerunClient, used for communication.
    client: &'a PerunClient<B>,
    /// Needed for creating the initial state, Params and for the application to
    /// decide if those are valid Parameters.
    proposal: LedgerChannelProposal,
    /// Holds all accept messages received so far.
    ///
    /// The data of Participant 0 is already stored in the proposal. We store
    /// this as an array regardless, to make future transitions to >2 Party
    /// channels easier.
    responses: [Option<LedgerChannelProposalAcc>; 1],
}

impl<'a, B: MessageBus> ProposedChannel<'a, B> {
    /// Create a new ProposedChannel.
    ///
    /// The caller ([PerunClient]) is responsible for sending the proposal
    /// message to all participants.
    pub(crate) fn new(
        client: &'a PerunClient<B>,
        part_id: PartID,
        prop: LedgerChannelProposal,
    ) -> Self {
        let c = ProposedChannel {
            part_id: part_id,
            client: client,
            proposal: prop,
            responses: [None],
        };
        c
    }

    /// Accept a proposed channel and reply to the participants.
    ///
    /// Do not call this if you have proposed the channel yourself, it will just
    /// return an Error.
    pub fn accept(
        &mut self,
        nonce_share: NonceShare,
        address: Address,
    ) -> Result<(), AlreadyAcceptedError> {
        if self.part_id == 0 || self.responses[self.part_id - 1].is_some() {
            return Err(AlreadyAcceptedError);
        }

        let acc: _ = LedgerChannelProposalAcc {
            nonce_share,
            participant: address,
        };
        self.responses[self.part_id - 1] = Some(acc);
        self.client
            .bus
            .send_to_participants(ParticipantMessage::ProposalAccepted(acc));

        Ok(())
    }

    /// Reject a proposed channel and send the reply to the participants.
    ///
    /// Drops the ProposedChannel object because using it no longer makes sense,
    /// as we have rejected the proposal.
    pub fn reject(self) {
        self.client
            .bus
            .send_to_participants(ParticipantMessage::ProposalRejected);
    }

    /// Call this when receiving an Accept response form a participant.
    ///
    /// Adds the response to the list of responses, needed to progress to the
    /// next Phase: Creating and signing the initial state.
    ///
    /// When receiving a reject message, the [ProposedChannel] object can be
    /// dropped.
    pub fn participant_accepted(
        &mut self,
        part_id: PartID,
        msg: LedgerChannelProposalAcc,
    ) -> Result<(), AlreadyAcceptedError> {
        let index = part_id - 1;
        match self.responses[index] {
            Some(_) => Err(AlreadyAcceptedError),
            None => {
                self.responses[index] = Some(msg);
                Ok(())
            }
        }
    }

    /// Progress to the next phase: Signing the initial state.
    pub fn build(self) -> Result<AgreedUponChannel<'a, B>, BuildError> {
        // Make sure we have responses from all participants
        for (index, res) in self.responses.iter().enumerate() {
            if res.is_none() {
                return Err(BuildError::MissingAccResponse(index + 1));
            }
        }

        // Go-Perun does NOT use keccak256 here, probably to be less dependent
        // on Ethereum. We do the same here.
        let mut hasher = Sha3_256::new();
        for response in self.responses {
            hasher.update(response.unwrap().nonce_share.0);
        }
        let nonce = U256::from_big_endian(hasher.finalize().as_slice());

        // Create the initial state
        let params: Params<2> = Params {
            challenge_duration: self.proposal.challenge_duration,
            nonce: nonce,
            participants: [
                self.proposal.participant,
                self.responses[0].unwrap().participant,
            ],
            app: [],
            ledger_channel: true,
            virtual_channel: false,
        };
        let init_state = State::new(params, self.proposal.init_bals)?;

        Ok(AgreedUponChannel::new(
            self.client,
            self.part_id,
            init_state,
            params,
        ))
    }
}

impl<'a, B: MessageBus> TryFrom<ProposedChannel<'a, B>> for AgreedUponChannel<'a, B> {
    type Error = BuildError;

    fn try_from(value: ProposedChannel<'a, B>) -> Result<Self, Self::Error> {
        value.build()
    }
}
