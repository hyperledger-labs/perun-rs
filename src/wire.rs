mod encoding;

use alloc::vec::Vec;
pub use encoding::ProtoBufEncodingLayer;

use crate::{
    channel::{PartIdx, Peers},
    messages::{FunderRequestMessage, ParticipantMessage, WatcherRequestMessage},
};

pub type Identity = Vec<u8>;

pub trait BytesBus {
    fn send_to_watcher(&self, msg: &[u8]);
    fn send_to_funder(&self, msg: &[u8]);
    fn send_to_participant(&self, sender: &Identity, recipient: &Identity, msg: &[u8]);
}

/// Low-Level abstraction over the network configuration.
///
/// Might be moved into a byte based MessageBus or behind a `unstable` feature
/// flag.
pub trait MessageBus {
    fn send_to_watcher(&self, msg: WatcherRequestMessage);
    fn send_to_funder(&self, msg: FunderRequestMessage);
    fn send_to_participant(&self, sender: &Identity, recipient: &Identity, msg: ParticipantMessage);
}

pub trait BroadcastMessageBus: MessageBus {
    fn broadcast_to_participants(&self, part_idx: PartIdx, peers: &Peers, msg: ParticipantMessage);
}

impl<B: MessageBus> BroadcastMessageBus for B {
    fn broadcast_to_participants(&self, part_idx: PartIdx, peers: &Peers, msg: ParticipantMessage) {
        let sender = &peers[part_idx];
        for (i, peer) in peers.iter().enumerate() {
            if i == part_idx {
                continue;
            }

            self.send_to_participant(sender, peer, msg.clone());
        }
    }
}
