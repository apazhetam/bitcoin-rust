use std::collections::HashMap;
use crate::types::address::Address;

#[derive(Debug, Clone)]
pub struct State {
    pub map: HashMap<Address, (u128, u128)>      // <account address, (account nonce, balance)>
}

impl State {
    pub fn new() -> Self {
        Self {
            map: HashMap::new()
        }
    }
}
