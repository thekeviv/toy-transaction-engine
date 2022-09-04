use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Account {
    client: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

pub struct TransactionEngine {
    accounts: Vec<Account>,
}

impl TransactionEngine {
    pub fn new() -> TransactionEngine {
        TransactionEngine {
            accounts: Vec::new(),
        }
    }
}
