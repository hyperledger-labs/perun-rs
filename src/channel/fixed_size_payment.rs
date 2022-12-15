//! Rust representations of the on-chain Solidity data types with fixed amount
//! of Participants and Assets.
//!
//! These types can be useful when the number of Participants and Assets are
//! known at compile time or we don't have heap allocation.

use super::Asset;
use crate::abiencode::{
    self, as_bytes, as_dyn_array,
    types::{Address, Hash, U256},
};
use serde::Serialize;

/// Parameters for this channel, exchanged during channel proposal and sent
/// on-chain during a dispute.
#[derive(Serialize, Debug, Copy, Clone)]
pub struct Params<const P: usize> {
    pub challenge_duration: u64,
    pub nonce: U256,
    #[serde(with = "as_dyn_array")]
    pub participants: [Address; P],
    #[serde(with = "as_bytes")]
    pub app: [u8; 0],
    pub ledger_channel: bool,
    pub virtual_channel: bool,
}

/// Stores the complete state of a channel.
#[derive(Serialize, Debug)]
pub struct State<const A: usize, const P: usize> {
    id: Hash,
    version: u64,
    outcome: Allocation<A, P>,
    #[serde(with = "as_bytes")]
    app_data: [u8; 0],
    is_final: bool,
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
        let id = abiencode::to_hash(&params)?;
        Ok(State {
            id,
            version: 0,
            outcome: init_bals,
            app_data: [],
            is_final: false,
        })
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

/// Stores which participant has how much of each asset.
#[derive(Serialize, Debug, Copy, Clone)]
pub struct Allocation<const A: usize, const P: usize> {
    #[serde(with = "as_dyn_array")]
    pub assets: [Asset; A],
    pub balances: Balances<A, P>,
    #[serde(with = "as_dyn_array")]
    locked: [(); 0], // Only needed for encoding
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abiencode::{
        self,
        tests::serialize_and_compare,
        types::{Address, Hash},
    };
    use uint::hex::{FromHex, ToHex};

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
