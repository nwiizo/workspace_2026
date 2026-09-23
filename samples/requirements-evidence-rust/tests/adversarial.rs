use requirements_evidence_rust::{
    AccountId, ActorId, Ledger, LedgerBuildError, MAX_TRANSFER_INPUT_BYTES, ParseTransferError,
    ParsedTransfer, RequestId, TransferAmount, TransferError,
};

fn actor(value: u64) -> ActorId {
    ActorId::new(value).expect("attack fixtures use non-zero actor IDs")
}

fn account(value: u64) -> AccountId {
    AccountId::new(value).expect("attack fixtures use non-zero account IDs")
}

fn transfer(request: u64, source: u64, destination: u64, amount: u64) -> ParsedTransfer {
    ParsedTransfer::new(
        RequestId::new(request).expect("attack request IDs are non-zero"),
        account(source),
        account(destination),
        TransferAmount::new(amount).expect("attack amount is valid at this layer"),
    )
    .expect("attack fixture accounts are distinct")
}

#[test]
fn oversized_non_utf8_input_is_rejected_before_decoding() {
    let payload = vec![0xff; MAX_TRANSFER_INPUT_BYTES + 1];

    assert_eq!(
        ParsedTransfer::parse(&payload),
        Err(ParseTransferError::InputTooLong)
    );
}

#[test]
fn numeric_overflow_is_classified_without_panicking() {
    assert_eq!(
        ParsedTransfer::parse(b"18446744073709551616,10,20,1"),
        Err(ParseTransferError::InvalidRequestId)
    );
    assert_eq!(
        ParsedTransfer::parse(b"1,10,20,18446744073709551616"),
        Err(ParseTransferError::InvalidAmount)
    );
}

#[test]
fn attacker_cannot_debit_another_actors_account() {
    let mut ledger = Ledger::new(&[(account(10), actor(1), 1_000), (account(20), actor(2), 300)])
        .expect("fixture account IDs are unique");

    assert_eq!(
        ledger
            .authorize(actor(9), transfer(1, 10, 20, 500))
            .unwrap_err(),
        TransferError::UnauthorizedSource
    );
    assert_eq!(ledger.balance(account(10)), Some(1_000));
    assert_eq!(ledger.balance(account(20)), Some(300));
}

#[test]
fn missing_accounts_are_not_confused_with_failed_authorization() {
    let mut missing_source =
        Ledger::new(&[(account(20), actor(2), 300)]).expect("fixture is valid");
    assert_eq!(
        missing_source
            .authorize(actor(1), transfer(1, 10, 20, 1))
            .unwrap_err(),
        TransferError::SourceAccountNotFound
    );

    let mut missing_destination =
        Ledger::new(&[(account(10), actor(1), 300)]).expect("fixture is valid");
    assert_eq!(
        missing_destination
            .authorize(actor(1), transfer(1, 10, 20, 1))
            .unwrap_err(),
        TransferError::DestinationAccountNotFound
    );
}

#[test]
fn duplicate_account_rows_are_rejected_during_restore() {
    assert_eq!(
        Ledger::new(&[(account(10), actor(1), 100), (account(10), actor(2), 900),]).unwrap_err(),
        LedgerBuildError::DuplicateAccount(account(10))
    );
}

#[test]
fn changed_payload_cannot_reuse_a_completed_request_id() {
    let mut ledger = Ledger::new(&[(account(10), actor(1), 1_000), (account(20), actor(2), 300)])
        .expect("fixture account IDs are unique");
    ledger
        .authorize(actor(1), transfer(7, 10, 20, 500))
        .expect("the owner is authorized")
        .execute()
        .expect("the first request applies");

    assert_eq!(
        ledger
            .authorize(actor(1), transfer(7, 10, 20, 1))
            .expect("the owner is authorized")
            .execute(),
        Err(TransferError::RequestIdConflict)
    );
    assert_eq!(ledger.balance(account(10)), Some(500));
    assert_eq!(ledger.balance(account(20)), Some(800));
}
