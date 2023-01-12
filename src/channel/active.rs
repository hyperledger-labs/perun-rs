use super::{
    channel_update::ChannelUpdate,
    fixed_size_payment::{self},
    withdrawal_auth, PartID, SignError,
};
use crate::{
    abiencode::{
        self,
        types::{Address, Hash, Signature},
    },
    messages::{
        LedgerChannelUpdate, LedgerChannelWatchUpdate, ParticipantMessage, WatcherRequestMessage,
    },
    sig,
    wire::MessageBus,
    PerunClient,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

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
    withdraw_receiver: Address,
    client: &'a PerunClient<B>,
    state: State,
    params: Params,
    signatures: [Signature; PARTICIPANTS],
}

impl<'cl, B: MessageBus> ActiveChannel<'cl, B> {
    pub(super) fn new(
        client: &'cl PerunClient<B>,
        part_id: PartID,
        withdraw_receiver: Address,
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
            withdraw_receiver,
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

    pub(super) fn force_update(
        &mut self,
        new_state: State,
        signatures: [Signature; PARTICIPANTS],
    ) -> Result<(), SignError> {
        // To prevent modifying self (the channel state+signatures) in case
        // send_current_state_to_watcher returns an Error we roll-back the
        // changes made here. At the moment this could only happen if we can't
        // abiencode the WithdrawalAuth message, but in the future this might
        // include Errors from the MessageBus.
        //
        // Alternatives considered:
        // - Make `make_watch_update` independent of self (associated funciton
        //   instead of member function). This would mean copying all the
        //   parameters to its call location (the function existed to not do
        //   that).
        // - Copy content of `make_watch_update` here (tried to avoid code
        //   duplication)
        // - give only the new state+signatures ot `make_watch_update` instead
        //   of rolling back. This would make the correctness dependent on
        //   whether `make_watch_update` uses the state passed via arguments or
        //   that in self, making it easy to use the wrong one and easy to not
        //   see that as a bug (especially since all other values are read from
        //   self) and this one is the exception.
        let old_state = self.state;
        let old_sigs = self.signatures;
        self.state = new_state;
        self.signatures = signatures;

        match self.send_current_state_to_watcher() {
            Ok(_) => Ok(()),
            Err(e) => {
                self.state = old_state;
                self.signatures = old_sigs;
                Err(e)
            }
        }
    }

    fn make_watch_update(&self) -> Result<LedgerChannelWatchUpdate, SignError> {
        let withdrawal_auths = withdrawal_auth::make_signed_withdrawal_auths(
            &self.client.signer,
            self.channel_id(),
            self.params,
            self.state,
            self.withdraw_receiver,
            self.part_id,
        )?;

        Ok(LedgerChannelWatchUpdate {
            state: self.state,
            signatures: self.signatures,
            withdrawal_auths,
        })
    }

    pub fn send_current_state_to_watcher(&self) -> Result<(), SignError> {
        self.client
            .bus
            .send_to_watcher(WatcherRequestMessage::Update(self.make_watch_update()?));
        Ok(())
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
    pub fn force_close(self) -> Result<(), SignError> {
        self.client
            .bus
            .send_to_watcher(WatcherRequestMessage::StartDispute(
                self.make_watch_update()?,
            ));
        Ok(())
    }

    // At the moment this just drops the channel. In the future it might make
    // sense to have a struct representing a closing channel, for example to
    // allow resending the last message.
    pub fn handle_dispute(self) {}
}
