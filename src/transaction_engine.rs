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
    pub accounts: HashMap<ClientId, AccountDetails>,
    transactions: HashMap<TransactionId, TransactionDetails>,
}

impl TransactionEngine {
    pub fn new() -> TransactionEngine {
        TransactionEngine {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
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
            Some(t) => match t.amount {
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
            },
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
