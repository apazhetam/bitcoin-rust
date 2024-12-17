use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::info;
use std::{
    time,
    sync::{Arc, Mutex},
    thread,
};
use crate::{
    network::server::Handle as ServerHandle,
    network::message::Message,
    types::{
        hash::Hashable,
        transaction::SignedTransaction,
        mempool::Mempool,
    },
};


#[derive(Clone)]
pub struct TransactionGenerator {
    server: ServerHandle,
    finished_txn_chan: Receiver<SignedTransaction>,
    mempool: Arc<Mutex<Mempool>>
}

impl TransactionGenerator {
    pub fn new(
        server: &ServerHandle,
        finished_txn_chan: Receiver<SignedTransaction>,
        mempool: &Arc<Mutex<Mempool>>
    ) -> Self {
        Self {
            server: server.clone(),
            finished_txn_chan: finished_txn_chan,
            mempool: Arc::clone(mempool)
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("transaction-generator".to_string())
            .spawn(move || {
                self.generate_transactions();
            })
            .unwrap();
        info!("Transaction generator started");
    }

    fn generate_transactions(&self) {
        loop {
            // Receive transaction from channel
            let txn = self.finished_txn_chan.recv()
                .expect("Error in getting finished transaction");
            
            // Insert this transaction into mempool
            let mut mempool = self.mempool.lock().unwrap();
            mempool.map.insert(txn.hash(), txn.clone());    // insert txn into mempool
            println!("Inserted transaction into mempool");
            drop(mempool);
            
            // Broadcast transaction hash as a NewTransactionHashes message
            let hash = vec![txn.hash()];
            self.server.broadcast(Message::NewTransactionHashes(hash));
        }
    }
}
