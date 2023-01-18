use core::cell::RefCell;
use perun::{
    channel::{
        fixed_size_payment::{Allocation, Balances, ParticipantBalances},
        Asset, ChannelUpdate, LedgerChannelProposal,
    },
    perunwire::{self, envelope},
    sig::Signer,
    wire::{BytesBus, Identity, MessageBus, ProtoBufEncodingLayer},
    Address, PerunClient,
};
use prost::Message;
use rand::{CryptoRng, Rng};
use std::{
    fmt::Debug,
    io::{Read, Write},
    net::TcpStream,
};

#[cfg(not(any(feature = "std", feature = "nostd-example")))]
compile_error!("When running this example in no_std add the feature flag 'nostd-example'");

const PARTICIPANTS: [&'static str; 2] = ["Alice", "Bob"];
const NORMAL_CLOSE: bool = false;
const SEND_DISPUTE: bool = true;

/// Message bus representing a tcp connection. For simplicity only using
/// [std::sync::mpsc] and printing the data to stdout.
#[derive(Debug)]
struct Bus {
    participant: usize,
    stream: RefCell<TcpStream>,
    remote_stream: RefCell<TcpStream>,
}

impl Bus {
    fn recv_envelope(&self) -> perunwire::Envelope {
        Self::recv_to(&self.stream)
    }

    fn recv_message(&self) -> perunwire::Message {
        Self::recv_to(&self.remote_stream)
    }

    fn recv_to<T: Message + Default>(stream: &RefCell<TcpStream>) -> T {
        let buf = Self::recv(stream);

        // Decode data (the Encoding Layer currently does not decode, so to
        // print stuff or call methods we have to do it at the moment).
        let msg = T::decode(buf.as_slice()).unwrap();
        println!("Received: {:#?}", msg);

        msg
    }

    fn recv(stream: &RefCell<TcpStream>) -> Vec<u8> {
        let mut stream = stream.borrow_mut();
        // big endian u16 for length in bytes
        let mut buf = [0u8; 2];
        stream.read_exact(&mut buf).unwrap();
        let len = u16::from_be_bytes(buf);
        // Protobuf encoded data
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).unwrap();
        buf
    }
}

impl BytesBus for &Bus {
    fn send_to_watcher(&self, msg: &[u8]) {
        println!("{}->Watcher: {:?}", PARTICIPANTS[self.participant], msg);
        self.remote_stream.borrow_mut().write(msg).unwrap();
    }

    fn send_to_funder(&self, msg: &[u8]) {
        println!("{}->Funder: {:?}", PARTICIPANTS[self.participant], msg);
        self.remote_stream.borrow_mut().write(msg).unwrap();
    }

    fn send_to_participant(&self, _: &Identity, _: &Identity, msg: &[u8]) {
        println!(
            "{}->{}: {:?}",
            PARTICIPANTS[self.participant],
            PARTICIPANTS[1 - self.participant],
            msg,
        );
        self.stream.borrow_mut().write(msg).unwrap();
    }
}

/// Helper macro to print significant places in the protocol.
macro_rules! print_bold {
    ($($arg:tt)*) => {
        print!("\x1b[1m");
        print!($($arg)*);
        println!("\x1b[0m");
    };
}

/// Helper macro to print points where the user/application has to interact.
macro_rules! print_user_interaction {
    ($($arg:tt)*) => {
        print!("\x1b[1;34m");
        print!($($arg)*);
        println!("\x1b[0m");
    };
}

#[cfg(feature = "std")]
fn get_rng() -> impl Rng + CryptoRng {
    rand::thread_rng()
}
#[cfg(not(feature = "std"))]
fn get_rng() -> impl Rng + CryptoRng {
    use rand::SeedableRng;

    rand::rngs::StdRng::seed_from_u64(0)
}

fn main() {
    let mut rng = get_rng();

    // Some information about the (temporary) blockchain we need, could be hard
    // coded into the application or received by some other means.
    let mut config_stream = TcpStream::connect("127.0.0.1:1339").unwrap();
    let mut buf = [0u8; 20];
    config_stream.read_exact(&mut buf).unwrap();
    let eth_holder = Address(buf);
    config_stream.read_exact(&mut buf).unwrap();
    let withdraw_receiver = Address(buf);
    drop(config_stream);

    // Networking
    let bus = Bus {
        participant: 0,
        stream: RefCell::new(TcpStream::connect("127.0.0.1:1337").unwrap()),
        remote_stream: RefCell::new(TcpStream::connect("127.0.0.1:1338").unwrap()),
    };

    let peers = vec!["Alice".as_bytes().to_vec(), "Bob".as_bytes().to_vec()];

    // Signer, Addresses and Client
    let signer = Signer::new(&mut rng);
    let addr = signer.address();
    let client = PerunClient::new(ProtoBufEncodingLayer { bus: &bus }, signer);
    client.send_handshake_msg(&peers[0], &peers[1]);
    bus.recv_envelope();

    // Create channel proposal (user configuration)
    print_user_interaction!("Proposing channel");
    let init_balance = Balances([ParticipantBalances([100.into(), 100.into()])]);
    let prop = LedgerChannelProposal {
        proposal_id: rng.gen(),
        challenge_duration: 25,
        nonce_share: rng.gen(),
        init_bals: Allocation::new(
            [Asset {
                chain_id: 1337.into(), // Default chainID when using a SimulatedBackend from go-ethereum
                holder: eth_holder,
            }],
            init_balance,
        ),
        funding_agreement: init_balance,
        participant: addr,
        peers,
    };
    // Propose new channel and wait for responses
    let mut channel = client.propose_channel(prop, withdraw_receiver).unwrap();
    match bus.recv_envelope().msg {
        Some(envelope::Msg::LedgerChannelProposalAccMsg(msg)) => channel
            .participant_accepted(1, msg.try_into().unwrap())
            .unwrap(),
        Some(envelope::Msg::ChannelProposalRejMsg(_)) => {
            print_bold!("Alice done: Received ProposalRejected");
            return;
        }
        Some(_) => panic!("Unexpected message"),
        None => panic!("Envelope did not contain a msg"),
    }
    print_bold!(
        "Both agreed on proposal and nonces, Both sign the initial state and exchange signatures"
    );

    // Go to Phase 2: Signing the initial state
    let mut channel = channel.build().unwrap();
    channel.sign().unwrap();
    match bus.recv_envelope().msg {
        Some(envelope::Msg::ChannelUpdateAccMsg(msg)) => {
            channel.add_signature(msg.try_into().unwrap()).unwrap()
        }
        Some(envelope::Msg::ChannelUpdateRejMsg(_)) => {
            print_bold!("Alice done: Did not receive Signature from Bob");
            return;
        }
        Some(_) => panic!("Unexpected message"),
        None => panic!("Envelope did not contain a msg"),
    }

    print_bold!("Alice: Received all signatures, send to watcher/funder");

    let channel = channel.build().unwrap();
    // Receive acknowledgements (currently not checked but we have to read them
    // anyways).
    bus.recv_message();
    bus.recv_message();

    let mut channel = channel.mark_funded();

    print_user_interaction!("Alice: Propose Update");
    let mut new_state = channel.state().make_next_state();
    new_state.outcome.balances.0[0].0[0] += 10.into();
    new_state.outcome.balances.0[0].0[1] -= 10.into();
    let update = channel.update(new_state).unwrap();
    handle_update_response(&bus, update);

    if NORMAL_CLOSE {
        print_user_interaction!("Alice: Propose Normal close");
        let mut new_state = channel.state().make_next_state();
        // Propose a normal closure
        new_state.is_final = true;
        let update = channel.update(new_state).unwrap();
        handle_update_response(&bus, update);
    }

    if SEND_DISPUTE {
        print_user_interaction!("Alice: Send StartDispute Message (force-close)");
        channel.force_close().unwrap();
        bus.recv_message();
    }

    print_bold!("Alice done");
}

fn handle_update_response<'a, 'b, B: MessageBus>(bus: &Bus, mut update: ChannelUpdate<'a, 'b, B>) {
    match bus.recv_envelope().msg {
        Some(envelope::Msg::ChannelUpdateAccMsg(msg)) => {
            update
                .participant_accepted(1, msg.try_into().unwrap())
                .unwrap();
            update.apply().unwrap();
        }
        Some(envelope::Msg::ChannelUpdateRejMsg(_)) => {
            print_bold!("Aborting update");
            drop(update);
        }
        Some(_) => panic!("Unexpected message"),
        None => panic!("Envelope did not contain a msg"),
    }
}
