use super::{fixed_size_payment, PartID};
use crate::{abiencode::types::Signature, wire::MessageBus, PerunClient};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

#[derive(Debug)]
pub struct SignedChannel<'a, B: MessageBus> {
    part_id: PartID,
    client: &'a PerunClient<B>,
    init_state: State,
    params: Params,
    signatures: [Signature; PARTICIPANTS],
}

impl<'a, B: MessageBus> SignedChannel<'a, B> {
    pub(super) fn new(
        client: &'a PerunClient<B>,
        part_id: PartID,
        init_state: State,
        params: Params,
        signatures: [Signature; PARTICIPANTS],
    ) -> Self {
        SignedChannel {
            part_id,
            client,
            init_state,
            params,
            signatures,
        }
    }
}
