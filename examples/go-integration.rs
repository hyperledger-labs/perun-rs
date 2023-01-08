use perun::{
    channel::{
        fixed_size_payment::{Allocation, Balances, ParticipantBalances},
        Asset, LedgerChannelProposal,
    },
    perunwire::{envelope, Envelope},
    sig::Signer,
    wire::{BytesBus, ProtoBufEncodingLayer},
    Address, PerunClient,
};
use prost::Message;
use std::{
    cell::RefCell,
    fmt::Debug,
    io::{Read, Write},
    net::TcpStream,
};

const PARTICIPANTS: [&'static str; 2] = ["Alice", "Bob"];

/// Message bus representing a tcp connection. For simplicity only using
/// [std::sync::mpsc] and printing the data to stdout.
#[derive(Debug)]
struct Bus {
    participant: usize,
    stream: RefCell<TcpStream>,
}

impl Bus {
    fn recv_envelope(&self) -> Envelope {
        let buf = self.recv();

        // Decode data (the Encoding Layer currently does not decode, so to
        // print stuff or call methods we have to do it at the moment).
        let msg = Envelope::decode(buf.as_slice()).unwrap();
        println!("Received: {:#?}", msg);

        msg
    }

    fn recv(&self) -> Vec<u8> {
        let mut stream = self.stream.borrow_mut();
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
        self.stream.borrow_mut().write(msg).unwrap();
    }

    fn send_to_funder(&self, msg: &[u8]) {
        println!("{}->Funder: {:?}", PARTICIPANTS[self.participant], msg);
        self.stream.borrow_mut().write(msg).unwrap();
    }

    fn send_to_participants(&self, msg: &[u8]) {
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

fn main() {
    // Some information about the (temporary) blockchain we need, could be hard
    // coded into the application or received by some other means.
    let mut config_stream = TcpStream::connect("127.0.0.1:1338").unwrap();
    let mut eth_holder = [0u8; 20];
    config_stream.read_exact(&mut eth_holder).unwrap();
    let eth_holder = Address(eth_holder);
    drop(config_stream);

    // Networking
    let stream = TcpStream::connect("127.0.0.1:1337").unwrap();
    let stream = RefCell::new(stream);
    let bus = Bus {
        participant: 0,
        stream: stream,
    };

    // Signer, Addresses and Client
    let signer = Signer::new(&mut rand::thread_rng());
    let addr = signer.addr;
    let client = PerunClient::new(ProtoBufEncodingLayer { bus: &bus }, signer);
    client.send_handshake_msg();
    bus.recv_envelope();

    // Create channel proposal (user configuration)
    print_user_interaction!("Proposing channel");
    let init_balance = Balances([ParticipantBalances([100.into(), 100.into()])]);
    let prop = LedgerChannelProposal {
        proposal_id: rand::random(),
        challenge_duration: 100,
        nonce_share: rand::random(),
        init_bals: Allocation::new(
            [Asset {
                chain_id: 1337.into(), // Default chainID when using a SimulatedBackend from go-ethereum
                holder: eth_holder,
            }],
            init_balance,
        ),
        funding_agreement: init_balance,
        participant: addr,
    };
    // Propose new channel and wait for responses
    let mut channel = client.propose_channel(prop);
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
    // anyways). At the moment it is not completely clear how exactly we want to
    // allow sending both types through the same session, we might need changes
    // to the original protobuf definition or use a completely separate port
    // with its own "Envelope"-like message.

    // bus.recv(); // TODO: Uncomment once the Go-side replies
    // bus.recv(); // TODO: Uncomment once the Go-side replies

    print_bold!("Alice done");
}
