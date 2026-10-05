//! Chapter 8: parameterized SQL, constraints and atomic transfers in real SQLite.
use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("account ID must be positive and its balance must be nonnegative")]
    InvalidAccount,
    #[error("transfer amount must be positive")]
    InvalidAmount,
    #[error("source and destination must be different accounts")]
    SameAccount,
    #[error("account {0} does not exist")]
    AccountNotFound(i64),
    #[error("source account has insufficient funds")]
    InsufficientFunds,
    #[error("destination balance exceeds the supported amount")]
    BalanceOverflow,
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
}

/// Owns the connection so callers cannot bypass balance constraints through this API.
pub struct AccountRepository {
    connection: Connection,
}

impl AccountRepository {
    /// Each instance has an isolated database that disappears when it is dropped.
    pub fn in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(
            "CREATE TABLE accounts (
                id INTEGER PRIMARY KEY CHECK (id > 0),
                balance_cents INTEGER NOT NULL CHECK (balance_cents >= 0)
            ) STRICT;",
        )?;
        Ok(Self { connection })
    }

    pub fn create(&self, id: i64, balance_cents: i64) -> Result<(), StorageError> {
        if id <= 0 || balance_cents < 0 {
            return Err(StorageError::InvalidAccount);
        }
        self.connection.execute(
            "INSERT INTO accounts (id, balance_cents) VALUES (?1, ?2)",
            (id, balance_cents),
        )?;
        Ok(())
    }

    pub fn balance(&self, id: i64) -> Result<i64, StorageError> {
        read_balance(&self.connection, id)
    }

    pub fn transfer(&mut self, from: i64, to: i64, amount: i64) -> Result<(), StorageError> {
        if amount <= 0 {
            return Err(StorageError::InvalidAmount);
        }
        if from == to {
            return Err(StorageError::SameAccount);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let source = read_balance(&transaction, from)?;
        let destination = read_balance(&transaction, to)?;
        let source = source
            .checked_sub(amount)
            .filter(|balance| *balance >= 0)
            .ok_or(StorageError::InsufficientFunds)?;
        transaction.execute(
            "UPDATE accounts SET balance_cents = ?1 WHERE id = ?2",
            (source, from),
        )?;

        // Deliberately check after the debit: the failure exercises real rollback.
        let destination = destination
            .checked_add(amount)
            .ok_or(StorageError::BalanceOverflow)?;
        transaction.execute(
            "UPDATE accounts SET balance_cents = ?1 WHERE id = ?2",
            (destination, to),
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn read_balance(connection: &Connection, id: i64) -> Result<i64, StorageError> {
    connection
        .query_row(
            "SELECT balance_cents FROM accounts WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(StorageError::AccountNotFound(id))
}

#[cfg(test)]
mod tests {
    use super::{AccountRepository, StorageError};

    #[test]
    fn sql_failure_during_credit_rolls_back_the_debit() {
        let mut repository = AccountRepository::in_memory().unwrap();
        repository.create(1, 10_000).unwrap();
        repository.create(2, 2_000).unwrap();
        repository
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_credit BEFORE UPDATE ON accounts
             WHEN NEW.id = 2
             BEGIN SELECT RAISE(ABORT, 'credit unavailable'); END;",
            )
            .unwrap();

        let error = repository.transfer(1, 2, 3_000).unwrap_err();
        assert!(matches!(error, StorageError::Database(_)));
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(repository.balance(1).unwrap(), 10_000);
        assert_eq!(repository.balance(2).unwrap(), 2_000);
    }
}
