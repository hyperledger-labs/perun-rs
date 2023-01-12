use serde::Serialize;

use crate::{
    abiencode::{self, types::U256},
    messages::SignedWithdrawalAuth,
    sig::Signer,
    Address, Hash,
};

use super::{fixed_size_payment, PartID};

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
    part_id: PartID,
) -> Result<[SignedWithdrawalAuth; ASSETS], abiencode::Error> {
    let mut withdrawal_auths = [SignedWithdrawalAuth::default(); ASSETS];
    for i in 0..ASSETS {
        let sig = signer.sign_eth(abiencode::to_hash(&WithdrawalAuth {
            channel_id: channel_id,
            participant: params.participants[part_id],
            receiver: withdraw_receiver,
            amount: state.outcome.balances.0[i].0[part_id],
        })?);
        withdrawal_auths[i] = SignedWithdrawalAuth {
            sig,
            receiver: withdraw_receiver,
        }
    }

    Ok(withdrawal_auths)
}
