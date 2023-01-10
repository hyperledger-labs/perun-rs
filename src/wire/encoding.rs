use prost::{bytes::BufMut, EncodeError};

use super::{BytesBus, FunderMessage, MessageBus, ParticipantMessage, WatcherMessage};
use crate::perunwire::{envelope, message, AuthResponseMsg, Envelope, Message};

#[derive(Debug)]
pub struct ProtoBufEncodingLayer<B: BytesBus> {
    pub bus: B,
}

impl<B: BytesBus> ProtoBufEncodingLayer<B> {
    fn encode<T: prost::Message>(msg: T) -> Result<Vec<u8>, EncodeError> {
        // Go-perun writes a u16 for the length (2 bytes), this means we cannot
        // use `encode_length_delimited`, which would write a variable length
        // integer using LEB128.
        let len = msg.encoded_len();
        // TODO: How should we handle this case? Go-perun seems to just cast to
        // uint16 and throws away the rest (no panic, no error). It is probably
        // best to return an error here, for now we're just panicking in this
        // case (as we're using unwrap below, too).
        assert!(len < (1 << 16));

        let mut buf = Vec::with_capacity(2 + len);
        buf.put_slice(&(len as u16).to_be_bytes());
        msg.encode(&mut buf)?;
        Ok(buf)
    }
}

impl<B: BytesBus> MessageBus for ProtoBufEncodingLayer<B> {
    fn send_to_watcher(&self, msg: WatcherMessage) {
        let wiremsg: message::Msg = match msg {
            WatcherMessage::WatchRequest(msg) => message::Msg::WatchRequest(msg.into()),
            WatcherMessage::Update(_) => todo!(),
            WatcherMessage::Ack { .. } => todo!(),
            WatcherMessage::StartDispute(_) => todo!(),
            WatcherMessage::DisputeAck { .. } => todo!(),
            WatcherMessage::DisputeNotification { .. } => todo!(),
        };
        let envelope = Message { msg: Some(wiremsg) };

        let buf = Self::encode(envelope).unwrap();
        self.bus.send_to_watcher(&buf);
    }

    fn send_to_funder(&self, msg: FunderMessage) {
        let wiremsg: message::Msg = match msg {
            FunderMessage::FundingRequest(msg) => message::Msg::FundingRequest(msg.into()),
            FunderMessage::Funded { .. } => todo!(),
        };
        let envelope = Message { msg: Some(wiremsg) };

        let buf = Self::encode(envelope).unwrap();
        self.bus.send_to_watcher(&buf);
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

        let buf = Self::encode(envelope).unwrap();
        self.bus.send_to_participants(&buf);
    }
}
