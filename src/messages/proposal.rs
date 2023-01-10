use crate::{
    abiencode::types::{Address, Bytes32, Hash},
    channel::{fixed_size_payment, NonceShare},
    messages::ConversionError,
    perunwire,
};
use alloc::vec;

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type Allocation = fixed_size_payment::Allocation<ASSETS, PARTICIPANTS>;
type Balances = fixed_size_payment::Balances<ASSETS, PARTICIPANTS>;

/// Channel configuration (also exchanged over the network)
#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelProposal {
    pub proposal_id: Hash,
    pub challenge_duration: u64,
    pub nonce_share: NonceShare,
    pub init_bals: Allocation,
    pub funding_agreement: Balances,
    pub participant: Address,
}

impl TryFrom<perunwire::LedgerChannelProposalMsg> for LedgerChannelProposal {
    type Error = ConversionError;

    fn try_from(value: perunwire::LedgerChannelProposalMsg) -> Result<Self, Self::Error> {
        let base = match value.base_channel_proposal {
            Some(v) => v,
            None => return Err(ConversionError::ExptectedSome),
        };
        let init_bals = match base.init_bals {
            Some(v) => v,
            None => return Err(ConversionError::ExptectedSome),
        };
        let funding_agreement = match base.funding_agreement {
            Some(v) => v,
            None => return Err(ConversionError::ExptectedSome),
        };

        Ok(LedgerChannelProposal {
            proposal_id: Hash(
                base.proposal_id
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            challenge_duration: base.challenge_duration,
            nonce_share: Bytes32(
                base.nonce_share
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            init_bals: init_bals.try_into()?,
            funding_agreement: funding_agreement.try_into()?,
            participant: Address(
                value
                    .participant
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
        })
    }
}

impl From<LedgerChannelProposal> for perunwire::LedgerChannelProposalMsg {
    fn from(value: LedgerChannelProposal) -> Self {
        Self {
            base_channel_proposal: Some(perunwire::BaseChannelProposal {
                proposal_id: value.proposal_id.0.to_vec(),
                challenge_duration: value.challenge_duration,
                nonce_share: value.nonce_share.0.to_vec(),
                app: vec![],
                init_data: vec![],
                init_bals: Some(value.init_bals.into()),
                funding_agreement: Some(value.funding_agreement.into()),
            }),
            participant: value.participant.0.to_vec(),
            peers: vec!["Alice".as_bytes().to_vec(), "Bob".as_bytes().to_vec()], // TODO: Use real data
        }
    }
}
