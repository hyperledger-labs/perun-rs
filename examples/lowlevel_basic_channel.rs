//! Example/Walkthrough demonstrating the low-level API (subject to change).

use perun::{
    channel::{
        fixed_size_payment::{Allocation, Balances, ParticipantBalances},
        Asset, LedgerChannelProposal,
    },
    sig::Signer,
    wire::{FunderMessage, MessageBus, ParticipantMessage, WatcherMessage},
    PerunClient,
};
use std::{fmt::Debug, sync::mpsc};
use tokio::{self, task::JoinHandle};

const PARTICIPANTS: [&'static str; 2] = ["Alice", "Bob"];
const ACCEPT_PROPOSAL: bool = true;
// Note that, due to the example MessageBus implementation, both threads hang if
// neither signs it.
const ALICE_SIGNS: bool = true;
const BOB_SIGNS: bool = true;

/// For simplicity of the communication channels, the Watcher and Funder are
/// implemented in the same thread in this example.
enum ServiceMsg {
    Watcher(WatcherMessage),
    Funder(FunderMessage),
}

/// Message bus representing a tcp connection. For simplicity only using
/// [std::sync::mpsc] and printing the data to stdout.
#[derive(Debug)]
struct Bus {
    participant: usize,
    tx: mpsc::Sender<ParticipantMessage>,
    rx: mpsc::Receiver<ParticipantMessage>,
    service_tx: mpsc::Sender<ServiceMsg>,
    service_rx: mpsc::Receiver<ServiceMsg>,
}

impl MessageBus for &Bus {
    fn send_to_watcher(&self, msg: WatcherMessage) {
        println!("{}->Watcher: {:#?}", PARTICIPANTS[self.participant], msg);
        self.service_tx.send(ServiceMsg::Watcher(msg)).unwrap();
    }

    fn send_to_funder(&self, msg: FunderMessage) {
        println!("{}->Funder: {:#?}", PARTICIPANTS[self.participant], msg);
        self.service_tx.send(ServiceMsg::Funder(msg)).unwrap();
    }

    fn send_to_participants(&self, msg: ParticipantMessage) {
        println!(
            "{}->{}: {:#?}",
            PARTICIPANTS[self.participant],
            PARTICIPANTS[1 - self.participant],
            msg,
        );
        self.tx.send(msg).unwrap();
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

/// Alice: Proposes new channel.
async fn alice(bus: Bus) {
    let signer = Signer::new(&mut rand::thread_rng());
    let addr = signer.addr;
    let client = PerunClient::new(&bus, signer);

    // Create channel proposal (user configuration)
    print_user_interaction!("Alice proposes a channel");
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
    match bus.rx.recv().unwrap() {
        ParticipantMessage::ProposalAccepted(msg) => {
            channel.participant_accepted(1, msg).unwrap();
        }
        ParticipantMessage::ProposalRejected => {
            println!("Alice done: Received ProposalRejected");
            return;
        }
        _ => panic!("Unexpected message"),
    }

    print_bold!(
        "Both agreed on proposal and nonces, Both sign the initial state and exchange signatures"
    );

    // Go to Phase 2: Signing the initial state
    let mut channel = channel.build().unwrap();

    if ALICE_SIGNS {
        channel.sign().unwrap();
    }
    match bus.rx.recv() {
        Ok(ParticipantMessage::ChannelUpdateAccepted(msg)) => {
            channel.add_signature(msg).unwrap();
        }
        Ok(_) => panic!("Unexpected message"),
        Err(_) => {
            // In reality some kind of timeout
            println!("Alice done: Did not receive Signature from Bob");
            return;
        }
    }

    print_bold!("Alice: Received all signatures, send to watcher/funder");
    let channel = channel.build().unwrap();
    // Wait for Funded and WatchRequestAck messages (content not checked in this
    // example)
    bus.service_rx.recv().unwrap();
    bus.service_rx.recv().unwrap();

    print_bold!("Alice: Received Funded + WatchAck Message => Channel can be used");
    channel.mark_funded();

    println!("Alice done");
}

/// Bob: Reacts to a proposed channel.
async fn bob(bus: Bus) {
    let signer = Signer::new(&mut rand::thread_rng());
    let addr = signer.addr;
    let client = PerunClient::new(&bus, signer);

    // Wait for Channel Proposal, then accept it
    let mut channel = match bus.rx.recv().unwrap() {
        ParticipantMessage::ChannelProposal(prop) => client.handle_proposal(prop),
        _ => panic!("Unexpected message"),
    };
    if ACCEPT_PROPOSAL {
        print_user_interaction!("Bob accepts proposed channel");
        channel.accept(rand::random(), addr).unwrap();
    } else {
        print_user_interaction!("Bob done: rejects proposed channel");
        channel.reject();
        return;
    }

    // Go to Phase 2: Signing the initial state
    let mut channel = channel.build().unwrap();

    if BOB_SIGNS {
        channel.sign().unwrap();
    }
    match bus.rx.recv() {
        Ok(ParticipantMessage::ChannelUpdateAccepted(msg)) => {
            channel.add_signature(msg).unwrap();
        }
        Ok(_) => panic!("Unexpected message"),
        Err(_) => {
            // In reality some kind of timeout
            println!("Bob done: Did not receive Signature from Alice");
            return;
        }
    }

    print_bold!("Bob: Received all signatures, send to watcher/funder");
    let channel = channel.build().unwrap();
    // Wait for Funded and WatchRequestAck messages (content not checked in this
    // example)
    bus.service_rx.recv().unwrap();
    bus.service_rx.recv().unwrap();

    print_bold!("Bob: Received Funded + WatchAck Message => Channel can be used");
    channel.mark_funded();

    println!("Bob done");
}

async fn service(
    participant: usize,
    snd: mpsc::Sender<ServiceMsg>,
    rcv: mpsc::Receiver<ServiceMsg>,
) {
    loop {
        match rcv.recv() {
            Ok(ServiceMsg::Watcher(WatcherMessage::WatchRequest(msg))) => {
                let res = WatcherMessage::WatchRequestAck {
                    id: msg.state.channel_id(),
                };
                println!("Watcher->{}: {:#?}", PARTICIPANTS[participant], res);
                snd.send(ServiceMsg::Watcher(res)).unwrap();
            }
            Ok(ServiceMsg::Watcher(_)) => panic!("Invalid Message"),
            Ok(ServiceMsg::Funder(FunderMessage::FundingRequest(msg))) => {
                let res = FunderMessage::Funded {
                    id: msg.state.channel_id(),
                };
                println!("Funder->{}: {:#?}", PARTICIPANTS[participant], res);
                snd.send(ServiceMsg::Funder(res)).unwrap();
            }
            Ok(ServiceMsg::Funder(_)) => panic!("Invalid Message"),
            Err(_) => {
                println!("Service done: Channel was closed");
                return;
            }
        }
    }
}

fn setup_participant(
    participant: usize,
    snd: mpsc::Sender<ParticipantMessage>,
    rcv: mpsc::Receiver<ParticipantMessage>,
) -> (Bus, JoinHandle<()>) {
    let (participant_snd, participant_rcv) = mpsc::channel();
    let (service_snd, service_rcv) = mpsc::channel();
    let bus = Bus {
        participant,
        rx: rcv,
        tx: snd,
        service_tx: service_snd,
        service_rx: participant_rcv,
    };

    let service_handle = tokio::spawn(service(participant, participant_snd, service_rcv));
    (bus, service_handle)
}

/// Main example code: Set up communication channels and spawn each party in a
/// tokio thread.
#[tokio::main]
async fn main() {
    // Alice <-> Bob
    let (alice_snd, alice_rcv) = mpsc::channel();
    let (bob_snd, bob_rcv) = mpsc::channel();

    let (alice_bus, as_handle) = setup_participant(0, alice_snd, bob_rcv);
    let a_handle = tokio::spawn(alice(alice_bus));

    let (bob_bus, bs_handle) = setup_participant(1, bob_snd, alice_rcv);
    let b_handle = tokio::spawn(bob(bob_bus));

    a_handle.await.unwrap();
    b_handle.await.unwrap();
    as_handle.await.unwrap();
    bs_handle.await.unwrap();
}
