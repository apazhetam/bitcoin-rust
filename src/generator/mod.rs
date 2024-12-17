pub mod generator;

use log::info;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use rand::Rng;
use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
    time,
    thread,
};
use crate::blockchain::Blockchain;
use crate::types::{
    transaction,
    transaction::{SignedTransaction, Transaction},
    mempool::Mempool,
    address::Address,
    key_pair,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

enum ControlSignal {
    Start(u64), // the number controls the theta of interval between transaction generation
    Update, // update the transaction in generation (not sure if necessary)
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    finished_txn_chan: Sender<SignedTransaction>,
    mempool: Arc<Mutex<Mempool>>,
    blockchain: Arc<Mutex<Blockchain>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the generator thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain: &Arc<Mutex<Blockchain>>, mempool: &Arc<Mutex<Mempool>>) -> (Context, Handle, Receiver<SignedTransaction>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_txn_sender, finished_txn_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_txn_chan: finished_txn_sender,
        mempool: Arc::clone(mempool),
        blockchain: Arc::clone(blockchain)
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_txn_receiver)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("generator".to_string())
            .spawn(move || {
                self.generator_loop();
            })
            .unwrap();
        info!("Transaction Generator initialized into paused mode");
    }

    fn generator_loop(&mut self) {
        // main transaction generator loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Transaction Generator shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Transaction Generator starting in continuous mode with theta {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Transaction Generator shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Transaction Generator starting in continuous mode with theta {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                unimplemented!()
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Transaction Generator control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // Begin actual transaction generation process
            println!("Starting the Transaction Generation Process...");

            // Get current tip of blockchain to find parent_state 
            let blockchain = self.blockchain.lock().unwrap();
            let parent_hash = blockchain.tip();
            let parent_state = match blockchain.get_state(&parent_hash) {
                Ok(state) => state.clone(),
                Err(_) => panic!("Parent node does not exist in blockchain.")
            };
            drop(blockchain);

            let mut rng = rand::thread_rng();

            // Choose a random sender
            let sender_seed = rng.gen_range(0..3);     // random seed from {1,2,3}
            let sender_key = Ed25519KeyPair::from_seed_unchecked(&[sender_seed;32]).unwrap();
            let sender_public_key = sender_key.public_key().as_ref().to_vec();
            let sender_address = Address::from_public_key_bytes(&sender_public_key);

            // Get the chosen sender's info from parent state
            let sender_info = parent_state.map[&sender_address];
            let sender_nonce = sender_info.0;
            let sender_balance = sender_info.1;

            // Skip if the chosen sender has no balance
            if sender_balance == 0 {
                continue;
            }
            
            // Choose a random receiver
            let mut receiver_seed = rng.gen_range(0..3);     // random seed from {1,2,3}
            while receiver_seed == sender_seed {        
                receiver_seed = rng.gen_range(0..3);     // ensure receiver is different from sender
            }
            let receiver_key = Ed25519KeyPair::from_seed_unchecked(&[receiver_seed;32]).unwrap();
            let receiver_public_key = receiver_key.public_key().as_ref().to_vec();
            let receiver_address = Address::from_public_key_bytes(&receiver_public_key);

            // Choose a random value for the transaction
            // value must be between [1, balance/2]
            let max_value = sender_balance / 2;
            if max_value <= 1 {
                continue;
            }
            let value = rng.gen_range(1..max_value);

            // Form the transaction
            let transaction = Transaction {
                account_nonce: sender_nonce + 1,    // increment previous nonce
                receiver: receiver_address, 
                value: value
            };

            // Sign the transaction
            let signature = transaction::sign(&transaction, &sender_key).as_ref().to_vec();

            // Form the signed transaction
            let signed_transaction = SignedTransaction {
                transaction: transaction, 
                signature: signature, 
                public_key: sender_public_key
            };

            println!("Transaction generated!");
   
            // Send signed transaction to channel
            self.finished_txn_chan.send(signed_transaction.clone()).expect("Sending to finished_txn_chan resulted in error.");
            
            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i * 200 as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}