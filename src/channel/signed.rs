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
pub struct SignedChannel<'cl, B: MessageBus>(ActiveChannel<'cl, B>);

impl<'cl, B: MessageBus> SignedChannel<'cl, B> {
    pub(super) fn new(
        client: &'cl PerunClient<B>,
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

    pub fn mark_funded(self) -> ActiveChannel<'cl, B> {
        self.0
    }

    pub fn channel_id(&self) -> Hash {
        self.0.channel_id()
    }
}
