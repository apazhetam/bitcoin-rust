use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{debug, info};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use crate::{
    blockchain::Blockchain,
    network::server::Handle as ServerHandle,
    network::message::Message,
    types::{
        hash::Hashable,
        block::Block,
        transaction::SignedTransaction,
        mempool::Mempool,
    },
};

#[derive(Clone)]
pub struct Worker {
    server: ServerHandle,
    finished_block_chan: Receiver<Block>,
    blockchain: Arc<Mutex<Blockchain>>
}

impl Worker {
    pub fn new(
        server: &ServerHandle,
        finished_block_chan: Receiver<Block>,
        blockchain: &Arc<Mutex<Blockchain>>
    ) -> Self {
        Self {
            server: server.clone(),
            finished_block_chan: finished_block_chan,
            blockchain: Arc::clone(blockchain)
        }
    }

    pub fn start(self) {
        thread::Builder::new()
            .name("miner-worker".to_string())
            .spawn(move || {
                self.worker_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn worker_loop(&self) {
        loop {
            // Receive block from channel
            let block = self.finished_block_chan.recv().expect("Receive finished block error");
            
            // Insert this block into blockchain
            let mut blockchain = self.blockchain.lock().unwrap();
            let result = blockchain.insert(&block);
            drop(blockchain);
            
            match result {
                Ok(_) => println!("SUCCESS - inserted block into blockchain"),
                Err(e) => panic!("{}", e)
            }

            // Broadcast block hash as a NewBlockHashes message
            let hash = vec![block.hash()];
            self.server.broadcast(Message::NewBlockHashes(hash));
            println!("Broadcast the new block everywhere");
        }
    }
}
