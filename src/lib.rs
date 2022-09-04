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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Transaction {
    #[serde(rename = "type")]
    kind: TransactionType,
    client: u16,
    tx: u32,
    amount: f32,
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let transaction_engine = transaction_engine::TransactionEngine::new();
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(config.input_path)?;
    for row_result in reader.deserialize() {
        let transaction: Transaction = row_result?;
        println!("{:?}", transaction);
    }
    Ok(())
}
