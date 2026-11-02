//! An intentionally unsound initialization sequence, executable only in Miri.
//! Models publishing a length before initialization and then being interrupted.

#[cfg(not(miri))]
fn main() -> std::process::ExitCode {
    eprintln!("Run with: rtk proxy cargo +nightly miri run --example unsafe_initialization");
    std::process::ExitCode::from(2)
}

#[cfg(miri)]
fn main() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let mut values = Vec::<u8>::with_capacity(2);
    let interrupted = catch_unwind(AssertUnwindSafe(|| {
        // INTENTIONALLY UNSOUND: the second element is not initialized.
        // Ordinary Vec APIs never allow callers to publish this false length.
        unsafe {
            values.set_len(2);
        }
        values[0] = 7;
        panic!("injected interruption before initializing the second element");
    }));
    assert!(interrupted.is_err());

    // Catching the panic does not restore the Vec's initialization invariant.
    // Miri must reject this read, not merely observe the caught panic above.
    std::hint::black_box(values[1]);
}
