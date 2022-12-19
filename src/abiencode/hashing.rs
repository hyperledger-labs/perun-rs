use super::{to_writer, types::Hash, Error, Writer};

use serde::Serialize;
use sha3::{
    digest::{core_api::CoreWrapper, Output},
    Digest, Keccak256, Keccak256Core,
};

pub struct Keccak256Writer {
    hasher: CoreWrapper<Keccak256Core>,
}

impl Default for Keccak256Writer {
    fn default() -> Self {
        Self {
            hasher: Keccak256::new(),
        }
    }
}

impl Writer for Keccak256Writer {
    fn write(&mut self, slot: &[u8]) {
        self.hasher.update(slot);
    }
}

impl Keccak256Writer {
    pub fn finalize(self) -> Output<Keccak256> {
        self.hasher.finalize()
    }
}

pub fn to_hash<T>(value: &T) -> Result<Hash, Error>
where
    T: Serialize,
{
    let mut writer = Keccak256Writer::default();
    to_writer(value, &mut writer)?;
    Ok(Hash(writer.finalize().into()))
}
