use perun::{
    channel::{
        fixed_size_payment::{Allocation, Balances, ParticipantBalances},
        Asset, LedgerChannelProposal,
    },
    perunwire::{envelope, Envelope},
    sig::Signer,
    wire::{BytesBus, ProtoBufEncodingLayer},
    PerunClient,
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
        let mut stream = self.stream.borrow_mut();
        // big endian u16 for length in bytes
        let mut buf = [0u8; 2];
        stream.read_exact(&mut buf).unwrap();
        let len = u16::from_be_bytes(buf);
        // Protobuf encoded data
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).unwrap();

        // Decode data (the Encoding Layer currently does not decode, so to
        // print stuff or call methods we have to do it at the moment).
        let msg = Envelope::decode(buf.as_slice()).unwrap();
        println!("Received: {:#?}", msg);

        msg
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
                chain_id: 1.into(),
                holder: rand::random(),
            }],
            init_balance,
        ),
        funding_agreement: init_balance,
        participant: addr,
    };
    // Propose new channel and wait for responses
    let mut channel = client.propose_channel(prop);
    let envelope = bus.recv_envelope();
    match envelope.msg {
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

    print_bold!("Alice done");
}
