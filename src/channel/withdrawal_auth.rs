use serde::Serialize;

use crate::{
    abiencode::{self, types::U256},
    messages::SignedWithdrawalAuth,
    sig::Signer,
    Address, Hash,
};

use super::{fixed_size_payment, PartIdx};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

#[derive(Serialize, Debug, Copy, Clone)]
struct WithdrawalAuth {
    pub channel_id: Hash,
    pub participant: Address, // Off-chain channel address
    pub receiver: Address,    // On-chain receiver of funds on withdrawal
    pub amount: U256,
}

pub fn make_signed_withdrawal_auths(
    signer: &Signer,
    channel_id: Hash,
    params: Params,
    state: State,
    withdraw_receiver: Address,
    part_idx: PartIdx,
) -> Result<[SignedWithdrawalAuth; ASSETS], abiencode::Error> {
    let mut withdrawal_auths = [SignedWithdrawalAuth::default(); ASSETS];

    // Just a defensive measure in case the State type is changed without
    // removing or updating ASSETS.
    debug_assert_eq!(withdrawal_auths.len(), state.outcome.balances.0.len());
    for (auth, bals) in withdrawal_auths.iter_mut().zip(state.outcome.balances.0) {
        let sig = signer.sign_eth(abiencode::to_hash(&WithdrawalAuth {
            channel_id,
            participant: params.participants[part_idx],
            receiver: withdraw_receiver,
            amount: bals.0[part_idx],
        })?);
        *auth = SignedWithdrawalAuth {
            sig,
            receiver: withdraw_receiver,
        }
    }

    Ok(withdrawal_auths)
}
