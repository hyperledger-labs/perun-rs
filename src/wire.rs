mod encoding;

use core::fmt::Debug;

pub use encoding::ProtoBufEncodingLayer;

use crate::messages::{FunderRequestMessage, ParticipantMessage, WatcherRequestMessage};

pub trait BytesBus: Debug {
    fn send_to_watcher(&self, msg: &[u8]);
    fn send_to_funder(&self, msg: &[u8]);
    fn send_to_participants(&self, msg: &[u8]);
}

/// Low-Level abstraction over the network configuration.
///
/// Might be moved into a byte based MessageBus or behind a `unstable` feature
/// flag.
pub trait MessageBus: Debug {
    fn send_to_watcher(&self, msg: WatcherRequestMessage);
    fn send_to_funder(&self, msg: FunderRequestMessage);
    fn send_to_participants(&self, msg: ParticipantMessage);
}
