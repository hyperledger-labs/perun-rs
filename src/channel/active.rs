use serde::Serialize;

use super::{
    channel_update::ChannelUpdate,
    fixed_size_payment::{self},
    PartID, SignError,
};
use crate::{
    abiencode::{
        self,
        types::{Address, Hash, Signature, U256},
    },
    messages::{
        LedgerChannelUpdate, LedgerChannelWatchUpdate, ParticipantMessage, SignedWithdrawalAuth,
        WatcherRequestMessage,
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

#[derive(Serialize, Debug, Copy, Clone)]
struct WithdrawalAuth {
    pub channel_id: Hash,
    pub participant: Address, // Off-chain channel address
    pub receiver: Address,    // On-chain receiver of funds on withdrawal
    pub amount: U256,
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
        self.state = new_state;
        self.signatures = signatures;
        self.send_current_state_to_watcher()
    }

    fn make_watch_update(&self) -> Result<LedgerChannelWatchUpdate, SignError> {
        let mut withdrawal_auths = [SignedWithdrawalAuth::default(); ASSETS];
        for i in 0..ASSETS {
            let sig = self
                .client
                .signer
                .sign_eth(abiencode::to_hash(&WithdrawalAuth {
                    channel_id: self.channel_id(),
                    participant: self.params.participants[self.part_id],
                    receiver: self.withdraw_receiver,
                    amount: self.state.outcome.balances.0[i].0[self.part_id],
                })?);
            withdrawal_auths[i] = SignedWithdrawalAuth {
                sig,
                receiver: self.withdraw_receiver,
            }
        }

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
