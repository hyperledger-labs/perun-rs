use super::{fixed_size_payment, signed::SignedChannel, PartID};
use crate::{
    abiencode::{
        self,
        types::{Address, Hash, Signature},
    },
    sig,
    wire::{FunderMessage, MessageBus, ParticipantMessage, WatcherMessage},
    PerunClient,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;
type Balances = fixed_size_payment::Balances<ASSETS, PARTICIPANTS>;

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelWatchRequest {
    pub params: Params,
    pub state: State,
    pub signatures: [Signature; PARTICIPANTS],
}

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelFundingRequest {
    pub funding_agreement: Balances,
    pub params: Params,
    pub state: State,
}

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelUpdateAccepted {
    channel: Hash,
    version: u64,
    sig: Signature,
}

#[derive(Debug)]
pub enum SignError {
    AbiEncodeError(abiencode::Error),
    AlreadySigned,
}
impl From<abiencode::Error> for SignError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}

#[derive(Debug)]
pub enum AddSignatureError {
    AbiEncodeError(abiencode::Error),
    RecoveryFailed(sig::Error),
    AlreadySigned,
    InvalidSignature(Address),
    InvalidChannelID,
    InvalidVersionNumber,
}
impl From<abiencode::Error> for AddSignatureError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}
impl From<sig::Error> for AddSignatureError {
    fn from(e: sig::Error) -> Self {
        Self::RecoveryFailed(e)
    }
}

#[derive(Debug)]
pub enum BuildError {
    MissingSignatureResponse(PartID),
}

#[derive(Debug)]
pub struct AgreedUponChannel<'a, B: MessageBus> {
    part_id: PartID,
    client: &'a PerunClient<B>,
    funding_agreement: Balances,
    init_state: State,
    params: Params,
    signatures: [Option<Signature>; 2],
}

impl<'a, B: MessageBus> AgreedUponChannel<'a, B> {
    pub(super) fn new(
        client: &'a PerunClient<B>,
        funding_agreement: Balances,
        part_id: PartID,
        init_state: State,
        params: Params,
    ) -> Self {
        AgreedUponChannel {
            part_id,
            client,
            funding_agreement,
            init_state,
            params,
            signatures: [None; PARTICIPANTS],
        }
    }

    pub fn sign(&mut self) -> Result<(), SignError> {
        match self.signatures[self.part_id] {
            Some(_) => Err(SignError::AlreadySigned),
            None => {
                // Sign the initial state
                let hash = abiencode::to_hash(&self.init_state)?;
                let sig = self.client.signer.sign_eth(hash);
                // Add signature to the proposed channel
                self.signatures[self.part_id] = Some(sig);
                // Send to other participants
                self.client
                    .bus
                    .send_to_participants(ParticipantMessage::ChannelUpdateAccepted(
                        LedgerChannelUpdateAccepted {
                            channel: self.init_state.channel_id(),
                            version: self.init_state.version(),
                            sig: sig,
                        },
                    ));
                Ok(())
            }
        }
    }

    // This function allows adding our own signature if we really want. There is
    // currently no easy way to get one, but it is possible.
    pub fn add_signature(
        &mut self,
        msg: LedgerChannelUpdateAccepted,
    ) -> Result<(), AddSignatureError> {
        if msg.channel != self.init_state.channel_id() {
            return Err(AddSignatureError::InvalidChannelID);
        }
        if msg.version != 0 {
            return Err(AddSignatureError::InvalidVersionNumber);
        }

        let hash = abiencode::to_hash(&self.init_state)?;
        let signer = self.client.signer.recover_signer(hash, msg.sig)?;

        // Verify signature is comming from a valid participant.
        let part_id: PartID = match self
            .params
            .participants
            .iter()
            .enumerate()
            .filter(|(_, &addr)| addr == signer)
            .next()
        {
            Some((part_id, _)) => part_id,
            None => return Err(AddSignatureError::InvalidSignature(signer)),
        };

        match self.signatures[part_id] {
            Some(_) => Err(AddSignatureError::AlreadySigned),
            None => {
                self.signatures[part_id] = Some(msg.sig);
                Ok(())
            }
        }
    }

    pub fn build(self) -> Result<SignedChannel<'a, B>, BuildError> {
        // Make sure we have the signature from all participants. They have
        // already been verified in `add_signature()` or we created it ourselves
        // with `sign()`. At the same time, this loop collects the signatures
        // for the next phase into an array.
        let mut signatures: [Signature; PARTICIPANTS] = [Signature::default(); PARTICIPANTS];
        for (part_id, s) in self.signatures.iter().enumerate() {
            signatures[part_id] = s.ok_or(BuildError::MissingSignatureResponse(part_id))?;
        }

        self.client
            .bus
            .send_to_watcher(WatcherMessage::WatchRequest(LedgerChannelWatchRequest {
                params: self.params,
                state: self.init_state,
                signatures: signatures,
            }));

        self.client
            .bus
            .send_to_funder(FunderMessage::FundingRequest(LedgerChannelFundingRequest {
                funding_agreement: self.funding_agreement,
                params: self.params,
                state: self.init_state,
            }));

        Ok(SignedChannel::new(
            self.client,
            self.part_id,
            self.init_state,
            self.params,
            signatures,
        ))
    }
}

impl<'a, B: MessageBus> TryFrom<AgreedUponChannel<'a, B>> for SignedChannel<'a, B> {
    type Error = BuildError;

    fn try_from(value: AgreedUponChannel<'a, B>) -> Result<Self, Self::Error> {
        value.build()
    }
}
