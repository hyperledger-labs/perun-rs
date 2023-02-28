use super::{
    active::ActiveChannel, agreed_upon::AddSignatureError, fixed_size_payment, PartIdx, SignError,
};
use crate::{
    abiencode::{self, types::Signature},
    messages::{LedgerChannelUpdateAccepted, ParticipantMessage},
    wire::{BroadcastMessageBus, MessageBus},
    Hash,
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
    WrongVersion,
    WrongChannelId,
}
impl From<abiencode::Error> for AcceptError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}
impl From<InvalidChannel> for AcceptError {
    fn from(e: InvalidChannel) -> Self {
        match e {
            InvalidChannel::WrongVersion => Self::WrongVersion,
            InvalidChannel::WrongChannelId => Self::WrongChannelId,
        }
    }
}

#[derive(Debug)]
pub enum ApplyError {
    MissingSignature(PartIdx),
    SignError(SignError),
    WrongVersion,
    WrongChannelId,
}
impl From<SignError> for ApplyError {
    fn from(e: SignError) -> Self {
        Self::SignError(e)
    }
}
impl From<InvalidChannel> for ApplyError {
    fn from(e: InvalidChannel) -> Self {
        match e {
            InvalidChannel::WrongVersion => Self::WrongVersion,
            InvalidChannel::WrongChannelId => Self::WrongChannelId,
        }
    }
}

pub enum InvalidChannel {
    WrongVersion,
    WrongChannelId,
}

#[derive(Debug)]
pub struct ChannelUpdate {
    // Previously we had a mutable reference here, which gave a good amount of
    // guarantees on the type-system level. Unfortunately this proved quite
    // difficult to work with so we've reduced the amount of compile-time
    // guarantees:
    // - It is now possible to create multiple ChannelUpdates, any of which may
    //   be applied, making all others invalid at the same time.
    // - Trying to apply an invalidated (outdated) channel update will now
    //   result in a runtime error, whereas this was previously impossible due
    //   to compile-time checks.
    //
    // This does assume that there is only one channel with the same channel_id,
    // breaking this assumption would require the user to intentionally create a
    // second, identical channel (as the ID is computed from the hash of the
    // parameters and you can't just duplicate a channel as there is no
    // Copy/Clone), which would break a lot of other things and might make both
    // channels insecure. TLDR: Using the channel Hash instead of the address of
    // the channel in memory moves some checks to the runtime and allows
    // multiple pending updates (which may or may not be desireable) without
    // degregading security or introducing things the user/application developer
    // could accidentaly get wrong.
    channel_id: Hash,
    new_state: State,
    signatures: [Option<Signature>; PARTICIPANTS],
}

impl ChannelUpdate {
    pub(crate) fn new(
        channel: &ActiveChannel<impl MessageBus>,
        new_state: State,
        sig_part_idx: PartIdx,
        sig: Signature,
    ) -> Self {
        let mut signatures = [None; PARTICIPANTS];
        signatures[sig_part_idx] = Some(sig);
        ChannelUpdate {
            channel_id: channel.channel_id(),
            new_state,
            signatures,
        }
    }

    pub fn accept(
        &mut self,
        channel: &mut ActiveChannel<impl MessageBus>,
    ) -> Result<(), AcceptError> {
        self.ensure_valid_channel(channel)?;

        match self.signatures[channel.part_idx()] {
            Some(_) => Err(AcceptError::AlreadyAccepted),
            None => {
                let hash = abiencode::to_hash(&self.new_state)?;
                let sig = channel.client().signer.sign_eth(hash);

                let acc: _ = LedgerChannelUpdateAccepted {
                    channel: self.channel_id,
                    version: self.new_state.version(),
                    sig,
                };
                self.signatures[channel.part_idx()] = Some(sig);
                channel.client().bus.broadcast_to_participants(
                    channel.part_idx(),
                    channel.peers(),
                    ParticipantMessage::ChannelUpdateAccepted(acc),
                );
                Ok(())
            }
        }
    }

    pub fn reject(
        self,
        channel: &mut ActiveChannel<impl MessageBus>,
        reason: &str,
    ) -> Result<(), InvalidChannel> {
        self.ensure_valid_channel(channel)?;

        channel.client().bus.broadcast_to_participants(
            channel.part_idx(),
            channel.peers(),
            ParticipantMessage::ChannelUpdateRejected {
                id: self.channel_id,
                version: self.new_state.version(),
                reason: reason.to_string(),
            },
        );
        Ok(())
    }

    pub fn participant_accepted(
        &mut self,
        channel: &ActiveChannel<impl MessageBus>,
        part_idx: PartIdx,
        msg: LedgerChannelUpdateAccepted,
    ) -> Result<(), AddSignatureError> {
        self.ensure_valid_channel(channel)?;

        if msg.channel != self.channel_id {
            return Err(AddSignatureError::InvalidChannelID);
        }
        if msg.version != self.new_state.version() {
            return Err(AddSignatureError::InvalidVersionNumber);
        }

        let hash = abiencode::to_hash(&self.new_state)?;
        let signer = channel.client().signer.recover_signer(hash, msg.sig)?;

        if channel.params().participants[part_idx] != signer {
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

    fn ensure_valid_channel(
        &self,
        channel: &ActiveChannel<impl MessageBus>,
    ) -> Result<(), InvalidChannel> {
        if self.new_state.version() != channel.version() + 1 {
            Err(InvalidChannel::WrongVersion)
        } else if self.channel_id != channel.channel_id() {
            Err(InvalidChannel::WrongChannelId)
        } else {
            Ok(())
        }
    }

    pub fn apply(
        &mut self,
        channel: &mut ActiveChannel<impl MessageBus>,
    ) -> Result<(), ApplyError> {
        self.ensure_valid_channel(channel)?;

        channel.force_update(self.new_state, self.signatures()?)?;
        Ok(())
    }
}
