use super::{fixed_size_payment, PartID};
use crate::{
    abiencode::{
        self,
        types::{Address, Hash, Signature},
    },
    sig,
    wire::{MessageBus, ParticipantMessage},
    PerunClient,
};

type State = fixed_size_payment::State<1, 2>;
type Params = fixed_size_payment::Params<2>;

#[derive(Debug, Clone, Copy)]
pub struct LedgerChannelUpdateAccepted {
    channel: Hash,
    version: u64,
    sig: Signature,
}

#[derive(Debug)]
pub enum SignError {
    AbiEncodeError(abiencode::Error),
    AlreadySigned,
}
impl From<abiencode::Error> for SignError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}

#[derive(Debug)]
pub enum AddSignatureError {
    AbiEncodeError(abiencode::Error),
    RecoveryFailed(sig::Error),
    AlreadySigned,
    InvalidSignature(Address),
    InvalidChannelID,
    InvalidVersionNumber,
}
impl From<abiencode::Error> for AddSignatureError {
    fn from(e: abiencode::Error) -> Self {
        Self::AbiEncodeError(e)
    }
}
impl From<sig::Error> for AddSignatureError {
    fn from(e: sig::Error) -> Self {
        Self::RecoveryFailed(e)
    }
}

#[derive(Debug)]
pub struct AgreedUponChannel<'a, B: MessageBus> {
    part_id: PartID,
    client: &'a PerunClient<B>,
    init_state: State,
    params: Params,
    signatures: [Option<Signature>; 2],
}

impl<'a, B: MessageBus> AgreedUponChannel<'a, B> {
    pub(super) fn new(
        client: &'a PerunClient<B>,
        part_id: PartID,
        init_state: State,
        params: Params,
    ) -> Self {
        AgreedUponChannel {
            part_id,
            client,
            init_state,
            params,
            signatures: [None; 2],
        }
    }

    pub fn sign(&mut self) -> Result<(), SignError> {
        match self.signatures[self.part_id] {
            Some(_) => Err(SignError::AlreadySigned),
            None => {
                // Sign the initial state
                let hash = abiencode::to_hash(&self.init_state)?;
                let sig = self.client.signer.sign_eth(hash);
                // Add signature to the proposed channel
                self.signatures[self.part_id] = Some(sig);
                // Send to other participants
                self.client
                    .bus
                    .send_to_participants(ParticipantMessage::ChannelUpdateAccepted(
                        LedgerChannelUpdateAccepted {
                            channel: self.init_state.channel_id(),
                            version: self.init_state.version(),
                            sig: sig,
                        },
                    ));
                Ok(())
            }
        }
    }

    pub fn add_signature(
        &mut self,
        msg: LedgerChannelUpdateAccepted,
    ) -> Result<(), AddSignatureError> {
        if msg.channel != self.init_state.channel_id() {
            return Err(AddSignatureError::InvalidChannelID);
        }
        if msg.version != 0 {
            return Err(AddSignatureError::InvalidVersionNumber);
        }

        let hash = abiencode::to_hash(&self.init_state)?;
        let signer = self.client.signer.recover_signer(hash, msg.sig)?;

        // Verify signature is comming from a valid participant.
        let part_id: PartID = match self
            .params
            .participants
            .iter()
            .enumerate()
            .filter(|(_, &addr)| addr == signer)
            .next()
        {
            Some((part_id, _)) => part_id,
            None => return Err(AddSignatureError::InvalidSignature(signer)),
        };

        match self.signatures[part_id] {
            Some(_) => Err(AddSignatureError::AlreadySigned),
            None => {
                self.signatures[part_id] = Some(msg.sig);
                Ok(())
            }
        }
    }
}
