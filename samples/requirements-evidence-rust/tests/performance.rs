use std::time::{Duration, Instant};

use requirements_evidence_rust::{
    AccountId, ActorId, Ledger, ParsedTransfer, RequestId, TransferAmount,
};

const TRANSFERS: u64 = 100_000;

#[test]
#[ignore = "run explicitly with a release build on a named machine"]
fn qr3_executes_one_hundred_thousand_transfers_within_two_seconds() {
    let source = AccountId::new(10).expect("ten is non-zero");
    let destination = AccountId::new(20).expect("twenty is non-zero");
    let owner = ActorId::new(1).expect("one is non-zero");
    let amount = TransferAmount::new(1).expect("one is a valid amount");
    let mut ledger = Ledger::new(&[
        (source, owner, TRANSFERS),
        (destination, ActorId::new(2).expect("two is non-zero"), 0),
    ])
    .expect("fixture account IDs are unique");

    let started = Instant::now();
    for request in 1..=TRANSFERS {
        let transfer = ParsedTransfer::new(
            RequestId::new(request).expect("the range starts at one"),
            source,
            destination,
            amount,
        )
        .expect("fixture accounts are distinct");
        ledger
            .authorize(owner, transfer)
            .expect("the owner is authorized")
            .execute()
            .expect("the generated transfer is valid");
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "{TRANSFERS} transfers took {elapsed:?}"
    );
}
