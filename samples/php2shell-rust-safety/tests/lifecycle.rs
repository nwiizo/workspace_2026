use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn safe_vec_keeps_only_initialized_elements_after_panic() {
    let mut values = Vec::with_capacity(2);
    let interrupted = catch_unwind(AssertUnwindSafe(|| {
        values.extend([7_u8, 8].into_iter().inspect(|&value| {
            assert_ne!(value, 8, "injected interruption before the second element");
        }));
    }));

    assert!(interrupted.is_err());
    assert_eq!(values, [7]);
    assert_eq!(values.get(1), None);

    // This local value is still memory-safe to use after unwinding. Its partial
    // contents also show that catch_unwind did not roll back the operation.
    values.push(9);
    assert_eq!(values, [7, 9]);
}

#[test]
fn bounds_check_stops_an_invalid_read_but_does_not_prevent_a_panic() {
    let values = [7_u8];
    let index = std::hint::black_box(1);

    let result = catch_unwind(|| values[index]);

    assert!(result.is_err());
    assert_eq!(values.get(index), None);
}
