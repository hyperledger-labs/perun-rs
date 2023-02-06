use super::{active::ActiveChannel, fixed_size_payment, PartIdx, Peers};
use crate::{
    abiencode::types::{Hash, Signature},
    wire::MessageBus,
    Address, PerunClient,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

#[derive(Debug)]
pub struct SignedChannel<'a, B: MessageBus>(ActiveChannel<'a, B>);

impl<'a, B: MessageBus> SignedChannel<'a, B> {
    pub(super) fn new(
        client: &'a PerunClient<B>,
        part_idx: PartIdx,
        withdraw_receiver: Address,
        init_state: State,
        params: Params,
        signatures: [Signature; PARTICIPANTS],
        peers: Peers,
    ) -> Self {
        SignedChannel(ActiveChannel::new(
            client,
            part_idx,
            withdraw_receiver,
            init_state,
            params,
            signatures,
            peers,
        ))
    }

    pub fn mark_funded(self) -> ActiveChannel<'a, B> {
        self.0
    }

    pub fn channel_id(&self) -> Hash {
        self.0.channel_id()
    }
}
