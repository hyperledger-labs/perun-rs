use super::{
    fixed_size_payment::{self},
    ChannelUpdate, PartID,
};
use crate::{
    abiencode::{
        self,
        types::{Address, Hash, Signature},
    },
    messages::{ConversionError, ParticipantMessage, WatcherMessage},
    perunwire, sig,
    wire::MessageBus,
    PerunClient,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelUpdate {
    pub state: State,
    pub actor_idx: PartID,
    pub sig: Signature,
}

impl TryFrom<perunwire::ChannelUpdateMsg> for LedgerChannelUpdate {
    type Error = ConversionError;

    fn try_from(value: perunwire::ChannelUpdateMsg) -> Result<Self, Self::Error> {
        let update = value.channel_update.ok_or(ConversionError::ExptectedSome)?;

        Ok(Self {
            state: update
                .state
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            actor_idx: update.actor_idx as usize,
            sig: Signature(
                value
                    .sig
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
        })
    }
}

impl From<LedgerChannelUpdate> for perunwire::ChannelUpdateMsg {
    fn from(value: LedgerChannelUpdate) -> Self {
        Self {
            channel_update: Some(perunwire::ChannelUpdate {
                state: Some(value.state.into()),
                actor_idx: value.actor_idx as u32,
            }),
            sig: value.sig.0.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelWatchUpdate {
    pub state: State,
    pub signatures: [Signature; PARTICIPANTS],
}

impl TryFrom<perunwire::WatchUpdateMsg> for LedgerChannelWatchUpdate {
    type Error = ConversionError;

    fn try_from(value: perunwire::WatchUpdateMsg) -> Result<Self, Self::Error> {
        if value.sigs.len() != PARTICIPANTS {
            Err(ConversionError::ParticipantSizeMissmatch)
        } else {
            let mut signatures = [Signature::default(); PARTICIPANTS];
            for (a, b) in signatures.iter_mut().zip(value.sigs) {
                *a = Signature(b.try_into().or(Err(ConversionError::ByteLengthMissmatch))?)
            }

            Ok(Self {
                state: value
                    .state
                    .ok_or(ConversionError::ExptectedSome)?
                    .try_into()?,
                signatures,
            })
        }
    }
}

impl From<LedgerChannelWatchUpdate> for perunwire::WatchUpdateMsg {
    fn from(value: LedgerChannelWatchUpdate) -> Self {
        Self {
            state: Some(value.state.into()),
            sigs: value.signatures.map(|s| s.0.to_vec()).to_vec(),
        }
    }
}

#[derive(Debug)]
pub enum ProposeUpdateError {
    AbiEncodeError(abiencode::Error),
}
impl From<abiencode::Error> for ProposeUpdateError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}

#[derive(Debug)]
pub enum HandleUpdateError {
    AbiEncodeError(abiencode::Error),
    RecoveryFailed(sig::Error),
    InvalidSignature(Address),
    InvalidChannelID,
    InvalidVersionNumber,
}
impl From<abiencode::Error> for HandleUpdateError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}
impl From<sig::Error> for HandleUpdateError {
    fn from(e: sig::Error) -> Self {
        Self::RecoveryFailed(e)
    }
}

#[derive(Debug)]
pub struct ActiveChannel<'a, B: MessageBus> {
    part_id: PartID,
    client: &'a PerunClient<B>,
    state: State,
    params: Params,
    signatures: [Signature; PARTICIPANTS],
}

impl<'cl, B: MessageBus> ActiveChannel<'cl, B> {
    pub(super) fn new(
        client: &'cl PerunClient<B>,
        part_id: PartID,
        init_state: State,
        params: Params,
        signatures: [Signature; PARTICIPANTS],
    ) -> Self {
        ActiveChannel {
            part_id,
            client,
            state: init_state,
            params,
            signatures,
        }
    }

    pub fn channel_id(&self) -> Hash {
        self.state.channel_id()
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn part_id(&self) -> PartID {
        self.part_id
    }

    pub fn client(&self) -> &PerunClient<B> {
        self.client
    }

    pub fn params(&self) -> Params {
        self.params
    }

    pub fn update<'ch>(
        &'ch mut self,
        new_state: State,
    ) -> Result<ChannelUpdate<'ch, 'cl, B>, ProposeUpdateError> {
        // TODO: Verify a few things about the state (like no creation of new
        // assets).

        // Sign immediately, we need the signature to send the proposal.
        let hash = abiencode::to_hash(&new_state)?;
        let sig = self.client.signer.sign_eth(hash);
        self.client
            .bus
            .send_to_participants(ParticipantMessage::ChannelUpdate(LedgerChannelUpdate {
                state: new_state,
                actor_idx: self.part_id,
                sig,
            }));

        Ok(ChannelUpdate::new(self, new_state, self.part_id, sig))
    }

    pub fn handle_update<'ch>(
        &'ch mut self,
        msg: LedgerChannelUpdate,
    ) -> Result<ChannelUpdate<'ch, 'cl, B>, HandleUpdateError> {
        if msg.state.channel_id() != self.state.channel_id() {
            return Err(HandleUpdateError::InvalidChannelID);
        }
        if msg.state.version() != self.state.version() + 1 {
            return Err(HandleUpdateError::InvalidVersionNumber);
        }

        let hash = abiencode::to_hash(&msg.state)?;
        let signer = self.client.signer.recover_signer(hash, msg.sig)?;

        if self.params.participants[msg.actor_idx] != signer {
            return Err(HandleUpdateError::InvalidSignature(signer));
        }

        Ok(ChannelUpdate::new(self, msg.state, msg.actor_idx, msg.sig))
    }

    pub(super) fn force_update(&mut self, new_state: State, signatures: [Signature; PARTICIPANTS]) {
        self.state = new_state;
        self.signatures = signatures;
        self.send_current_state_to_watcher();
    }

    pub fn send_current_state_to_watcher(&self) {
        self.client
            .bus
            .send_to_watcher(WatcherMessage::Update(LedgerChannelWatchUpdate {
                state: self.state,
                signatures: self.signatures,
            }))
    }

    // Use `update()` if the state has to change, too
    pub fn close_normal<'ch>(
        &'ch mut self,
    ) -> Result<ChannelUpdate<'ch, 'cl, B>, ProposeUpdateError> {
        let mut new_state = self.state.make_next_state();
        new_state.is_final = true;
        self.update(new_state)
    }

    // At the moment this just drops the channel after sending the message. In
    // the future it might make sense to have a struct representing a closing
    // channel, for example to allow resending the last message.
    pub fn force_close(self) {
        self.client
            .bus
            .send_to_watcher(WatcherMessage::StartDispute(LedgerChannelWatchUpdate {
                state: self.state,
                signatures: self.signatures,
            }));
    }

    // At the moment this just drops the channel. In the future it might make
    // sense to have a struct representing a closing channel, for example to
    // allow resending the last message.
    pub fn handle_dispute(self) {}
}
