use super::{active::ActiveChannel, agreed_upon::AddSignatureError, fixed_size_payment, PartID};
use crate::{
    abiencode::{self, types::Signature},
    messages::{LedgerChannelUpdateAccepted, ParticipantMessage},
    wire::MessageBus,
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
pub struct MissingSignatureError {
    pub part_id: PartID,
}

#[derive(Debug)]
pub struct ChannelUpdate<'ch, 'cl, B: MessageBus> {
    channel: &'ch mut ActiveChannel<'cl, B>,
    new_state: State,
    signatures: [Option<Signature>; PARTICIPANTS],
}

impl<'ch, 'cl, B: MessageBus> ChannelUpdate<'ch, 'cl, B> {
    pub(crate) fn new(
        channel: &'ch mut ActiveChannel<'cl, B>,
        new_state: State,
        sig_part_id: PartID,
        sig: Signature,
    ) -> Self {
        let mut signatures = [None; PARTICIPANTS];
        signatures[sig_part_id] = Some(sig);
        ChannelUpdate {
            channel,
            new_state,
            signatures,
        }
    }

    pub fn accept(&mut self) -> Result<(), AcceptError> {
        match self.signatures[self.channel.part_id()] {
            Some(_) => Err(AcceptError::AlreadyAccepted),
            None => {
                let hash = abiencode::to_hash(&self.new_state)?;
                let sig = self.channel.client().signer.sign_eth(hash);

                let acc: _ = LedgerChannelUpdateAccepted {
                    channel: self.channel.channel_id(),
                    version: self.new_state.version(),
                    sig: sig,
                };
                self.signatures[self.channel.part_id()] = Some(sig);
                self.channel
                    .client()
                    .bus
                    .send_to_participants(ParticipantMessage::ChannelUpdateAccepted(acc));
                Ok(())
            }
        }
    }

    pub fn reject(self, reason: &str) {
        self.channel
            .client()
            .bus
            .send_to_participants(ParticipantMessage::ChannelUpdateRejected {
                id: self.channel.channel_id(),
                version: self.new_state.version(),
                reason: reason.to_string(),
            });
    }

    pub fn participant_accepted(
        &mut self,
        part_id: PartID,
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

        if self.channel.params().participants[part_id] != signer {
            return Err(AddSignatureError::InvalidSignature(signer));
        }

        match self.signatures[part_id] {
            Some(_) => Err(AddSignatureError::AlreadySigned),
            None => {
                self.signatures[part_id] = Some(msg.sig);
                Ok(())
            }
        }
    }

    pub fn signatures(&self) -> Result<[Signature; PARTICIPANTS], MissingSignatureError> {
        let mut signatures: [Signature; PARTICIPANTS] = [Signature::default(); PARTICIPANTS];
        for (part_id, s) in self.signatures.iter().enumerate() {
            signatures[part_id] = s.ok_or(MissingSignatureError { part_id })?;
        }

        Ok(signatures)
    }

    pub fn apply(self) -> Result<(), MissingSignatureError> {
        self.channel
            .force_update(self.new_state, self.signatures()?);
        Ok(())
    }
}
