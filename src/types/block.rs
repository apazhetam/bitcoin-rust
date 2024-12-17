use crate::types::{
    hash::{H256, Hashable},
    transaction::SignedTransaction,
    merkle::MerkleTree,
};
use rand::Rng;
use bincode;
use serde::{Serialize, Deserialize};

// A Block, composed of a Header and Content
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

// A Header, composed of a block's attributes 
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256,
}

// A Content, containing the transactions data of a block
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content {
    pub transactions: Vec<SignedTransaction>
}

// Implement the hash function for Header
impl Hashable for Header {
    fn hash(&self) -> H256 {
        let serialized_header: Vec<u8> = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &serialized_header).into()
    }
}

// Implement the hash function for Block
impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()    // simply hash the Block's header
    }
}

// Implement getter functions for Block
impl Block {
    pub fn get_parent(&self) -> H256 {
        self.header.parent
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

//------------------------------------------------------------------------------------

// Generate a random Block to help test the Blockchain implementation
#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {
    let mut rng = rand::thread_rng();  // create a random number generator
    let nonce: u32 = rng.gen();        // make nonce a random integer

    let difficulty = H256::default();       // use default difficulty
    let timestamp = rng.gen::<u128>();      // use current time

    let transactions: Vec<SignedTransaction> = Vec::new();  // empty transactions vector
    let merkle_tree = MerkleTree::new(&transactions);       // empty merkle tree
    let merkle_root = merkle_tree.root();

    let content = Content{ transactions };      // content with empty transactions
    
    let header = Header {
        parent: *parent,
        nonce: nonce,
        difficulty: difficulty,
        timestamp: timestamp,
        merkle_root: merkle_root
    };

    Block{ header, content }
}