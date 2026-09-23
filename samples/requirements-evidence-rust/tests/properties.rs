use proptest::prelude::*;
use requirements_evidence_rust::{
    AccountId, ActorId, Disposition, Ledger, MAX_TRANSFER_AMOUNT, ParsedTransfer, RequestId,
    TransferAmount, TransferError,
};

fn actor(value: u64) -> ActorId {
    ActorId::new(value).expect("generated fixtures use non-zero actor IDs")
}

fn account(value: u64) -> AccountId {
    AccountId::new(value).expect("generated fixtures use non-zero account IDs")
}

fn transfer(amount: u64) -> ParsedTransfer {
    ParsedTransfer::new(
        RequestId::new(1).expect("one is non-zero"),
        account(10),
        account(20),
        TransferAmount::new(amount).expect("the strategy generates a valid amount"),
    )
    .expect("fixture accounts are distinct")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn qr1_parser_never_panics_for_arbitrary_bytes(input in prop::collection::vec(any::<u8>(), 0..=512)) {
        let _ = ParsedTransfer::parse(&input);
    }

    #[test]
    fn fr3_transition_matches_its_preconditions(
        source_balance in any::<u64>(),
        destination_balance in any::<u64>(),
        amount in 1_u64..=MAX_TRANSFER_AMOUNT,
    ) {
        let mut ledger = Ledger::new(&[
            (account(10), actor(1), source_balance),
            (account(20), actor(2), destination_balance),
        ]).expect("fixture account IDs are unique");

        let result = ledger
            .authorize(actor(1), transfer(amount))
            .expect("the generated actor owns the source")
            .execute();

        if source_balance < amount {
            prop_assert_eq!(result, Err(TransferError::InsufficientFunds));
            prop_assert_eq!(ledger.balance(account(10)), Some(source_balance));
            prop_assert_eq!(ledger.balance(account(20)), Some(destination_balance));
        } else if destination_balance.checked_add(amount).is_none() {
            prop_assert_eq!(result, Err(TransferError::DestinationOverflow));
            prop_assert_eq!(ledger.balance(account(10)), Some(source_balance));
            prop_assert_eq!(ledger.balance(account(20)), Some(destination_balance));
        } else {
            let receipt = result.expect("preconditions allow the transfer");
            let new_source = ledger.balance(account(10)).expect("source still exists");
            let new_destination = ledger.balance(account(20)).expect("destination still exists");
            prop_assert_eq!(receipt.disposition(), Disposition::Applied);
            prop_assert_eq!(new_source, source_balance - amount);
            prop_assert_eq!(new_destination, destination_balance + amount);
            prop_assert_eq!(
                u128::from(new_source) + u128::from(new_destination),
                u128::from(source_balance) + u128::from(destination_balance)
            );
        }
    }

    #[test]
    fn fr4_identical_retry_is_idempotent(
        amount in 1_u64..=MAX_TRANSFER_AMOUNT,
        source_extra in 0_u64..=MAX_TRANSFER_AMOUNT,
        destination_balance in 0_u64..=MAX_TRANSFER_AMOUNT,
    ) {
        let source_balance = amount + source_extra;
        let request = transfer(amount);
        let mut ledger = Ledger::new(&[
            (account(10), actor(1), source_balance),
            (account(20), actor(2), destination_balance),
        ]).expect("fixture account IDs are unique");

        let first = ledger
            .authorize(actor(1), request)
            .expect("the generated actor owns the source")
            .execute()
            .expect("generated balances allow the transfer");
        let after_first = (
            ledger.balance(account(10)),
            ledger.balance(account(20)),
        );
        let retry = ledger
            .authorize(actor(1), request)
            .expect("the generated actor still owns the source")
            .execute()
            .expect("an identical retry succeeds");

        prop_assert_eq!(first.disposition(), Disposition::Applied);
        prop_assert_eq!(retry.disposition(), Disposition::Replayed);
        prop_assert_eq!(
            (ledger.balance(account(10)), ledger.balance(account(20))),
            after_first
        );
    }
}
