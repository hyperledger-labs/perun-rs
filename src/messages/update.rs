use super::ConversionError;
use crate::{
    abiencode::types::{Hash, Signature},
    channel::{fixed_size_payment, PartIdx},
    perunwire,
};

const ASSETS: usize = 1;
const PARTICIPANTS: usize = 2;
type State = fixed_size_payment::State<ASSETS, PARTICIPANTS>;

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelUpdate {
    pub state: State,
    pub actor_idx: PartIdx,
    pub sig: Signature,
}

impl TryFrom<perunwire::ChannelUpdateMsg> for LedgerChannelUpdate {
    type Error = ConversionError;

    fn try_from(value: perunwire::ChannelUpdateMsg) -> Result<Self, Self::Error> {
        let update = value.channel_update.ok_or(ConversionError::ExptectedSome)?;

        Ok(Self {
            state: update
                .state
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            actor_idx: update.actor_idx as usize,
            sig: Signature(
                value
                    .sig
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
        })
    }
}

impl From<LedgerChannelUpdate> for perunwire::ChannelUpdateMsg {
    fn from(value: LedgerChannelUpdate) -> Self {
        Self {
            channel_update: Some(perunwire::ChannelUpdate {
                state: Some(value.state.into()),
                actor_idx: value.actor_idx as u32,
            }),
            sig: value.sig.0.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelUpdateAccepted {
    pub channel: Hash,
    pub version: u64,
    pub sig: Signature,
}

impl TryFrom<perunwire::ChannelUpdateAccMsg> for LedgerChannelUpdateAccepted {
    type Error = ConversionError;

    fn try_from(value: perunwire::ChannelUpdateAccMsg) -> Result<Self, Self::Error> {
        Ok(LedgerChannelUpdateAccepted {
            channel: Hash(
                value
                    .channel_id
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            version: value.version,
            sig: Signature(
                value
                    .sig
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
        })
    }
}

impl From<LedgerChannelUpdateAccepted> for perunwire::ChannelUpdateAccMsg {
    fn from(value: LedgerChannelUpdateAccepted) -> Self {
        Self {
            channel_id: value.channel.0.to_vec(),
            version: value.version,
            sig: value.sig.0.to_vec(),
        }
    }
}
