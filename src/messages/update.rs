use super::ConversionError;
use crate::{
    abiencode::types::{Hash, Signature},
    perunwire,
};

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
