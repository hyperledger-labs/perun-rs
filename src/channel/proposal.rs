//! Low-level API for the Proposal phase.
//!
//! Currently, this can only handle channels with one asset and two
//! participants. In the future we'll likely generalize it to work with
//! arbitrary channel sizes and potentially even arbitrary ways to represent the
//! data in rust (e.g. using `Vec<T>` vs no-heap `Vec<T>` vs `fixed-size<A,P>`).

use super::{
    agreed_upon::AgreedUponChannel,
    fixed_size_payment::{self, ConversionError},
    NonceShare, PartID,
};
use crate::{
    abiencode::{
        self,
        types::{Address, Bytes32, Hash, U256},
    },
    perunwire,
    wire::{MessageBus, ParticipantMessage},
    PerunClient,
};
use sha3::{Digest, Sha3_256};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type Allocation = fixed_size_payment::Allocation<ASSETS, PARTICIPANTS>;
type Balances = fixed_size_payment::Balances<ASSETS, PARTICIPANTS>;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

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

impl TryFrom<perunwire::LedgerChannelProposalMsg> for LedgerChannelProposal {
    type Error = ConversionError;

    fn try_from(value: perunwire::LedgerChannelProposalMsg) -> Result<Self, Self::Error> {
        let base = match value.base_channel_proposal {
            Some(v) => v,
            None => return Err(ConversionError::ExptectedSome),
        };
        let init_bals = match base.init_bals {
            Some(v) => v,
            None => return Err(ConversionError::ExptectedSome),
        };
        let funding_agreement = match base.funding_agreement {
            Some(v) => v,
            None => return Err(ConversionError::ExptectedSome),
        };

        Ok(LedgerChannelProposal {
            proposal_id: Hash(
                base.proposal_id
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            challenge_duration: base.challenge_duration,
            nonce_share: Bytes32(
                base.nonce_share
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            init_bals: init_bals.try_into()?,
            funding_agreement: funding_agreement.try_into()?,
            participant: Address(
                value
                    .participant
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
        })
    }
}

impl From<LedgerChannelProposal> for perunwire::LedgerChannelProposalMsg {
    fn from(value: LedgerChannelProposal) -> Self {
        Self {
            base_channel_proposal: Some(perunwire::BaseChannelProposal {
                proposal_id: value.proposal_id.0.to_vec(),
                challenge_duration: value.challenge_duration,
                nonce_share: value.nonce_share.0.to_vec(),
                app: vec![],
                init_data: vec![],
                init_bals: Some(value.init_bals.into()),
                funding_agreement: Some(value.funding_agreement.into()),
            }),
            participant: value.participant.0.to_vec(),
            peers: vec!["Alice".as_bytes().to_vec(), "Bob".as_bytes().to_vec()], // TODO: Use real data
        }
    }
}

/// Message sent when a participant accepts the proposed channel.
#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelProposalAcc {
    proposal_id: Hash,
    nonce_share: NonceShare,
    participant: Address,
}

impl TryFrom<perunwire::LedgerChannelProposalAccMsg> for LedgerChannelProposalAcc {
    type Error = ConversionError;

    fn try_from(value: perunwire::LedgerChannelProposalAccMsg) -> Result<Self, Self::Error> {
        let base = value
            .base_channel_proposal_acc
            .ok_or(ConversionError::ExptectedSome)?;

        Ok(Self {
            proposal_id: Hash(
                base.proposal_id
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            nonce_share: Bytes32(
                base.nonce_share
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            participant: Address(
                value
                    .participant
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
        })
    }
}

impl From<LedgerChannelProposalAcc> for perunwire::LedgerChannelProposalAccMsg {
    fn from(value: LedgerChannelProposalAcc) -> Self {
        Self {
            base_channel_proposal_acc: Some(perunwire::BaseChannelProposalAcc {
                proposal_id: value.proposal_id.0.to_vec(),
                nonce_share: value.nonce_share.0.to_vec(),
            }),
            participant: value.participant.0.to_vec(),
        }
    }
}

/// Error returned when the proposal was already accepted by a participant.
#[derive(Debug)]
pub struct AlreadyAcceptedError;

#[derive(Debug)]
pub enum HandleAcceptError {
    InvalidProposalID,
    AlreadyAccepted,
}

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
            proposal_id: self.proposal.proposal_id,
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
    ) -> Result<(), HandleAcceptError> {
        if msg.proposal_id != self.proposal.proposal_id {
            return Err(HandleAcceptError::InvalidProposalID);
        }

        let index = part_id - 1;
        match self.responses[index] {
            Some(_) => Err(HandleAcceptError::AlreadyAccepted),
            None => {
                self.responses[index] = Some(msg);
                Ok(())
            }
        }
    }

    /// Progress to the next phase: Signing the initial state.
    pub fn build(self) -> Result<AgreedUponChannel<'a, B>, BuildError> {
        let mut participants = [Address::default(); PARTICIPANTS];
        participants[0] = self.proposal.participant;

        // Go-Perun does NOT use keccak256 here, probably to be less dependent
        // on Ethereum. We do the same here.
        let mut hasher = Sha3_256::new();
        hasher.update(self.proposal.nonce_share.0);

        // Go through all responses and make sure none is missing. Additionally
        // collect information needed later.
        //
        // Call combining it into a single loop premature optimization if you
        // want, but I didn't like that two loops either required to call
        // `unwrap()` or `unwrap_unchecked()`, while not knowing if the compiler
        // combines it into one loop. Nor did I find a good way to do this
        // purely with iterators. This might even be more readable than two
        // loops using `unwrap()` or `unwrap_unchecked()`, where you have to
        // argue why it is save to do so. (I didn't want to introduce another
        // intermediate representation array, which I don't know if the compiler
        // would optimize away).
        for (index, res) in self.responses.iter().enumerate() {
            // Unwrap all responses, returning an error if one is missing
            let res = res.ok_or(BuildError::MissingAccResponse(index + 1))?;

            // Store in new participants list that doesn't use options and
            // combine the nonces
            participants[index + 1] = res.participant;
            hasher.update(res.nonce_share.0);
        }

        // Finalize the nonce.
        let nonce = U256::from_big_endian(hasher.finalize().as_slice());

        // Create the initial state
        let params: Params = Params {
            challenge_duration: self.proposal.challenge_duration,
            nonce: nonce,
            participants,
            app: Address([0u8; 20]),
            ledger_channel: true,
            virtual_channel: false,
        };
        let init_state = State::new(params, self.proposal.init_bals)?;

        Ok(AgreedUponChannel::new(
            self.client,
            self.proposal.funding_agreement,
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
