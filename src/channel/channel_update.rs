use super::{
    active::ActiveChannel, agreed_upon::AddSignatureError, fixed_size_payment, PartIdx, SignError,
};
use crate::{
    abiencode::{self, types::Signature},
    messages::{LedgerChannelUpdateAccepted, ParticipantMessage},
    wire::{BroadcastMessageBus, MessageBus},
};
use alloc::string::ToString;

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;

/// Error returned when the proposal was already accepted by a participant.
#[derive(Debug)]
pub enum AcceptError {
    AbiEncodeError(abiencode::Error),
    AlreadyAccepted,
}
impl From<abiencode::Error> for AcceptError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}

#[derive(Debug)]
pub enum ApplyError {
    MissingSignature(PartIdx),
    SignError(SignError),
}
impl From<SignError> for ApplyError {
    fn from(e: SignError) -> Self {
        Self::SignError(e)
    }
}

#[derive(Debug)]
pub struct ChannelUpdate<'cl, 'ch, B: MessageBus> {
    channel: &'ch mut ActiveChannel<'cl, B>,
    new_state: State,
    signatures: [Option<Signature>; PARTICIPANTS],
}

impl<'cl, 'ch, B: MessageBus> ChannelUpdate<'cl, 'ch, B> {
    pub(crate) fn new(
        channel: &'ch mut ActiveChannel<'cl, B>,
        new_state: State,
        sig_part_idx: PartIdx,
        sig: Signature,
    ) -> Self {
        let mut signatures = [None; PARTICIPANTS];
        signatures[sig_part_idx] = Some(sig);
        ChannelUpdate {
            channel,
            new_state,
            signatures,
        }
    }

    pub fn accept(&mut self) -> Result<(), AcceptError> {
        match self.signatures[self.channel.part_idx()] {
            Some(_) => Err(AcceptError::AlreadyAccepted),
            None => {
                let hash = abiencode::to_hash(&self.new_state)?;
                let sig = self.channel.client().signer.sign_eth(hash);

                let acc: _ = LedgerChannelUpdateAccepted {
                    channel: self.channel.channel_id(),
                    version: self.new_state.version(),
                    sig,
                };
                self.signatures[self.channel.part_idx()] = Some(sig);
                self.channel.client().bus.broadcast_to_participants(
                    self.channel.part_idx(),
                    self.channel.peers(),
                    ParticipantMessage::ChannelUpdateAccepted(acc),
                );
                Ok(())
            }
        }
    }

    pub fn reject(self, reason: &str) {
        self.channel.client().bus.broadcast_to_participants(
            self.channel.part_idx(),
            self.channel.peers(),
            ParticipantMessage::ChannelUpdateRejected {
                id: self.channel.channel_id(),
                version: self.new_state.version(),
                reason: reason.to_string(),
            },
        );
    }

    pub fn participant_accepted(
        &mut self,
        part_idx: PartIdx,
        msg: LedgerChannelUpdateAccepted,
    ) -> Result<(), AddSignatureError> {
        if msg.channel != self.channel.channel_id() {
            return Err(AddSignatureError::InvalidChannelID);
        }
        if msg.version != self.new_state.version() {
            return Err(AddSignatureError::InvalidVersionNumber);
        }

        let hash = abiencode::to_hash(&self.new_state)?;
        let signer = self.channel.client().signer.recover_signer(hash, msg.sig)?;

        if self.channel.params().participants[part_idx] != signer {
            return Err(AddSignatureError::InvalidSignature(signer));
        }

        match self.signatures[part_idx] {
            Some(_) => Err(AddSignatureError::AlreadySigned),
            None => {
                self.signatures[part_idx] = Some(msg.sig);
                Ok(())
            }
        }
    }

    fn signatures(&self) -> Result<[Signature; PARTICIPANTS], ApplyError> {
        let mut signatures: [Signature; PARTICIPANTS] = [Signature::default(); PARTICIPANTS];
        for (part_idx, s) in self.signatures.iter().enumerate() {
            signatures[part_idx] = s.ok_or(ApplyError::MissingSignature(part_idx))?;
        }

        Ok(signatures)
    }

    pub fn apply(self) -> Result<(), (Self, ApplyError)> {
        let signatures = match self.signatures() {
            Ok(v) => v,
            Err(e) => return Err((self, e)),
        };
        match self.channel.force_update(self.new_state, signatures) {
            Ok(_) => Ok(()),
            Err(e) => Err((self, e.into())),
        }
    }
}
