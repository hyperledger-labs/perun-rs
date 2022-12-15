//! Example/Walkthrough demonstrating the low-level API (subject to change).
//!
//! Required feature flags:
//! - std
//! - secp256k1

use perun::{
    channel::{
        fixed_size_payment::{Allocation, Balances, ParticipantBalances},
        Asset, LedgerChannelProposal,
    },
    wire::{FunderMessage, MessageBus, ParticipantMessage, WatcherMessage},
    Address, PerunClient,
};
use secp256k1::{All, Secp256k1, SecretKey};
use std::sync::mpsc;
use tokio;

const PARTICIPANTS: [&'static str; 2] = ["Alice", "Bob"];
const ACCEPT_PROPOSAL: bool = true;

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
        println!("{}->Watcher: {:?}", PARTICIPANTS[self.participant], msg);
    }

    fn send_to_funder(&self, msg: FunderMessage) {
        println!("{}->Funder: {:?}", PARTICIPANTS[self.participant], msg);
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

/// Small helper function to setup signing and public keys for both parties.
/// Will most likely be done automatically in the public API.
fn setup_crypto() -> (Secp256k1<All>, Address, SecretKey) {
    let secp = Secp256k1::new();
    let (sk, pk) = secp.generate_keypair(&mut rand::thread_rng());
    (secp, pk.into(), sk)
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
    let (secp, addr, sk) = setup_crypto();
    let client = PerunClient::new(&bus);

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

    print_bold!("Both agreed on proposal and nonces, Both sign it");

    // Go to Phase 2: Signing the initial state
    channel.build().unwrap();

    println!("Alice done");
}

/// Bob: Reacts to a proposed channel.
async fn bob(bus: Bus) {
    let (secp, addr, sk) = setup_crypto();
    let c = PerunClient::new(&bus);

    // Wait for Channel Proposal, then accept it
    let mut channel = match bus.rx.recv().unwrap() {
        ParticipantMessage::ChannelProposal(prop) => c.handle_proposal(prop),
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
    channel.build().unwrap();

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
