use std::error::Error;

use serde::{Deserialize, Serialize};

mod transaction_engine;

pub struct Config {
    pub input_path: String,
}

impl Config {
    //TODO:
    //Change this back later
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        return Ok(Config {
            input_path: "src/tx.csv".to_string(),
        });
        // if args.len() < 2 {
        //     return Err(
        //         "Required arguments not passed. You must pass the input path as an argument",
        //     );
        // }

        // let input_path = args[1].clone();
        // Ok(Config {
        //     input_path: "tx.csv".to_string(),
        // })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

pub type ClientId = u16;
pub type TransactionId = u32;
pub type Amount = f32;

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionInput {
    #[serde(rename = "type")]
    kind: TransactionType,
    client: ClientId,
    tx: TransactionId,
    amount: Option<Amount>,
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let mut transaction_engine = transaction_engine::TransactionEngine::new();
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(config.input_path)?;
    for row_result in reader.deserialize() {
        let transaction: TransactionInput = row_result?;
        if let Err(e) = transaction_engine.process_transaction(transaction) {
            eprintln!("An error occurred when processing a transaction and it was skipped. We'll continue with next transactions. Error: {}", e);
        }
    }
    transaction_engine.print_accounts_state();
    Ok(())
}
