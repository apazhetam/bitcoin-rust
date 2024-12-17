use super::{
    hash::{Hashable, H256},
    transaction::SignedTransaction,
};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Mempool {
    pub map: HashMap<H256, SignedTransaction>
}

impl Mempool {
    pub fn new() -> Self {
        Self{
            map: HashMap::new()
        }
    }
}
