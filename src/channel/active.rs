use super::{fixed_size_payment, ChannelUpdate, PartID};
use crate::{
    abiencode::{
        self,
        types::{Address, Hash, Signature},
    },
    sig,
    wire::{MessageBus, ParticipantMessage, WatcherMessage},
    PerunClient,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelUpdate {
    state: State,
    actor_idx: PartID,
    sig: Signature,
}

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelWatchUpdate {
    pub state: State,
    pub signatures: [Signature; PARTICIPANTS],
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

    pub fn state(&self) -> &State {
        // TODO: Check if we need it or if the Copy is sufficient (it should)
        //
        // Return an immutable reference to the state. In Go or C this would
        // allow the caller to modify the internal state of the channel due to
        // the lack of distinction between mutable and immutable references (it
        // is a pointer). Therefore we don't have to clone (or force a copy)
        // upon the caller in Rust, which would probably be the ideal way to do
        // it in Go.
        &self.state
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
        // TODO: How to handle/prevent the case that this channel is already
        // closed? New channel type/dropped channel/runtime error?

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

        self.client
            .bus
            .send_to_watcher(WatcherMessage::Update(LedgerChannelWatchUpdate {
                state: self.state,
                signatures: self.signatures,
            }))
    }
}
