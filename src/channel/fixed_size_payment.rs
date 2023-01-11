//! Rust representations of the on-chain Solidity data types with fixed amount
//! of Participants and Assets.
//!
//! These types can be useful when the number of Participants and Assets are
//! known at compile time or we don't have heap allocation.

use super::Asset;
use crate::{
    abiencode::{
        self, as_bytes, as_dyn_array,
        types::{Address, Hash, U256},
    },
    perunwire,
};
use alloc::vec;
use serde::Serialize;

#[derive(Debug)]
pub enum ConversionError {
    ParticipantSizeMissmatch,
    AssetSizeMissmatch,
    ByteLengthMissmatch,
    ExptectedSome,
    StateChannelsNotSupported,
}

/// Parameters for this channel, exchanged during channel proposal and sent
/// on-chain during a dispute.
#[derive(Serialize, Debug, Copy, Clone)]
pub struct Params<const P: usize> {
    pub challenge_duration: u64,
    pub nonce: U256,
    #[serde(with = "as_dyn_array")]
    pub participants: [Address; P],
    pub app: Address,
    pub ledger_channel: bool,
    pub virtual_channel: bool,
}

impl<const P: usize> Params<P> {
    fn channel_id(&self) -> Result<Hash, abiencode::Error> {
        abiencode::to_hash(self)
    }
}

impl<const P: usize> TryFrom<perunwire::Params> for Params<P> {
    type Error = ConversionError;

    fn try_from(value: perunwire::Params) -> Result<Self, Self::Error> {
        let mut participants = [Address::default(); P];
        for (a, b) in participants.iter_mut().zip(value.parts) {
            *a = Address(b.try_into().or(Err(ConversionError::ByteLengthMissmatch))?);
        }

        Ok(Self {
            challenge_duration: value.challenge_duration,
            nonce: U256::from_big_endian(&value.nonce),
            participants,
            app: Address(
                value
                    .app
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            ledger_channel: value.ledger_channel,
            virtual_channel: value.virtual_channel,
        })
    }
}

impl<const P: usize> From<Params<P>> for perunwire::Params {
    fn from(value: Params<P>) -> Self {
        Self {
            id: value
                .channel_id()
                .expect("should be impossible to get an encoding-error for a Params object")
                .0
                .to_vec(),
            challenge_duration: value.challenge_duration,
            nonce: {
                let mut buf = vec![0u8; 32];
                value.nonce.to_big_endian(&mut buf);
                buf
            },
            parts: value.participants.map(|a| a.0.to_vec()).to_vec(),
            app: value.app.0.to_vec(),
            ledger_channel: value.ledger_channel,
            virtual_channel: value.virtual_channel,
        }
    }
}

/// Stores the complete state of a channel.
#[derive(Serialize, Debug, Copy, Clone)]
pub struct State<const A: usize, const P: usize> {
    id: Hash,
    version: u64,
    pub outcome: Allocation<A, P>,
    #[serde(with = "as_bytes")]
    app_data: [u8; 0],
    pub is_final: bool,
}

impl<const A: usize, const P: usize> State<A, P> {
    pub fn version(&self) -> u64 {
        self.version
    }
    pub fn channel_id(&self) -> Hash {
        self.id
    }
}

impl<const A: usize, const P: usize> State<A, P> {
    pub fn new(params: Params<P>, init_bals: Allocation<A, P>) -> Result<Self, abiencode::Error> {
        Ok(State {
            id: params.channel_id()?,
            version: 0,
            outcome: init_bals,
            app_data: [],
            is_final: false,
        })
    }

    /// Create a new state that will replace this state.
    ///
    /// Having id and version as private fields forces the caller to not
    /// accidentally write garbage to one of those fields, which could only be
    /// cought by the Channels via panics or by returning an Error, thus
    /// requiring extra checks at runtime. This forces compatibility at compile
    /// time.
    pub fn make_next_state(&self) -> Self {
        State {
            id: self.id,
            version: self.version + 1,
            outcome: self.outcome,
            app_data: self.app_data,
            is_final: self.is_final,
        }
    }
}

impl<const A: usize, const P: usize> TryFrom<perunwire::State> for State<A, P> {
    type Error = ConversionError;

    fn try_from(value: perunwire::State) -> Result<Self, Self::Error> {
        if value.data.len() != 0 {
            return Err(ConversionError::StateChannelsNotSupported);
        }

        Ok(Self {
            id: Hash(
                value
                    .id
                    .try_into()
                    .or(Err(ConversionError::ByteLengthMissmatch))?,
            ),
            version: value.version,
            outcome: value
                .allocation
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            app_data: [],
            is_final: value.is_final,
        })
    }
}

impl<const A: usize, const P: usize> From<State<A, P>> for perunwire::State {
    fn from(value: State<A, P>) -> Self {
        Self {
            id: value.id.0.to_vec(),
            version: value.version,
            allocation: Some(value.outcome.into()),
            app: vec![], // Only different if it is a state channel, which we don't support, yet
            data: value.app_data.to_vec(),
            is_final: value.is_final,
        }
    }
}

/// Separate type for storing just the allocated balance, not the assets.
///
/// This type is used in the channel proposals to specify the funding agreement.
#[derive(Serialize, Debug, Copy, Clone)]
#[serde(transparent)]
pub struct Balances<const A: usize, const P: usize>(
    #[serde(with = "as_dyn_array")] pub [ParticipantBalances<P>; A],
);

impl<const A: usize, const P: usize> Default for Balances<A, P> {
    fn default() -> Self {
        Self([ParticipantBalances::default(); A])
    }
}

impl<const A: usize, const P: usize> TryFrom<perunwire::Balances> for Balances<A, P> {
    type Error = ConversionError;

    fn try_from(value: perunwire::Balances) -> Result<Self, Self::Error> {
        if value.balances.len() != A {
            Err(ConversionError::AssetSizeMissmatch)
        } else {
            let mut balances = Self::default();
            for (a, b) in balances.0.iter_mut().zip(value.balances) {
                *a = b.try_into()?;
            }

            Ok(balances)
        }
    }
}

impl<const A: usize, const P: usize> From<Balances<A, P>> for perunwire::Balances {
    fn from(value: Balances<A, P>) -> Self {
        perunwire::Balances {
            balances: value.0.map(|x| x.into()).to_vec(),
        }
    }
}

/// Stores which participant has how much of each asset.
#[derive(Serialize, Debug, Copy, Clone)]
pub struct Allocation<const A: usize, const P: usize> {
    #[serde(with = "as_dyn_array")]
    pub assets: [Asset; A],
    pub balances: Balances<A, P>,
    #[serde(with = "as_dyn_array")]
    locked: [(); 0], // Only needed for encoding
}

impl<const A: usize, const P: usize> TryFrom<perunwire::Allocation> for Allocation<A, P> {
    type Error = ConversionError;

    fn try_from(value: perunwire::Allocation) -> Result<Self, Self::Error> {
        let mut assets = [Asset::default(); A];
        for (a, b) in assets.iter_mut().zip(value.assets) {
            if b.len() < 2 + 20 + 2 {
                // We have to at least store two lengths (2 bytes each), the first of which has
                // to be 20 bytes.
                return Err(ConversionError::ByteLengthMissmatch);
            }
            // holder
            let holder_length = u16::from_le_bytes(b[..2].try_into().unwrap());
            if holder_length != 20u16 {
                return Err(ConversionError::ByteLengthMissmatch);
            }
            let b: &[u8] = &b[2..];
            let mut holder = Address::default();
            holder.0.copy_from_slice(&b[..20]);
            let b = &b[20..];
            // chainid
            let chain_id_length = u16::from_le_bytes(b[..2].try_into().unwrap());
            if chain_id_length > 32u16 || b.len() != chain_id_length as usize {
                // if it is larger than 32 bytes we cannot represent it in this
                // type, and a larger value (while representable in Go) doesn't
                // make sense in this context. Additionally, the buffer b has to
                // have this amount of bytes remaining, which is not checked in
                // the first condition.
                return Err(ConversionError::ByteLengthMissmatch);
            }
            // We can only use from_big_endian if we have 32 bytes
            let mut buffer = [0u8; 32];
            buffer[(32 - chain_id_length as usize)..].copy_from_slice(b);
            let chain_id = U256::from_big_endian(&buffer);

            *a = Asset { chain_id, holder }
        }

        Ok(Self {
            assets,
            balances: value
                .balances
                .ok_or(ConversionError::ExptectedSome)?
                .try_into()?,
            locked: [],
        })
    }
}

impl<const A: usize, const P: usize> From<Allocation<A, P>> for perunwire::Allocation {
    fn from(value: Allocation<A, P>) -> Self {
        perunwire::Allocation {
            assets: value
                .assets
                .map(|a| {
                    let mut b = vec![];

                    // go-perun uses less bytes, as it strips away some leading
                    // zeroes, which this implementation does not (for
                    // simplicity). However this should still be understandable
                    // by go-perun.
                    b.extend_from_slice(&32u16.to_le_bytes());
                    let mut buf = [0u8; 32];
                    a.chain_id.to_big_endian(&mut buf);
                    b.extend_from_slice(&buf);

                    // go-perun currently uses `encoding/binary` in go and
                    // manually adds the length of each field.
                    b.extend_from_slice(&20u16.to_le_bytes()); // Length of asset holder (address)
                    b.extend_from_slice(&a.holder.0);

                    b
                })
                .to_vec(),
            balances: Some(value.balances.into()),
            locked: vec![],
        }
    }
}

impl<const A: usize, const P: usize> Allocation<A, P> {
    pub fn new(assets: [Asset; A], balances: Balances<A, P>) -> Self {
        Self {
            assets,
            balances,
            locked: [],
        }
    }
}

/// Currently needed as a work-around for marking nested arrays as dynamic.
///
/// We cannot easily set the `serde(with = "...")` attribute or use a custom
/// serialization method if the item type of the outer array does not have its
/// own type. It should be possible to do it by wrapping each item into a new
/// type before calling `serialize_element`.
#[derive(Serialize, Debug, Copy, Clone)]
#[serde(transparent)]
pub struct ParticipantBalances<const P: usize>(#[serde(with = "as_dyn_array")] pub [U256; P]);

impl<const P: usize> Default for ParticipantBalances<P> {
    fn default() -> Self {
        Self([U256::default(); P])
    }
}

impl<const P: usize> TryFrom<perunwire::Balance> for ParticipantBalances<P> {
    type Error = ConversionError;

    fn try_from(value: perunwire::Balance) -> Result<Self, Self::Error> {
        if value.balance.len() != P {
            Err(ConversionError::ParticipantSizeMissmatch)
        } else {
            let mut balances = Self::default();
            for (a, b) in balances.0.iter_mut().zip(value.balance) {
                *a = U256::from_big_endian(&b)
            }
            Ok(balances)
        }
    }
}

impl<const P: usize> From<ParticipantBalances<P>> for perunwire::Balance {
    fn from(value: ParticipantBalances<P>) -> Self {
        perunwire::Balance {
            balance: value
                .0
                .map(|v| {
                    let mut buf = vec![0u8; 32];
                    v.to_big_endian(&mut buf);
                    buf
                })
                .to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abiencode::{
        self,
        tests::serialize_and_compare,
        types::{Address, Hash},
    };
    use uint::hex::FromHex;

    /* Solidity: get_state_1A2P()
    ```solidity
    function get_state_1A2P() internal pure returns(Channel.State memory) {
        Channel.State memory s;
        s.channelID = "1111";
        s.version = 0x2222;
        s.outcome.assets = new Channel.Asset[](1);
        s.outcome.assets[0].chainID = 0x3333;
        s.outcome.assets[0].holder = 0x5B38Da6a701c568545dCfcB03FcB875f56beddC4;
        s.outcome.balances = new uint256[][](1); // 1 Asset, 2 Participants
        s.outcome.balances[0] = new uint256[](2);
        s.outcome.balances[0][0] = 0x5555;
        s.outcome.balances[0][1] = 0x6666;
        s.appData = "";
        s.isFinal = true;
        return s;
    }
    ```
    */

    fn build_test_state() -> State<1, 2> {
        // Random address from etherscan, do not use!
        let addr = "5B38Da6a701c568545dCfcB03FcB875f56beddC4";
        let addr = Address(<[u8; 20]>::from_hex(addr).unwrap());

        State {
            id: Hash(*b"1111\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"),
            version: 0x2222,
            outcome: Allocation {
                assets: [Asset {
                    chain_id: (0x3333.into()),
                    holder: addr,
                }],
                balances: Balances([ParticipantBalances([0x5555.into(), 0x6666.into()])]),
                locked: [],
            },
            app_data: [],
            is_final: true,
        }
    }

    #[test]
    fn state_1a2p_encode() {
        /*
        ```solidity
        function encode_state_1A2P() public pure returns(bytes memory) {
            return Channel.encodeState(get_state_1A2P());
        }
        ```
        */
        let state = build_test_state();

        let expected = "
            0000000000000000000000000000000000000000000000000000000000000020
            3131313100000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000002222
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000000000000000000000000000000000000000000220
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000060
            00000000000000000000000000000000000000000000000000000000000000c0
            0000000000000000000000000000000000000000000000000000000000000160
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000003333
            0000000000000000000000005b38da6a701c568545dcfcb03fcb875f56beddc4
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000000000000000000000000000000000000000005555
            0000000000000000000000000000000000000000000000000000000000006666
            0000000000000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000000
            ";

        serialize_and_compare(&state, expected)
    }

    #[test]
    fn state_1a2p_hash() {
        /*
        ```solidity
        function hash_state_1A2P(address signer, bytes memory sig) public pure returns(bytes32) {
            Channel.State memory state = get_state_1A2P();
            return Channel.encodeState(state);
        }
        ```
        */

        let state = build_test_state();
        let hash = abiencode::to_hash(&state).unwrap();

        let expected: Hash = Hash(
            <[u8; 32]>::from_hex(
                "e7518ad2414d38370ea5f21f1351eabce47480ab191c984ac12a3aedf70eda3d",
            )
            .unwrap(),
        );

        assert_eq!(hash, expected);
    }

    #[cfg(feature = "secp256k1")]
    #[test]
    fn state_1a2p_sign() {
        use rand::{rngs::StdRng, SeedableRng};
        use uint::hex::ToHex;

        use crate::sig::Signer;

        /*
        ```solidity
        function verify_sig_state_1A2P(address signer, bytes memory sig) public pure {
            Channel.State memory state = get_state_1A2P();
            require(Sig.verify(Channel.encodeState(state), sig, signer), "invalid signature");
        }
        ```
        */

        let state = build_test_state();

        let hash = abiencode::to_hash(&state).unwrap();

        // Do not use that on any real device, this is just for testing.
        let mut rng = StdRng::seed_from_u64(0);
        let signer = Signer::new(&mut rng);

        let sig = signer.sign_eth(hash);

        println!("Signer: 0x{:}", signer.addr.0.encode_hex::<String>());
        println!("Sig: 0x{}", sig.0.encode_hex::<String>());

        // Test against some known good values (constant because we seed the
        // randomness with 0). When changing these make sure that they are
        // accepted by a smart contract.
        assert_eq!(
            signer.addr.0.encode_hex::<String>(),
            "0xa9572220348b1080264e81c0779f77c144790cd6"[2..]
        );
        assert_eq!(
                sig.0.encode_hex::<String>(),
                "0xe274ea53fa64de7338bffbf264dc1f58a81e3660e426d328a2838944cbcc040205353a79da2bf1c67650c14e32e944ae6644c1a7f8f06146f7b6d152c87bdfb11c"[2..]
            );
    }
}
