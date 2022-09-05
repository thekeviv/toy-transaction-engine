# Coding Test

## Description

The repository contains a simple toy transaction engine which is able to process different kinds of transactions.

## Important Regarding Error Message Logging

I am currently logging error messages to stderr. So they will show up in a console depending on your console configuration. If you redirect the stdout to a file, the errors won't show up in that file and the output will be the expected output.

## Assumptions Made

1. It is assumed that clients with different client ids are allowed to interact with accounts which weren't created by them directly.
2. It is assumed that only deposit and withdraw transactions can be disputed, resolved or charged back.
3. It is assumed that a transaction which has already been disputed is not allowed to be disputed again.
4. It is assumed that spacing and ordering in rows doesn't matter.

## Design Decisions

1. I decided not to store accounts and transactions as simple lists but instead as hashmaps with account/transaction id's as keys. I noticed that access to them is made many times throughout the code and using a vec, the search will be slower whereas with a hashmap, it will be faster.
2. The library still returns errors on why transactions fail. I understand that this can be a security risk but I assume it wouldn't be used directly where any business logic to hide sensitive errors (like account not existing etc.) can be added.
3. I thought of using a Trait for a transaction with a method to process that transaction and then having a different struct for each transaction type which implement that trait. The reason for that being that I can better model the amount not existing on some transaction types. However, on thinking more about it, I came to the realization that a single struct type and using an enum makes the code simpler and easier to maintain with just the downside of requiring a few extra run time checks. I think an that due to using enums, if in future, we need to add new transaction types, the match will ensure exhaustive case checking to make sure that an implementation for processing it is provided.
4. While I didn't test with a large input file, I read that internally, the csv reader does buffering when opening large files. So, opening large files should be ok. The code does keep accounts and transactions in memory which can become an issue in future as accounts and transactions become very large. In that case, we may need to move one or both of them to some external storage. I know that the Solana blockchain keeps account states in memory but for transactions, it keeps the past some months of transactions in memory while older ones are archived to Bigtable. This will need future revisiting for adapting this for production use.

## Building, Running and Testing

1. Building - Run `cargo build`.
2. Running - Run `cargo run -- path_to_csv_file > output.csv`.
3. Testing - Run `cargo test`.
