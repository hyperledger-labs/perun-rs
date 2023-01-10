use super::ConversionError;
use crate::{abiencode::types::Signature, channel::fixed_size_payment, perunwire};

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
            participant: 1, // TODO
            state: Some(perunwire::SignedState {
                params: Some(value.params.into()),
                state: Some(value.state.into()),
                sigs: value.signatures.map(|sig| sig.0.to_vec()).to_vec(),
            }),
        }
    }
}
