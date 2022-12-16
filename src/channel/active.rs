use super::{fixed_size_payment, PartID};
use crate::{
    abiencode::types::{Hash, Signature},
    wire::MessageBus,
    PerunClient,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

#[derive(Debug)]
pub struct ActiveChannel<'a, B: MessageBus> {
    part_id: PartID,
    client: &'a PerunClient<B>,
    state: State,
    params: Params,
    signatures: [Signature; PARTICIPANTS],
}

impl<'a, B: MessageBus> ActiveChannel<'a, B> {
    pub(super) fn new(
        client: &'a PerunClient<B>,
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
}
