use super::ConversionError;
use crate::{abiencode::types::Signature, channel::fixed_size_payment, perunwire, Address};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelWatchRequest {
    pub params: Params,
    pub state: State,
    pub signatures: [Signature; PARTICIPANTS],
}

impl TryFrom<perunwire::WatchRequestMsg> for LedgerChannelWatchRequest {
    type Error = ConversionError;

    fn try_from(value: perunwire::WatchRequestMsg) -> Result<Self, Self::Error> {
        let signed_state = value.state.ok_or(ConversionError::ExptectedSome)?;

        if signed_state.sigs.len() != PARTICIPANTS {
            return Err(ConversionError::ParticipantSizeMissmatch);
        }

        let mut signatures = [Signature::default(); PARTICIPANTS];
        for (a, b) in signatures.iter_mut().zip(signed_state.sigs) {
            *a = Signature(b.try_into().or(Err(ConversionError::ByteLengthMissmatch))?);
        }

        Ok(Self {
            params: signed_state
                .params
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            state: signed_state
                .state
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            signatures,
        })
    }
}

impl From<LedgerChannelWatchRequest> for perunwire::WatchRequestMsg {
    fn from(value: LedgerChannelWatchRequest) -> Self {
        Self {
            participant: 0, // TODO
            state: Some(perunwire::SignedState {
                params: Some(value.params.into()),
                state: Some(value.state.into()),
                sigs: value.signatures.map(|sig| sig.0.to_vec()).to_vec(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SignedWithdrawalAuth {
    pub sig: Signature,
    pub receiver: Address,
}

impl TryFrom<perunwire::SignedWithdrawalAuth> for SignedWithdrawalAuth {
    type Error = ConversionError;

    fn try_from(value: perunwire::SignedWithdrawalAuth) -> Result<Self, Self::Error> {
        Ok(Self {
            sig: Signature(
                value
                    .sig
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            receiver: Address(
                value
                    .receiver
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
        })
    }
}

impl From<SignedWithdrawalAuth> for perunwire::SignedWithdrawalAuth {
    fn from(value: SignedWithdrawalAuth) -> Self {
        Self {
            sig: value.sig.0.to_vec(),
            receiver: value.receiver.0.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelWatchUpdate {
    pub state: State,
    pub signatures: [Signature; PARTICIPANTS],
    pub withdrawal_auths: [SignedWithdrawalAuth; ASSETS],
}

impl TryFrom<perunwire::WatchUpdateMsg> for LedgerChannelWatchUpdate {
    type Error = ConversionError;

    fn try_from(value: perunwire::WatchUpdateMsg) -> Result<Self, Self::Error> {
        if value.sigs.len() != PARTICIPANTS {
            Err(ConversionError::ParticipantSizeMissmatch)
        } else if value.withdrawal_auths.len() != ASSETS {
            Err(ConversionError::AssetSizeMissmatch)
        } else {
            let mut signatures = [Signature::default(); PARTICIPANTS];
            for (a, b) in signatures.iter_mut().zip(value.sigs) {
                *a = Signature(b.try_into().or(Err(ConversionError::ByteLengthMissmatch))?)
            }

            let mut withdrawal_auths = [SignedWithdrawalAuth::default(); ASSETS];
            for (a, b) in withdrawal_auths.iter_mut().zip(value.withdrawal_auths) {
                *a = b.try_into()?;
            }

            Ok(Self {
                state: value
                    .state
                    .ok_or(ConversionError::ExptectedSome)?
                    .try_into()?,
                signatures,
                withdrawal_auths,
            })
        }
    }
}

impl From<LedgerChannelWatchUpdate> for perunwire::WatchUpdateMsg {
    fn from(value: LedgerChannelWatchUpdate) -> Self {
        Self {
            state: Some(value.state.into()),
            sigs: value.signatures.map(|s| s.0.to_vec()).to_vec(),
            withdrawal_auths: value.withdrawal_auths.map(|a| a.into()).to_vec(),
        }
    }
}

impl TryFrom<perunwire::ForceCloseRequestMsg> for LedgerChannelWatchUpdate {
    type Error = ConversionError;

    fn try_from(value: perunwire::ForceCloseRequestMsg) -> Result<Self, Self::Error> {
        value
            .latest
            .ok_or(ConversionError::ExptectedSome)?
            .try_into()
    }
}

impl From<LedgerChannelWatchUpdate> for perunwire::ForceCloseRequestMsg {
    fn from(value: LedgerChannelWatchUpdate) -> Self {
        Self {
            channel_id: value.state.channel_id().0.to_vec(),
            latest: Some(value.into()),
        }
    }
}
