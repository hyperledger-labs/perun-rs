use super::ConversionError;
use crate::{
    channel::{fixed_size_payment, PartIdx},
    perunwire,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;
type Params = fixed_size_payment::Params<PARTICIPANTS>;
type Balances = fixed_size_payment::Balances<ASSETS, PARTICIPANTS>;

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelFundingRequest {
    pub part_idx: PartIdx,
    pub funding_agreement: Balances,
    pub params: Params,
    pub state: State,
}

impl TryFrom<perunwire::FundingRequestMsg> for LedgerChannelFundingRequest {
    type Error = ConversionError;

    fn try_from(value: perunwire::FundingRequestMsg) -> Result<Self, Self::Error> {
        Ok(Self {
            part_idx: value.participant as usize,
            funding_agreement: value
                .funding_agreement
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            params: value
                .params
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            state: value
                .initial_state
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
        })
    }
}

impl From<LedgerChannelFundingRequest> for perunwire::FundingRequestMsg {
    fn from(value: LedgerChannelFundingRequest) -> Self {
        Self {
            funding_agreement: Some(value.funding_agreement.into()),
            params: Some(value.params.into()),
            initial_state: Some(value.state.into()),
            participant: value.part_idx as u32,
        }
    }
}
