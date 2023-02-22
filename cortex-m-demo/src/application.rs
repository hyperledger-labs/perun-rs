//! State machine for the application logic. We need to do networking during the
//! setup and application logic, so we need to do it while the main loop is
//! running. Additionally, some steps cannot finish immediately. Since we
//! (currently at least) don't have an async runtime in this demo the easiest
//! way to do this is to have a state machine for the setup and application
//! logic, too, which is contained in this module.

use core::cell::RefCell;

use alloc::vec::Vec;
use perun::{
    channel::{
        fixed_size_payment::{Allocation, Balances, ParticipantBalances},
        Asset,
    },
    messages::{ConversionError, LedgerChannelProposal, ParticipantMessage},
    perunwire::{envelope::Msg, Envelope},
    wire::ProtoBufEncodingLayer,
    Address, Hash, InvalidProposal, PerunClient,
};
use prost::{DecodeError, Message};
use rand::{rngs::StdRng, Rng};
use rand_core::RngCore;
use smoltcp::{
    iface::{Interface, SocketHandle},
    phy::Device,
    socket::TcpSocket,
    wire::IpAddress,
};

use crate::{
    bus::Bus,
    channel::{self, Channel},
    LedOutputPin,
};

/// Configuration for the demo: Peers and where to find the
/// participant/watcher/funder.
pub struct Config {
    pub config_server: (IpAddress, u16),
    pub other_participant: (IpAddress, u16),
    pub participants: [&'static str; 2],
}

/// State machine for the demo logic: Fetch information about the blockchain
/// from the go-side, create TCP socket with participant and propose channel.
pub struct Application<'cl: 'ch, 'ch, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    state: ApplicationState<'cl, 'ch, DeviceT>,
    iface: &'cl RefCell<Interface<'cl, DeviceT>>,
    handle: SocketHandle,
    config: Config,
    rng: StdRng,
    client: &'cl PerunClient<ProtoBufEncodingLayer<Bus<'cl, DeviceT>>>,
    addr: Address,

    // TODO: Remove this debugging LED
    green_led: LedOutputPin<0>,
}

/// Enum to represent the states the Application can be in.
enum ApplicationState<'cl: 'ch, 'ch, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    InitialState,
    ConnectingToConfigDealer,
    Configured {
        eth_holder: Address,
        withdraw_receiver: Address,
    },
    Connecting {
        eth_holder: Address,
        withdraw_receiver: Address,
    },
    Handshake {
        eth_holder: Address,
        withdraw_receiver: Address,
    },
    Active {
        channel: Channel<'cl, 'ch, ProtoBufEncodingLayer<Bus<'cl, DeviceT>>>,
    },
}

pub enum Error {
    Network(smoltcp::Error),
    InvalidProposal(InvalidProposal),
    ProstDecode(DecodeError),
    EnvelopeHasNoMsg,
    UnexpectedMsg,
    ConversionError(ConversionError),
    ChannelError(channel::Error),
}

impl From<smoltcp::Error> for Error {
    fn from(e: smoltcp::Error) -> Self {
        Self::Network(e)
    }
}
impl From<InvalidProposal> for Error {
    fn from(e: InvalidProposal) -> Self {
        Self::InvalidProposal(e)
    }
}
impl From<prost::DecodeError> for Error {
    fn from(e: prost::DecodeError) -> Self {
        Self::ProstDecode(e)
    }
}
impl From<ConversionError> for Error {
    fn from(e: ConversionError) -> Self {
        Self::ConversionError(e)
    }
}
impl From<channel::Error> for Error {
    fn from(e: channel::Error) -> Self {
        Self::ChannelError(e)
    }
}

impl<'cl: 'ch, 'ch, DeviceT> Application<'cl, 'ch, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    pub fn new(
        handle: SocketHandle,
        config: Config,
        rng: StdRng,
        addr: Address,
        client: &'cl PerunClient<ProtoBufEncodingLayer<Bus<'cl, DeviceT>>>,
        green_led: LedOutputPin<0>,
        iface: &'cl RefCell<Interface<'cl, DeviceT>>,
    ) -> Self {
        Self {
            state: ApplicationState::InitialState,
            handle,
            config,
            rng,
            client,
            addr,
            green_led,
            iface,
        }
    }

    fn connect_config_dealer(&mut self) -> Result<(), Error> {
        // Connect to the server IP. Does not wait for the handshake to finish.
        let mut iface = self.iface.borrow_mut();
        let (socket, cx) = iface.get_socket_and_context::<TcpSocket>(self.handle);
        socket.connect(
            cx,
            self.config.config_server,
            (IpAddress::Unspecified, self.get_ethemeral_port()),
        )?;

        self.state = ApplicationState::ConnectingToConfigDealer;
        Ok(())
    }

    fn wait_connected_and_read_config(&mut self) -> Result<(), Error> {
        let mut iface = self.iface.borrow_mut();
        let socket = iface.get_socket::<TcpSocket>(self.handle);
        if socket.is_active() && socket.can_recv() {
            // Try reading from the socket. Returns Err if there is something
            // wrong with the socket (unexpected tcp state). Returns None if not
            // enough bytes are available (we only received partial data for
            // some reason).
            if let Some((eth_holder, withdraw_receiver)) = socket.recv(|x| {
                if x.len() >= 40 {
                    let eth_holder = Address(x[..20].try_into().unwrap());
                    let withdraw_receiver = Address(x[20..40].try_into().unwrap());
                    (40, Some((eth_holder, withdraw_receiver)))
                } else {
                    (0, None)
                }
            })? {
                self.state = ApplicationState::Configured {
                    eth_holder,
                    withdraw_receiver,
                };
                socket.close();
            }
        }
        Ok(())
    }

    // We could do this while setting up the connection for reading the config,
    // but doing it separately means we have one less active connection at the
    // same time.
    fn connect(&mut self, eth_holder: Address, withdraw_receiver: Address) -> Result<(), Error> {
        let mut iface = self.iface.borrow_mut();
        let (socket, cx) = iface.get_socket_and_context::<TcpSocket>(self.handle);
        if socket.is_active() {
            // Only transition to the next state if the socket is free (i.e.
            // closed) and avaliable. Alternatively we could use
            // `socket.abort()`, resulting in a non-graceful shutdown but
            // slightly faster transition times. One downside of doing it this
            // way is that a malicious config dealer could DoS us by never
            // sending a Fin, but since the config dealer is only necessary for
            // the demo (which can't use hard-coded addresses) this is not a
            // problem.
            return Ok(());
        }
        socket.connect(
            cx,
            self.config.other_participant,
            (IpAddress::Unspecified, self.get_ethemeral_port()),
        )?;

        self.state = ApplicationState::Connecting {
            eth_holder,
            withdraw_receiver,
        };
        Ok(())
    }

    fn wait_connected_and_send_handshake(
        &mut self,
        eth_holder: Address,
        withdraw_receiver: Address,
    ) -> Result<(), Error> {
        let mut iface = self.iface.borrow_mut();
        let socket = iface.get_socket::<TcpSocket>(self.handle);
        if socket.is_active() && socket.may_recv() && socket.may_send() {
            // propose_channel neeeds to be able to borrow the interface to send
            // things on the network. Because of this we need to drop the
            // interface first. Alternatively we could have moved
            // propose_channel to a new state or restructured this function to
            // automatically drop it before calling propose_channel.
            drop(socket);
            drop(iface);

            // Handshake
            let peers: Vec<Vec<u8>> = self
                .config
                .participants
                .map(|p| p.as_bytes().to_vec())
                .into();
            self.client.send_handshake_msg(&peers[0], &peers[1]);

            self.state = ApplicationState::Handshake {
                eth_holder,
                withdraw_receiver,
            }
        }
        Ok(())
    }

    fn try_recv_envelope(&mut self) -> Result<Option<Envelope>, Error> {
        let mut iface = self.iface.borrow_mut();
        let socket = iface.get_socket::<TcpSocket>(self.handle);

        // Only receive complete packets
        let env = socket.recv(|x| {
            // For simplicity of the state machine we're only processing
            // complete packets when they arrive, even though the go-side
            // currently sends them in two fragments for some reason. This
            // packet inspection allows us to look into the rx buffer to see if
            // we have an entire packet, thus allowing us to process it
            // atomically so we don't need an extra half-received state.
            if x.len() < 2 {
                return (0, None);
            }
            let length = u16::from_be_bytes(x[..2].try_into().unwrap());
            let length: usize = length.into();
            if x.len() < 2 + length {
                return (0, None);
            }
            let env = Envelope::decode(&x[2..2 + length]);
            (2 + length, Some(env))
        })?;

        match env {
            Some(Ok(e)) => Ok(Some(e)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    fn wait_handshake_and_propose_channel(
        &mut self,
        eth_holder: Address,
        withdraw_receiver: Address,
    ) -> Result<(), Error> {
        // Only continue if we have a complete package and there was no decoding
        // error.
        let env = match self.try_recv_envelope()? {
            Some(env) => env,
            None => return Ok(()),
        };

        // Make sure it is what we expect, if it is propose a channel. Note that
        // we currently don't check the envelopes receiver and sender for
        // simplicity.
        match env.msg {
            Some(Msg::AuthResponseMsg(_)) => self.propose_channel(eth_holder, withdraw_receiver),
            None => Ok(()),
            _ => Ok(()),
        }
    }

    fn propose_channel(
        &mut self,
        eth_holder: Address,
        withdraw_receiver: Address,
    ) -> Result<(), Error> {
        // Channel Proposal
        let init_balance = Balances([ParticipantBalances([100.into(), 100.into()])]);
        let peers = self
            .config
            .participants
            .map(|p| p.as_bytes().to_vec())
            .into();
        let prop = LedgerChannelProposal {
            proposal_id: self.rng.gen(),
            challenge_duration: 25,
            nonce_share: self.rng.gen(),
            init_bals: Allocation::new(
                [Asset {
                    chain_id: 1337.into(), // Default chainID when using a SimulatedBackend from go-ethereum or Ganache
                    holder: eth_holder,
                }],
                init_balance,
            ),
            funding_agreement: init_balance,
            participant: self.addr,
            peers,
        };
        let channel = self.client.propose_channel(prop, withdraw_receiver)?;
        // Setup sub-state-machine for handling the channel
        let channel = Channel::new(channel);
        self.state = ApplicationState::Active { channel };
        Ok(())
    }

    fn try_recv_participant_msg(&mut self) -> Result<Option<ParticipantMessage>, Error> {
        let env = match self.try_recv_envelope()? {
            Some(env) => env,
            None => return Ok(None),
        };
        let msg = match env.msg {
            Some(m) => m,
            None => return Err(Error::EnvelopeHasNoMsg),
        };
        let msg = match msg {
            Msg::PingMsg(_) => unimplemented!(),
            Msg::PongMsg(_) => unimplemented!(),
            Msg::ShutdownMsg(_) => unimplemented!(),
            Msg::AuthResponseMsg(_) => return Err(Error::UnexpectedMsg),
            Msg::LedgerChannelProposalMsg(_) => return Err(Error::UnexpectedMsg), // Possible in the library but this Application does not support incoming requests.
            Msg::LedgerChannelProposalAccMsg(m) => {
                ParticipantMessage::ProposalAccepted(m.try_into()?)
            }
            Msg::SubChannelProposalMsg(m) => unimplemented!(),
            Msg::SubChannelProposalAccMsg(_) => unimplemented!(),
            Msg::VirtualChannelProposalMsg(_) => unimplemented!(),
            Msg::VirtualChannelProposalAccMsg(_) => unimplemented!(),
            Msg::ChannelProposalRejMsg(m) => ParticipantMessage::ProposalRejected {
                id: Hash(m.proposal_id.try_into().unwrap()),
                reason: m.reason,
            },
            Msg::ChannelUpdateMsg(m) => ParticipantMessage::ChannelUpdate(m.try_into()?),
            Msg::VirtualChannelFundingProposalMsg(_) => unimplemented!(),
            Msg::VirtualChannelSettlementProposalMsg(_) => unimplemented!(),
            Msg::ChannelUpdateAccMsg(m) => ParticipantMessage::ChannelUpdateAccepted(m.try_into()?),
            Msg::ChannelUpdateRejMsg(m) => ParticipantMessage::ChannelUpdateRejected {
                id: Hash(m.channel_id.try_into().unwrap()),
                version: m.version,
                reason: m.reason,
            },
            Msg::ChannelSyncMsg(_) => unimplemented!(),
        };
        Ok(Some(msg))
    }

    fn forward_messages_to_channel(&mut self) -> Result<(), Error> {
        // First try receiving all the possible messages
        let participant_msg = self.try_recv_participant_msg()?;

        // Now get the (mutable) channel object so we don't get issues with mutability.
        let channel = match self.state {
            ApplicationState::Active { ref mut channel } => channel,
            _ => unreachable!("This function is only called when in Active"),
        };

        // Apply messages
        match participant_msg {
            Some(msg) => channel.process_participant_msg(msg)?,
            None => {}
        }
        Ok(())
    }

    // echo service to test sending and receiving of data. This echo service
    // will break if the other side does not read from the socket in time.
    // Since this is only intended for testing it should be fine. If it
    // would be a problem we could query the amount of available rx and tx
    // buffer space and only read then write that amount to not panic at one
    // of the unwraps below.
    pub fn poll(&mut self) -> Result<(), Error> {
        match self.state {
            ApplicationState::InitialState => self.connect_config_dealer(),
            ApplicationState::ConnectingToConfigDealer => self.wait_connected_and_read_config(),
            ApplicationState::Configured {
                eth_holder,
                withdraw_receiver,
            } => self.connect(eth_holder, withdraw_receiver),
            ApplicationState::Connecting {
                eth_holder,
                withdraw_receiver,
            } => self.wait_connected_and_send_handshake(eth_holder, withdraw_receiver),
            ApplicationState::Handshake {
                eth_holder,
                withdraw_receiver,
            } => self.wait_handshake_and_propose_channel(eth_holder, withdraw_receiver),
            ApplicationState::Active { .. } => self.forward_messages_to_channel(),
        }
    }

    fn get_ethemeral_port(&mut self) -> u16 {
        const MIN: u16 = 49152;
        const MAX: u16 = 65535;
        // Note: This is not evenly distributed but sufficient for what we need.
        MIN + (self.rng.next_u32() as u16) % (MAX - MIN)
    }
}
