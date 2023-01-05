use prost::{bytes::BufMut, Message};

use super::{BytesBus, FunderMessage, MessageBus, ParticipantMessage, WatcherMessage};
use crate::perunwire::{envelope, AuthResponseMsg, Envelope};

#[derive(Debug)]
pub struct ProtoBufEncodingLayer<B: BytesBus> {
    pub bus: B,
}

impl<B: BytesBus> MessageBus for ProtoBufEncodingLayer<B> {
    fn send_to_watcher(&self, msg: WatcherMessage) {
        todo!()
    }

    fn send_to_funder(&self, msg: FunderMessage) {
        todo!()
    }

    fn send_to_participants(&self, msg: ParticipantMessage) {
        let wiremsg: envelope::Msg = match msg {
            ParticipantMessage::Auth => envelope::Msg::AuthResponseMsg(AuthResponseMsg {}),
            ParticipantMessage::ChannelProposal(p) => {
                envelope::Msg::LedgerChannelProposalMsg(p.into())
            }
            ParticipantMessage::ProposalAccepted(_) => todo!(),
            ParticipantMessage::ProposalRejected => todo!(),
            ParticipantMessage::ChannelUpdate(_) => todo!(),
            ParticipantMessage::ChannelUpdateAccepted(msg) => {
                envelope::Msg::ChannelUpdateAccMsg(msg.into())
            }
            ParticipantMessage::ChannelUpdateRejected { .. } => todo!(),
        };

        let envelope = Envelope {
            sender: "Alice".as_bytes().to_vec(),  // TODO
            recipient: "Bob".as_bytes().to_vec(), // TODO
            msg: Some(wiremsg),
        };
        // Go-perun writes a u16 for the length (2 bytes), this means we cannot
        // use `encode_length_delimited`, which would write a variable length
        // integer using LEB128.
        let len = envelope.encoded_len();
        // TODO: How should we handle this case? Go-perun seems to just cast to
        // uint16 and throws away the rest (no panic, no error). It is probably
        // best to return an error here, for now we're just panicking in this
        // case (as we're using unwrap below, too).
        assert!(len < (1 << 16));

        let mut buf = Vec::with_capacity(2 + len);
        buf.put_slice(&(len as u16).to_be_bytes());
        envelope.encode(&mut buf).unwrap();

        self.bus.send_to_participants(&buf)
    }
}
