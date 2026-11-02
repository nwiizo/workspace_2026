//! Bounded exploration of old/new behavior, including migration between operations.

use rust_domain_driven_transformation::{
    CinemaError, ShowApi, legacy::LegacyShow, migration::MigratingShow,
};

#[derive(Clone, Copy, Debug)]
enum Operation {
    Publish,
    Reserve(u16),
}

impl Operation {
    fn apply(self, show: &mut impl ShowApi) -> Result<(), CinemaError> {
        match self {
            Self::Publish => show.publish(),
            Self::Reserve(seat) => show.reserve(seat),
        }
    }
}

#[test]
fn every_five_operation_sequence_preserves_behavior_across_migration() -> Result<(), CinemaError> {
    // Three seats: 0 and 4 are invalid, while 1, 2 and 3 include every valid seat.
    const OPERATIONS: [Operation; 6] = [
        Operation::Publish,
        Operation::Reserve(0),
        Operation::Reserve(1),
        Operation::Reserve(2),
        Operation::Reserve(3),
        Operation::Reserve(4),
    ];
    // 6^5 = 7,776 sequences. Prefixes also cover all shorter sequences.
    for sequence in 0..OPERATIONS.len().pow(5) {
        let mut encoded = sequence;
        let mut legacy = LegacyShow::new(42, "Cinema at Dawn".into(), 3)?;
        let mut modern = MigratingShow::new(42, "Cinema at Dawn".into(), 3)?;
        for step in 0..5 {
            let operation = OPERATIONS[encoded % OPERATIONS.len()];
            encoded /= OPERATIONS.len();
            let before = legacy.snapshot();
            let mut restored = MigratingShow::from_snapshot(&before)?;
            assert_eq!(restored.snapshot(), before);

            let expected = operation.apply(&mut legacy);
            assert_eq!(
                operation.apply(&mut modern),
                expected,
                "sequence {sequence}, step {step}, operation {operation:?}"
            );
            assert_eq!(operation.apply(&mut restored), expected);
            let after = legacy.snapshot();
            assert_eq!(modern.snapshot(), after);
            assert_eq!(restored.snapshot(), after);
            if expected.is_err() {
                assert_eq!(after, before, "a rejected operation must preserve state");
            }
        }
    }
    Ok(())
}

#[test]
fn constructor_validation_order_matches_the_existing_api() {
    let cases = [
        (0, "", 0, CinemaError::InvalidShowId),
        (1, " \t\n", 0, CinemaError::EmptyTitle),
        (1, "Cinema at Dawn", 0, CinemaError::InvalidCapacity),
    ];
    for (id, title, capacity, expected) in cases {
        assert_eq!(
            LegacyShow::new(id, title.into(), capacity).err(),
            Some(expected)
        );
        assert_eq!(
            MigratingShow::new(id, title.into(), capacity).err(),
            LegacyShow::new(id, title.into(), capacity).err()
        );
    }
}

#[test]
fn migration_preserves_identity_and_title_and_sorts_seats() -> Result<(), CinemaError> {
    let mut legacy = LegacyShow::new(u32::MAX, " 夜明けの映画館 ".into(), 3)?;
    legacy.publish()?;
    legacy.reserve(3)?;
    legacy.reserve(1)?;
    let expected = legacy.snapshot();
    let mut source = expected.clone();
    source.reserved_seats.reverse();
    let restored = MigratingShow::from_snapshot(&source)?;
    assert_eq!(restored.snapshot(), expected);
    assert_eq!(source.reserved_seats, vec![3, 1]);
    assert_eq!(restored.snapshot().title, " 夜明けの映画館 ");
    Ok(())
}

#[test]
fn two_restored_values_do_not_coordinate_reservations() -> Result<(), CinemaError> {
    // A counterexample to treating exclusive borrows as a system-wide booking lock.
    let mut legacy = LegacyShow::new(42, "Cinema at Dawn".into(), 3)?;
    legacy.publish()?;
    let source = legacy.snapshot();
    let mut first = MigratingShow::from_snapshot(&source)?;
    let mut second = MigratingShow::from_snapshot(&source)?;
    first.reserve(1)?;
    second.reserve(1)?;
    assert_eq!(first.snapshot().reserved_seats, vec![1]);
    assert_eq!(second.snapshot().reserved_seats, vec![1]);
    assert!(legacy.snapshot().reserved_seats.is_empty());
    Ok(())
}
