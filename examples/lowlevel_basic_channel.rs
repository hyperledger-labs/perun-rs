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
use std::sync::mpsc;
use tokio;

const PARTICIPANTS: [&'static str; 2] = ["Alice", "Bob"];
const ACCEPT_PROPOSAL: bool = true;
// Note that, due to the example MessageBus implementation, both threads hang if
// neither signs it.
const ALICE_SIGNS: bool = true;
const BOB_SIGNS: bool = true;

/// Message bus representing a tcp connection. For simplicity only using
/// [std::sync::mpsc] and printing the data to stdout.
#[derive(Debug)]
struct Bus {
    participant: usize,
    tx: mpsc::Sender<ParticipantMessage>,
    rx: mpsc::Receiver<ParticipantMessage>,
}

impl MessageBus for &Bus {
    fn send_to_watcher(&self, msg: WatcherMessage) {
        println!("{}->Watcher: {:#?}", PARTICIPANTS[self.participant], msg);
    }

    fn send_to_funder(&self, msg: FunderMessage) {
        println!("{}->Funder: {:#?}", PARTICIPANTS[self.participant], msg);
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

    println!("Bob done");
}

/// Main example code: Set up communication channels and spawn each party in a
/// tokio thread.
#[tokio::main]
async fn main() {
    let bob_to_alice = mpsc::channel();
    let alice_to_bob = mpsc::channel();

    let a_handle = tokio::spawn(alice(Bus {
        participant: 0,
        tx: alice_to_bob.0,
        rx: bob_to_alice.1,
    }));

    let b_handle = tokio::spawn(bob(Bus {
        participant: 1,
        tx: bob_to_alice.0,
        rx: alice_to_bob.1,
    }));

    a_handle.await.unwrap();
    b_handle.await.unwrap();
}
