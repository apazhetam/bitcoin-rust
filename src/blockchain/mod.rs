use crate::types::{
    address::Address,
    block::{Block, Content, Header},
    hash::{H256, Hashable},
    transaction,
    transaction::SignedTransaction,
    merkle::MerkleTree,
    state::State
};
use std::collections::HashMap;
use hex_literal::hex;
use ring::signature::{Ed25519KeyPair, KeyPair};

// A BlockNode is a node in the Blockchain
pub struct BlockNode {
    block: Block, 
    height: u64,
    pub state: State
}

// A Blockchain
pub struct Blockchain {
    map: HashMap<H256, BlockNode>,
    tip: H256
}

// Implement functions for the Blockchain
impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let mut map = HashMap::new();

        let genesis_parent: H256 = (hex!("0000000000000000000000000000000000000000000000000000000000000000")).into();
        let nonce: u32 = 0;
        
        let transactions: Vec<SignedTransaction> = Vec::new();
        let merkle_tree = MerkleTree::new(&transactions);
        let merkle_root = merkle_tree.root();
        
        let difficulty: H256 = hex!("0000100000000000000000000000000000000000000000000000000000000000").into();
        let timestamp: u128 = 0;

        let content = Content { transactions };

        let header = Header {
            parent: genesis_parent,
            nonce: nonce,
            difficulty: difficulty,
            timestamp: timestamp,
            merkle_root: merkle_root
        };

        let genesis_block = Block { header, content };
        let tip = genesis_block.hash();
        println!("GENISIS HASH: {}", tip);

        // Initialize the genesis block node's state with 3 accounts
        let mut state = State::new();
        for seed in 0..3 {
            let key = Ed25519KeyPair::from_seed_unchecked(&[seed;32]).unwrap();
            let public_key = key.public_key().as_ref().to_vec();
            let addr = Address::from_public_key_bytes(&public_key);
            
            // Only first account has a nonzero balance
            let balance = if seed == 0 { 10000u128 } else { 0 };
            state.map.insert(addr, (0, balance));    // account_nonce initialized to 0
        }

        map.insert(genesis_block.hash(), BlockNode { block: genesis_block, height: 0, state: state });

        Blockchain { map, tip }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) -> Result<(), bool> {
        let parent_node = match self.map.get(&block.get_parent()) {
            Some(node) => node,    // parent exists in hashmap
            None => {
                // parent is missing in hashmap, so return an error
                return Err(true);
            }
        };

        // Check if block is a duplicate
        if self.map.contains_key(&block.hash()) {
            return Err(false);   // block already exists
        }

        let height = parent_node.height + 1;
        let parent_state = parent_node.state.clone();
        
        // Validate all transactions in the block
        for txn in block.content.transactions.iter() {
            // Check transaction validity
            if !transaction::verify(&txn.transaction, &txn.public_key, &txn.signature) {
                return Err(false);       // transaction verification failed
            }

            // Check account state
            let sender_address = Address::from_public_key_bytes(&txn.public_key);
            let sender_info = match parent_state.map.get(&sender_address) {
                Some(acc_info) => acc_info,
                None => {
                    return Err(false);   // sender's address not in state hashmap
                },
            };
            let sender_nonce = sender_info.0;
            let sender_balance = sender_info.1;

            // Check if the new account nonce in the transaction is correct
            if sender_nonce + 1 != txn.transaction.account_nonce {
                return Err(false);      // transaction has invalid account nonce
            }
            
            // Check if sender's balance is enough
            if sender_balance < txn.transaction.value {
                return Err(false);      // balance is not enough
            }
        }

        // All the transactions are valid, so create a new state for them
        let mut new_state = parent_state.clone();
        for txn in block.content.transactions.iter() {
            let sender_address = Address::from_public_key_bytes(&txn.public_key);
            let receiver_address = txn.transaction.receiver;
            let value = txn.transaction.value;

            if let Some(sender_info) = parent_state.map.get(&sender_address) {
                // Txn value is subracted from sender's balance
                let new_sender_balance = sender_info.1 - value;
                new_state.map.insert(sender_address, (sender_info.0 + 1, new_sender_balance));
            }
        
            if let Some(receiver_info) = parent_state.map.get(&receiver_address) {
                // Txn value is added to receiver's balance
                let new_receiver_balance = receiver_info.1 + value;
                new_state.map.insert(receiver_address, (receiver_info.0, new_receiver_balance));
            }
        }
        
        let blocknode = BlockNode { 
            block: block.clone(), 
            height: height,
            state: new_state.clone()
        }; 

        // Insert blocknode into hashmap
        self.map.insert(block.hash(), blocknode);

        // Update tip
        let tip_node = self.map.get(&self.tip).unwrap();        
        if height > tip_node.height {
            self.tip = block.hash();
        }

        Ok(())    // Successfully inserted block
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        return self.tip;
    }

    /// Get a desired block from the blockchain
    pub fn get_block(&self, blockhash: &H256) -> Result<&Block, &'static str> {
        match self.map.get(blockhash){
            Some(node) => {
                return Ok(&node.block);     // block exists in hashmap
            }
            None => {
                return Err("Block does not exist in blockchain.");   // block not found
            }
        }
    }

    /// Get a desired block's state
    pub fn get_state(&self, blockhash: &H256) -> Result<&State, &'static str> {
        match self.map.get(blockhash){
            Some(node) => {
                return Ok(&node.state);     // block exists in hashmap
            }
            None => {
                return Err("Block does not exist in blockchain.");   // block not found
            }
        }
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut longest_chain: Vec<H256> = Vec::new();
        
        // Start with the tip 
        let mut cur_block_hash: H256 = self.tip; 

        // Move upwards through chain until genesis block is reached
        loop {
            longest_chain.push(cur_block_hash); 
            let cur_blocknode = self.map.get(&cur_block_hash).unwrap();
            if cur_blocknode.height == 0 { break; }   // end loop at genesis block
            cur_block_hash = cur_blocknode.block.get_parent();   // move to parent
        }

        longest_chain.reverse();   // reverses longest_chain vector in-place
        
        longest_chain
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        let _ = blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }

    #[test]
    fn insert_three() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block1 = generate_random_block(&genesis_hash);
        let block2 = generate_random_block(&block1.hash());
        let block3 = generate_random_block(&block2.hash());

        let _ = blockchain.insert(&block1);
        let _ = blockchain.insert(&block2);
        let _ = blockchain.insert(&block3);
        
        let chain = blockchain.all_blocks_in_longest_chain();

        // longest chain: gen -> b1 -> b2 -> b3
        // tip: b3
        
        assert_eq!(blockchain.tip(), block3.hash());
        assert_eq!(chain.len(), 4);
        assert_eq!(chain[0], genesis_hash);
        assert_eq!(chain[1], block1.hash());
        assert_eq!(chain[2], block2.hash());
        assert_eq!(chain[3], block3.hash());

        // Check if height values are correct
        assert_eq!(blockchain.map.get(&genesis_hash).unwrap().height, 0);
        assert_eq!(blockchain.map.get(&block1.hash()).unwrap().height, 1);
        assert_eq!(blockchain.map.get(&block2.hash()).unwrap().height, 2);
        assert_eq!(blockchain.map.get(&block3.hash()).unwrap().height, 3);
    }

    #[test]
    fn insert_four_with_fork() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block1 = generate_random_block(&genesis_hash);
        let block2 = generate_random_block(&block1.hash());
        let block3 = generate_random_block(&block1.hash());
        let block4 = generate_random_block(&block3.hash());
        
        let _ = blockchain.insert(&block1);
        let _ = blockchain.insert(&block2);
        let _ = blockchain.insert(&block3);
        let _ = blockchain.insert(&block4);
        
        let chain = blockchain.all_blocks_in_longest_chain();

        // longest chain: gen -> b1 -> b3 -> b4
        // tip: b4
        
        assert_eq!(blockchain.tip(), block4.hash());
        assert_eq!(chain.len(), 4);
        assert_eq!(chain[0], genesis_hash);
        assert_eq!(chain[1], block1.hash());
        assert_eq!(chain[2], block3.hash());
        assert_eq!(chain[3], block4.hash());

        // Check if height values are correct
        assert_eq!(blockchain.map.get(&genesis_hash).unwrap().height, 0);
        assert_eq!(blockchain.map.get(&block1.hash()).unwrap().height, 1);
        assert_eq!(blockchain.map.get(&block2.hash()).unwrap().height, 2);
        assert_eq!(blockchain.map.get(&block3.hash()).unwrap().height, 2);
        assert_eq!(blockchain.map.get(&block4.hash()).unwrap().height, 3);
    }

    #[test]
    fn insert_six_with_err() {
        // This test was adapted from an Ed post by another student.
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block1 = generate_random_block(&genesis_hash);
        let block2 = generate_random_block(&genesis_hash);
        let block3 = generate_random_block(&genesis_hash.hash().hash());
        let block4 = generate_random_block(&block1.hash());
        let block5 = generate_random_block(&block2.hash());
        let block6 = generate_random_block(&block5.hash());

        //      genesis
        //        / \
        //       1   2   (3)
        //       |   | 
        //       4   5
        //           |
        //           6    
        
        let a = blockchain.insert(&block1); 
        assert_eq!(blockchain.tip(), block1.hash());

        let b = blockchain.insert(&block2); 
        assert_eq!(blockchain.tip(), block1.hash());

        let c = blockchain.insert(&block3);
        let d = blockchain.insert(&block2);
        let e = blockchain.insert(&block4);
        assert_eq!(blockchain.tip(), block4.hash());

        let f = blockchain.insert(&block5); 
        assert_eq!(blockchain.tip(), block4.hash());
        
        let g = blockchain.insert(&block6);
        assert_eq!(blockchain.tip(), block6.hash());
        
        assert!(!a.is_err());   // Ok
        assert!(!b.is_err());   // Ok (forked chain)
        assert!(c.is_err());    // Err (parent does not exist)
        assert!(d.is_err());    // Err (duplicate block)
        assert!(!e.is_err());   // Ok (new tip)
        assert!(!f.is_err());   // Ok
        assert!(!g.is_err());   // Ok (new tip)

        // Check longest chain
        let mut hash_vec = Vec::new();
        hash_vec.push(genesis_hash);
        hash_vec.push(block2.hash());
        hash_vec.push(block5.hash());
        hash_vec.push(block6.hash());
        assert_eq!(blockchain.all_blocks_in_longest_chain(), hash_vec);

        // Check if height values are correct
        assert_eq!(blockchain.map.get(&genesis_hash).unwrap().height, 0);
        assert_eq!(blockchain.map.get(&block1.hash()).unwrap().height, 1);
        assert_eq!(blockchain.map.get(&block2.hash()).unwrap().height, 1);
        assert_eq!(blockchain.map.get(&block4.hash()).unwrap().height, 2);
        assert_eq!(blockchain.map.get(&block5.hash()).unwrap().height, 2);
        assert_eq!(blockchain.map.get(&block6.hash()).unwrap().height, 3);
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST