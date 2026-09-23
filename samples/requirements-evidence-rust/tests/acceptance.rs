use requirements_evidence_rust::{
    AccountId, ActorId, Disposition, Ledger, ParsedTransfer, RequestId, TransferAmount,
    TransferError,
};

fn actor(value: u64) -> ActorId {
    ActorId::new(value).expect("test actor IDs are non-zero")
}

fn account(value: u64) -> AccountId {
    AccountId::new(value).expect("test account IDs are non-zero")
}

fn transfer(request: u64, source: u64, destination: u64, amount: u64) -> ParsedTransfer {
    ParsedTransfer::new(
        RequestId::new(request).expect("test request IDs are non-zero"),
        account(source),
        account(destination),
        TransferAmount::new(amount).expect("test amounts are valid"),
    )
    .expect("test accounts are distinct")
}

fn ledger(source_balance: u64, destination_balance: u64) -> Ledger {
    Ledger::new(&[
        (account(10), actor(1), source_balance),
        (account(20), actor(2), destination_balance),
    ])
    .expect("test account IDs are unique")
}

#[test]
fn fr1_parses_a_valid_transfer() {
    let parsed = ParsedTransfer::parse(b"7,10,20,500");

    assert!(parsed.is_ok());
}

#[test]
fn fr1_rejects_invalid_transfer_inputs() {
    for input in [
        "0,10,20,500",
        "7,0,20,500",
        "7,10,0,500",
        "7,10,10,500",
        "7,10,20,0",
        "7,10,20,1000001",
        "7,10,20",
        "7,10,20,500,extra",
        "7,source,20,500",
    ] {
        assert!(
            ParsedTransfer::parse(input.as_bytes()).is_err(),
            "input must be rejected: {input}"
        );
    }
}

#[test]
fn fr2_only_the_source_owner_is_authorized() {
    let request = transfer(7, 10, 20, 500);
    let mut owned = ledger(1_000, 300);
    assert!(owned.authorize(actor(1), request).is_ok());

    let mut attacked = ledger(1_000, 300);
    assert_eq!(
        attacked.authorize(actor(9), request).unwrap_err(),
        TransferError::UnauthorizedSource
    );
    assert_eq!(attacked.balance(account(10)), Some(1_000));
    assert_eq!(attacked.balance(account(20)), Some(300));
}

#[test]
fn fr3_moves_the_exact_amount_and_preserves_the_sum() {
    let mut ledger = ledger(1_000, 300);

    let receipt = ledger
        .authorize(actor(1), transfer(7, 10, 20, 500))
        .expect("the owner must be authorized")
        .execute()
        .expect("the balances can accept the transfer");

    assert_eq!(receipt.disposition(), Disposition::Applied);
    assert_eq!(ledger.balance(account(10)), Some(500));
    assert_eq!(ledger.balance(account(20)), Some(800));
}

#[test]
fn fr3_rejection_does_not_partially_change_balances() {
    let mut insufficient = ledger(499, 300);
    let result = insufficient
        .authorize(actor(1), transfer(7, 10, 20, 500))
        .expect("the owner must be authorized")
        .execute();
    assert_eq!(result, Err(TransferError::InsufficientFunds));
    assert_eq!(insufficient.balance(account(10)), Some(499));
    assert_eq!(insufficient.balance(account(20)), Some(300));

    let mut overflowing = ledger(1_000, u64::MAX);
    let result = overflowing
        .authorize(actor(1), transfer(8, 10, 20, 1))
        .expect("the owner must be authorized")
        .execute();
    assert_eq!(result, Err(TransferError::DestinationOverflow));
    assert_eq!(overflowing.balance(account(10)), Some(1_000));
    assert_eq!(overflowing.balance(account(20)), Some(u64::MAX));
}

#[test]
fn fr4_replays_identical_requests_without_a_second_debit() {
    let request = transfer(7, 10, 20, 500);
    let mut ledger = ledger(1_000, 300);

    let first = ledger
        .authorize(actor(1), request)
        .expect("the owner must be authorized")
        .execute()
        .expect("the first request must apply");
    let replay = ledger
        .authorize(actor(1), request)
        .expect("the owner must still be authorized")
        .execute()
        .expect("an identical retry must succeed");

    assert_eq!(first.disposition(), Disposition::Applied);
    assert_eq!(replay.disposition(), Disposition::Replayed);
    assert_eq!(ledger.balance(account(10)), Some(500));
    assert_eq!(ledger.balance(account(20)), Some(800));
}

#[test]
fn fr4_rejects_request_id_reuse_with_different_contents() {
    let mut ledger = ledger(1_000, 300);
    ledger
        .authorize(actor(1), transfer(7, 10, 20, 500))
        .expect("the owner must be authorized")
        .execute()
        .expect("the first request must apply");

    let result = ledger
        .authorize(actor(1), transfer(7, 10, 20, 400))
        .expect("the owner must be authorized")
        .execute();

    assert_eq!(result, Err(TransferError::RequestIdConflict));
    assert_eq!(ledger.balance(account(10)), Some(500));
    assert_eq!(ledger.balance(account(20)), Some(800));
}
