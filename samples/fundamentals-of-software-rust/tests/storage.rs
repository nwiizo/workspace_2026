use fundamentals_of_software_rust::storage::{AccountRepository, StorageError};

fn accounts(source: i64, destination: i64) -> AccountRepository {
    let repository = AccountRepository::in_memory().unwrap();
    repository.create(1, source).unwrap();
    repository.create(2, destination).unwrap();
    repository
}

#[test]
fn transfer_commits_both_balances_and_preserves_the_total() {
    let mut repository = accounts(10_000, 2_000);
    repository.transfer(1, 2, 3_000).unwrap();
    assert_eq!(repository.balance(1).unwrap(), 7_000);
    assert_eq!(repository.balance(2).unwrap(), 5_000);
    assert_eq!(
        repository.balance(1).unwrap() + repository.balance(2).unwrap(),
        12_000
    );
}

#[test]
fn destination_overflow_rolls_back_an_already_executed_debit() {
    let mut repository = accounts(10_000, i64::MAX);
    assert!(matches!(
        repository.transfer(1, 2, 1),
        Err(StorageError::BalanceOverflow)
    ));
    assert_eq!(repository.balance(1).unwrap(), 10_000);
    assert_eq!(repository.balance(2).unwrap(), i64::MAX);
    // The failed transaction must also release its lock and allow another transfer.
    repository.transfer(2, 1, 1).unwrap();
    assert_eq!(repository.balance(1).unwrap(), 10_001);
    assert_eq!(repository.balance(2).unwrap(), i64::MAX - 1);
}

#[test]
fn insufficient_funds_do_not_change_either_account() {
    let mut repository = accounts(100, 200);
    assert!(matches!(
        repository.transfer(1, 2, 101),
        Err(StorageError::InsufficientFunds)
    ));
    assert_eq!(repository.balance(1).unwrap(), 100);
    assert_eq!(repository.balance(2).unwrap(), 200);
}

#[test]
fn missing_accounts_do_not_debit_the_source() {
    let mut repository = accounts(100, 200);
    assert!(matches!(
        repository.transfer(1, 9, 1),
        Err(StorageError::AccountNotFound(9))
    ));
    assert!(matches!(
        repository.transfer(9, 1, 1),
        Err(StorageError::AccountNotFound(9))
    ));
    assert_eq!(repository.balance(1).unwrap(), 100);
    assert_eq!(repository.balance(2).unwrap(), 200);
}

#[test]
fn invalid_transfers_leave_balances_unchanged() {
    let mut repository = accounts(100, 200);
    for amount in [0, -1, i64::MIN] {
        assert!(matches!(
            repository.transfer(1, 2, amount),
            Err(StorageError::InvalidAmount)
        ));
    }
    assert!(matches!(
        repository.transfer(1, 1, 1),
        Err(StorageError::SameAccount)
    ));
    assert_eq!(repository.balance(1).unwrap(), 100);
    assert_eq!(repository.balance(2).unwrap(), 200);
}

#[test]
fn duplicate_account_creation_preserves_the_original_balance() {
    let repository = accounts(100, 200);
    assert!(matches!(
        repository.create(1, 999),
        Err(StorageError::Database(_))
    ));
    assert_eq!(repository.balance(1).unwrap(), 100);
}

#[test]
fn invalid_accounts_are_rejected_and_databases_are_isolated() {
    let repository = AccountRepository::in_memory().unwrap();
    for (id, balance) in [(0, 100), (-1, 100), (1, -1)] {
        assert!(matches!(
            repository.create(id, balance),
            Err(StorageError::InvalidAccount)
        ));
    }
    repository.create(1, 0).unwrap();
    let other = AccountRepository::in_memory().unwrap();
    assert!(matches!(
        other.balance(1),
        Err(StorageError::AccountNotFound(1))
    ));
}

#[test]
fn the_entire_source_balance_can_be_transferred() {
    let mut repository = accounts(i64::MAX, 0);
    repository.transfer(1, 2, i64::MAX).unwrap();
    assert_eq!(repository.balance(1).unwrap(), 0);
    assert_eq!(repository.balance(2).unwrap(), i64::MAX);
}
