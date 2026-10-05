use fundamentals_of_software_rust::storage::{AccountRepository, StorageError};

fn main() -> Result<(), StorageError> {
    let mut repository = AccountRepository::in_memory()?;
    repository.create(1, 10_000)?;
    repository.create(2, 2_000)?;
    repository.transfer(1, 2, 3_000)?;
    println!(
        "Committed: source={}, destination={}",
        repository.balance(1)?,
        repository.balance(2)?
    );
    if let Err(error) = repository.transfer(1, 2, 8_000) {
        println!("Rejected: {error}");
    }
    println!(
        "Unchanged: source={}, destination={}",
        repository.balance(1)?,
        repository.balance(2)?
    );

    repository.create(3, i64::MAX)?;
    if let Err(error) = repository.transfer(1, 3, 1) {
        println!("Rolled back after debit: {error}");
    }
    println!("Source after rollback: {}", repository.balance(1)?);
    Ok(())
}
