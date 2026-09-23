#![no_main]

use libfuzzer_sys::fuzz_target;
use requirements_evidence_rust::{MAX_TRANSFER_AMOUNT, MAX_TRANSFER_INPUT_BYTES, ParsedTransfer};

fuzz_target!(|input: &[u8]| {
    if let Ok(transfer) = ParsedTransfer::parse(input) {
        assert!(input.len() <= MAX_TRANSFER_INPUT_BYTES);
        assert_ne!(transfer.source(), transfer.destination());
        assert!((1..=MAX_TRANSFER_AMOUNT).contains(&transfer.amount().get()));
    }
});
