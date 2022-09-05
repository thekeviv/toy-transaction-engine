use std::collections::HashMap;

use thiserror::Error;

pub use crate::{Amount, ClientId, TransactionId};
pub use crate::{TransactionInput, TransactionType};

#[derive(Error, Debug, Clone)]
pub enum TransactionProcessingError {
    #[error("transaction can't be processed as account is locked")]
    AccountLocked,

    #[error("account not found")]
    AccountNotFound,

    #[error("transaction cannot be completed due to insufficient funds")]
    InsufficientFunds,

    #[error("amount value required to process the transaction of specified type")]
    AmountValueNotFound,

    #[error("provided transaction id not found")]
    TransactionNotFound,

    #[error("provided transaction id not found")]
    AmountNotFoundOnTransactionToDispute,

    #[error("cannot resolve a non disputed transaction")]
    CannotResolveNonDisputedTransaction,

    #[error("cannot dispute an already disputed transaction")]
    CannotDisputeAnAlreadyDisputedTransaction,
}

pub struct AccountDetails {
    pub available: Amount,
    pub held: Amount,
    pub total: Amount,
    pub locked: bool,
}

struct TransactionDetails {
    kind: TransactionType,
    client: ClientId,
    amount: Option<Amount>,
    is_disputed: bool,
}

pub struct TransactionEngine {
    // not putting client inside and using a hashmap as searching which would need to be
    // done when processing every tx, would be an O(1)
    // operation while in a simple vec, it would take longer
    accounts: HashMap<ClientId, AccountDetails>,
    transactions: HashMap<TransactionId, TransactionDetails>,
}

impl TransactionEngine {
    pub fn new() -> TransactionEngine {
        TransactionEngine {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    pub fn print_accounts_state(self) -> () {
        println!("client, available, held, total, locked");
        for (client_id, client_details) in self.accounts {
            println!(
                "{:>6},{:>10.4},{:>5.4},{:>6.4},{:>7}",
                client_id,
                client_details.available,
                client_details.held,
                client_details.total,
                client_details.locked
            );
        }
    }

    pub fn process_transaction(
        &mut self,
        transaction: TransactionInput,
    ) -> Result<(), TransactionProcessingError> {
        let previous_account_data = self.accounts.get(&transaction.client);
        // if the account is locked, no transaction is allowed on it
        if let Some(a) = previous_account_data {
            if a.locked {
                return Err(TransactionProcessingError::AccountLocked.into());
            }
        }

        match transaction.kind {
            TransactionType::Deposit => {
                if let Some(amount) = transaction.amount {
                    return self.process_deposit_transaction(
                        transaction.tx,
                        transaction.client,
                        amount,
                    );
                } else {
                    return Err(TransactionProcessingError::AmountValueNotFound.into());
                };
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = transaction.amount {
                    return self.process_withdrawal_transaction(
                        transaction.tx,
                        transaction.client,
                        amount,
                    );
                } else {
                    return Err(TransactionProcessingError::AmountValueNotFound.into());
                };
            }
            TransactionType::Dispute => return self.process_dispute_transaction(transaction.tx),
            TransactionType::Resolve => return self.process_resolve_transaction(transaction.tx),
            TransactionType::Chargeback => {
                return self.process_chargeback_transaction(transaction.tx)
            }
        }
    }

    fn process_deposit_transaction(
        &mut self,
        transaction_id: TransactionId,
        client_id: u16,
        amount: f32,
    ) -> Result<(), TransactionProcessingError> {
        let previous_account_data = self.accounts.entry(client_id).or_insert(AccountDetails {
            available: 0.0,
            total: 0.0,
            held: 0.0,
            locked: false,
        });

        *previous_account_data = AccountDetails {
            available: previous_account_data.available + amount,
            total: previous_account_data.total + amount,
            held: previous_account_data.held,
            locked: previous_account_data.locked,
        };
        self.transactions.insert(
            transaction_id,
            TransactionDetails {
                kind: TransactionType::Deposit,
                client: client_id,
                amount: Some(amount),
                is_disputed: false,
            },
        );
        return Ok(());
    }

    fn process_withdrawal_transaction(
        &mut self,
        transaction_id: TransactionId,
        client_id: ClientId,
        amount: f32,
    ) -> Result<(), TransactionProcessingError> {
        let previous_account_data = self.accounts.get_mut(&client_id);
        match previous_account_data {
            Some(account) => {
                if account.available > amount {
                    *account = AccountDetails {
                        available: account.available - amount,
                        total: account.total - amount,
                        held: account.held,
                        locked: account.locked,
                    };
                    self.transactions.insert(
                        transaction_id,
                        TransactionDetails {
                            kind: TransactionType::Deposit,
                            client: client_id,
                            amount: Some(amount),
                            is_disputed: false,
                        },
                    );
                    return Ok(());
                } else {
                    return Err(TransactionProcessingError::InsufficientFunds);
                }
            }
            None => return Err(TransactionProcessingError::AccountNotFound),
        }
    }

    fn process_dispute_transaction(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<(), TransactionProcessingError> {
        let existing_transaction_details = self.transactions.get_mut(&transaction_id);
        match existing_transaction_details {
            Some(t) => {
                if t.is_disputed {
                    return Err(
                        TransactionProcessingError::CannotDisputeAnAlreadyDisputedTransaction
                            .into(),
                    );
                }

                match t.amount {
                    Some(amount) => {
                        let account_details = self.accounts.get_mut(&t.client);
                        match account_details {
                            Some(a) => {
                                *a = AccountDetails {
                                    available: a.available - amount,
                                    total: a.total,
                                    held: a.held + amount,
                                    locked: a.locked,
                                };

                                *t = TransactionDetails {
                                    kind: t.kind,
                                    client: t.client,
                                    amount: t.amount,
                                    is_disputed: true,
                                };
                                Ok(())
                            }
                            None => return Err(TransactionProcessingError::AccountNotFound),
                        }
                    }
                    None => {
                        return Err(
                            TransactionProcessingError::AmountNotFoundOnTransactionToDispute.into(),
                        );
                    }
                }
            }
            None => {
                return Err(TransactionProcessingError::TransactionNotFound.into());
            }
        }
    }

    fn process_resolve_transaction(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<(), TransactionProcessingError> {
        let existing_transaction_details = self.transactions.get_mut(&transaction_id);
        match existing_transaction_details {
            Some(t) => {
                if t.is_disputed {
                    match t.amount {
                        Some(amount) => {
                            let account_details = self.accounts.get_mut(&t.client);
                            match account_details {
                                Some(a) => {
                                    *a = AccountDetails {
                                        available: a.available + amount,
                                        total: a.total,
                                        held: a.held - amount,
                                        locked: a.locked,
                                    };

                                    *t = TransactionDetails {
                                        kind: t.kind,
                                        client: t.client,
                                        amount: t.amount,
                                        is_disputed: false,
                                    };
                                    Ok(())
                                }
                                None => return Err(TransactionProcessingError::AccountNotFound),
                            }
                        }
                        None => {
                            return Err(
                                TransactionProcessingError::AmountNotFoundOnTransactionToDispute
                                    .into(),
                            );
                        }
                    }
                } else {
                    return Err(
                        TransactionProcessingError::CannotResolveNonDisputedTransaction.into(),
                    );
                }
            }
            None => {
                return Err(TransactionProcessingError::TransactionNotFound.into());
            }
        }
    }

    fn process_chargeback_transaction(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<(), TransactionProcessingError> {
        let existing_transaction_details = self.transactions.get_mut(&transaction_id);
        match existing_transaction_details {
            Some(t) => {
                if t.is_disputed {
                    match t.amount {
                        Some(amount) => {
                            let account_details = self.accounts.get_mut(&t.client);
                            match account_details {
                                Some(a) => {
                                    *a = AccountDetails {
                                        available: a.available,
                                        total: a.total - amount,
                                        held: a.held - amount,
                                        locked: true,
                                    };

                                    *t = TransactionDetails {
                                        kind: t.kind,
                                        client: t.client,
                                        amount: t.amount,
                                        is_disputed: false,
                                    };
                                    Ok(())
                                }
                                None => return Err(TransactionProcessingError::AccountNotFound),
                            }
                        }
                        None => {
                            return Err(
                                TransactionProcessingError::AmountNotFoundOnTransactionToDispute
                                    .into(),
                            );
                        }
                    }
                } else {
                    return Err(
                        TransactionProcessingError::CannotResolveNonDisputedTransaction.into(),
                    );
                }
            }
            None => {
                return Err(TransactionProcessingError::TransactionNotFound.into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TransactionEngine, TransactionProcessingError};
    use crate::{TransactionInput, TransactionType};

    #[test]
    fn test_deposit_transaction() {
        let mut transaction_engine = TransactionEngine::new();
        let deposit_transaction_1 = TransactionInput {
            amount: Some(5.0004),
            client: 1,
            kind: TransactionType::Deposit,
            tx: 1,
        };
        let result = transaction_engine.process_transaction(deposit_transaction_1);
        match result {
            Ok(r) => {
                assert_eq!(r, ());
                let created_account = transaction_engine
                    .accounts
                    .get(&1)
                    .expect("An account wasn't found for the client 1");
                assert_eq!(created_account.available, 5.0004);
                assert_eq!(created_account.held, 0.0);
                assert_eq!(created_account.total, 5.0004);
                assert_eq!(created_account.locked, false);
            }
            Err(e) => {
                panic!(
                    "The deposit transaction should have succeeded but it failed with error: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_withdraw_transaction() {
        let mut transaction_engine = TransactionEngine::new();
        let result = transaction_engine.process_transaction(TransactionInput {
            amount: Some(1.0004),
            client: 1,
            kind: TransactionType::Withdrawal,
            tx: 1,
        });
        match result {
            Ok(_) => {
                panic!("The withdraw transaction on account with insufficient balance must fail");
            }
            Err(e) => match e {
                TransactionProcessingError::AccountNotFound => (),
                _ => {
                    panic!("Expected the error to be account not found error")
                }
            },
        }
        let deposit_result = transaction_engine.process_transaction(TransactionInput {
            amount: Some(5.0004),
            client: 1,
            kind: TransactionType::Deposit,
            tx: 1,
        });
        match deposit_result {
            Ok(_) => {
                let created_account = transaction_engine
                    .accounts
                    .get(&1)
                    .expect("An account wasn't found for the client 1");
                assert_eq!(created_account.available, 5.0004);
                assert_eq!(created_account.held, 0.0);
                assert_eq!(created_account.total, 5.0004);
                assert_eq!(created_account.locked, false);
                let withdraw_result = transaction_engine.process_transaction(TransactionInput {
                    amount: Some(1.0004),
                    client: 1,
                    kind: TransactionType::Withdrawal,
                    tx: 1,
                });
                match withdraw_result {
                    Ok(_) => {
                        let updated_account = transaction_engine
                            .accounts
                            .get(&1)
                            .expect("An account wasn't found for the client 1");
                        assert_eq!(updated_account.available, 4.0);
                        assert_eq!(updated_account.held, 0.0);
                        assert_eq!(updated_account.total, 4.0);
                        assert_eq!(updated_account.locked, false);
                        let withdraw_result_2 =
                            transaction_engine.process_transaction(TransactionInput {
                                amount: Some(6.0),
                                client: 1,
                                kind: TransactionType::Withdrawal,
                                tx: 1,
                            });
                        match withdraw_result_2 {
                            Ok(_) => {
                                panic!("Expected transaction to fail due to insufficient funds");
                            }
                            Err(e) => match e {
                                TransactionProcessingError::InsufficientFunds => (),
                                _ => {
                                    panic!("expected error to be insufficient funds error");
                                }
                            },
                        }
                    }
                    Err(e) => {
                        panic!(
                            "The withdraw transaction should have succeeded. Error: {}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                panic!(
                    "The deposit transaction should have succeeded but it failed with error: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_dispute_transaction() {
        let mut transaction_engine = TransactionEngine::new();
        let result = transaction_engine.process_transaction(TransactionInput {
            amount: None,
            client: 1,
            kind: TransactionType::Dispute,
            tx: 1,
        });
        match result {
            Ok(_) => {
                panic!("Expected dispute to fail for non existing transaction");
            }
            Err(e) => match e {
                TransactionProcessingError::TransactionNotFound => (),
                _ => {
                    panic!("Expected error to be a transaction not found error");
                }
            },
        }
        let result = transaction_engine.process_transaction(TransactionInput {
            amount: Some(1.1),
            client: 1,
            kind: TransactionType::Deposit,
            tx: 1,
        });
        match result {
            Ok(_) => {
                let dispute_result = transaction_engine.process_transaction(TransactionInput {
                    amount: None,
                    client: 1,
                    kind: TransactionType::Dispute,
                    tx: 1,
                });
                match dispute_result {
                    Ok(_) => {
                        let account_state = transaction_engine
                            .accounts
                            .get(&1)
                            .expect("An account wasn't found for the client 1");
                        assert_eq!(account_state.available, 0.0);
                        assert_eq!(account_state.held, 1.1);
                        assert_eq!(account_state.total, 1.1);
                        assert_eq!(account_state.locked, false);
                        let dispute_result_2 =
                            transaction_engine.process_transaction(TransactionInput {
                                amount: None,
                                client: 1,
                                kind: TransactionType::Dispute,
                                tx: 1,
                            });
                        match dispute_result_2 {
                            Ok(_) => {
                                panic!("Expected dispute on already disputed transaction to fail");
                            }
                            Err(e) => {
                                match e {
                                    TransactionProcessingError::CannotDisputeAnAlreadyDisputedTransaction => (),
                                    _ => {
                                        panic!("Expected CannotDisputeAnAlreadyDisputedTransaction error type")
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        panic!("Expected dispute transaction to succeed");
                    }
                }
            }
            Err(_) => {
                panic!("Expected deposit transaction to succeed");
            }
        }
    }

    #[test]
    fn test_resolve_transaction() {
        let mut transaction_engine = TransactionEngine::new();
        let result = transaction_engine.process_transaction(TransactionInput {
            amount: None,
            client: 1,
            kind: TransactionType::Resolve,
            tx: 1,
        });
        match result {
            Ok(_) => {
                panic!("Expected resolve to fail for non existing transaction");
            }
            Err(e) => match e {
                TransactionProcessingError::TransactionNotFound => (),
                _ => {
                    panic!("Expected error to be a transaction not found error");
                }
            },
        };
        let result = transaction_engine.process_transaction(TransactionInput {
            amount: Some(1.1),
            client: 1,
            kind: TransactionType::Deposit,
            tx: 1,
        });
        match result {
            Ok(_) => {
                let dispute_result = transaction_engine.process_transaction(TransactionInput {
                    amount: None,
                    client: 1,
                    kind: TransactionType::Dispute,
                    tx: 1,
                });
                match dispute_result {
                    Ok(_) => {
                        let account_state = transaction_engine
                            .accounts
                            .get(&1)
                            .expect("An account wasn't found for the client 1");
                        assert_eq!(account_state.available, 0.0);
                        assert_eq!(account_state.held, 1.1);
                        assert_eq!(account_state.total, 1.1);
                        assert_eq!(account_state.locked, false);
                        let resolve_result =
                            transaction_engine.process_transaction(TransactionInput {
                                kind: TransactionType::Resolve,
                                client: 1,
                                tx: 1,
                                amount: None,
                            });
                        match resolve_result {
                            Ok(_) => {
                                let account_state = transaction_engine
                                    .accounts
                                    .get(&1)
                                    .expect("An account wasn't found for the client 1");
                                assert_eq!(account_state.available, 1.1);
                                assert_eq!(account_state.held, 0.0);
                                assert_eq!(account_state.total, 1.1);
                                assert_eq!(account_state.locked, false);
                            }
                            Err(_) => {
                                panic!("Expected resolve to succeed");
                            }
                        }
                    }
                    Err(_) => {
                        panic!("Expected dispute transaction to succeed");
                    }
                }
            }
            Err(_) => {
                panic!("Expected deposit transaction to succeed");
            }
        }
    }

    #[test]
    fn test_chargeback_transaction() {
        let mut transaction_engine = TransactionEngine::new();
        let result = transaction_engine.process_transaction(TransactionInput {
            amount: None,
            client: 1,
            kind: TransactionType::Resolve,
            tx: 1,
        });
        match result {
            Ok(_) => {
                panic!("Expected resolve to fail for non existing transaction");
            }
            Err(e) => match e {
                TransactionProcessingError::TransactionNotFound => (),
                _ => {
                    panic!("Expected error to be a transaction not found error");
                }
            },
        };
        let result = transaction_engine.process_transaction(TransactionInput {
            amount: Some(1.1),
            client: 1,
            kind: TransactionType::Deposit,
            tx: 1,
        });
        match result {
            Ok(_) => {
                let dispute_result = transaction_engine.process_transaction(TransactionInput {
                    amount: None,
                    client: 1,
                    kind: TransactionType::Dispute,
                    tx: 1,
                });
                match dispute_result {
                    Ok(_) => {
                        let account_state = transaction_engine
                            .accounts
                            .get(&1)
                            .expect("An account wasn't found for the client 1");
                        assert_eq!(account_state.available, 0.0);
                        assert_eq!(account_state.held, 1.1);
                        assert_eq!(account_state.total, 1.1);
                        assert_eq!(account_state.locked, false);

                        let chargeback_result =
                            transaction_engine.process_transaction(TransactionInput {
                                kind: TransactionType::Chargeback,
                                client: 1,
                                tx: 1,
                                amount: None,
                            });
                        match chargeback_result {
                            Ok(_) => {
                                let account_state = transaction_engine
                                    .accounts
                                    .get(&1)
                                    .expect("An account wasn't found for the client 1");
                                assert_eq!(account_state.available, 0.0);
                                assert_eq!(account_state.held, 0.0);
                                assert_eq!(account_state.total, 0.0);
                                assert_eq!(account_state.locked, true);
                            }
                            Err(_) => {
                                panic!("Expected chargeback to succeed");
                            }
                        }
                    }
                    Err(_) => {
                        panic!("Expected dispute transaction to succeed");
                    }
                }
            }
            Err(_) => {
                panic!("Expected deposit transaction to succeed");
            }
        }
    }
}
