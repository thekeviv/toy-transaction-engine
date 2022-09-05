use std::{env, process};

use toy_transaction_engine::Config;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Couldn't pass the arguments: {}", err);
        process::exit(1)
    });

    if let Err(e) = toy_transaction_engine::run(config) {
        eprintln!("An error occurred in the application: {e}");
        process::exit(1);
    }
}
