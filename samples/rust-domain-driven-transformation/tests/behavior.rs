use rust_domain_driven_transformation::{
    CinemaError, LegacySnapshot, ShowApi,
    legacy::LegacyShow,
    migration::MigratingShow,
    planning::{ScheduledShow, ShowId},
    sales::{SeatNumber, SeatingPlan},
};

fn characterize(mut show: impl ShowApi) -> Result<(), CinemaError> {
    let draft = show.snapshot();
    assert_eq!(show.reserve(1), Err(CinemaError::NotPublished));
    assert_eq!(show.snapshot(), draft);

    show.publish()?;
    show.reserve(3)?;
    show.reserve(1)?;
    let expected = LegacySnapshot {
        id: 42,
        title: "Cinema at Dawn".into(),
        capacity: 3,
        published: true,
        reserved_seats: vec![1, 3],
    };
    assert_eq!(show.snapshot(), expected);
    assert_eq!(show.reserve(3), Err(CinemaError::SeatTaken(3)));
    assert_eq!(show.reserve(0), Err(CinemaError::InvalidSeat(0)));
    assert_eq!(show.reserve(4), Err(CinemaError::InvalidSeat(4)));
    assert_eq!(show.publish(), Err(CinemaError::AlreadyPublished));
    assert_eq!(show.snapshot(), expected);
    Ok(())
}

#[test]
fn legacy_behavior_is_fixed_before_extraction() -> Result<(), CinemaError> {
    characterize(LegacyShow::new(42, "Cinema at Dawn".into(), 3)?)
}

#[test]
fn a_published_show_can_be_migrated_with_existing_reservations() -> Result<(), CinemaError> {
    let mut legacy = LegacyShow::new(42, "Cinema at Dawn".into(), 3)?;
    legacy.publish()?;
    legacy.reserve(2)?;
    let source = legacy.snapshot();
    let mut migrated = MigratingShow::from_snapshot(&source)?;
    assert_eq!(migrated.snapshot(), source);
    assert_eq!(migrated.reserve(2), Err(CinemaError::SeatTaken(2)));
    migrated.reserve(1)?;
    legacy.reserve(1)?;
    assert_eq!(migrated.snapshot(), legacy.snapshot());
    Ok(())
}

#[test]
fn extracted_models_keep_the_old_api_behavior() -> Result<(), CinemaError> {
    characterize(MigratingShow::new(42, "Cinema at Dawn".into(), 3)?)
}

#[test]
fn draft_migration_does_not_open_reservations() -> Result<(), CinemaError> {
    let source = LegacyShow::new(42, "Cinema at Dawn".into(), 3)?.snapshot();
    let mut migrated = MigratingShow::from_snapshot(&source)?;
    assert_eq!(migrated.snapshot(), source);
    assert_eq!(migrated.reserve(1), Err(CinemaError::NotPublished));
    migrated.publish()?;
    migrated.reserve(1)?;
    assert_eq!(migrated.snapshot().reserved_seats, vec![1]);
    Ok(())
}

#[test]
fn invalid_snapshots_are_rejected_without_changing_the_source() {
    let base = LegacySnapshot {
        id: 42,
        title: "Cinema at Dawn".into(),
        capacity: 3,
        published: true,
        reserved_seats: vec![1],
    };
    let cases = [
        (
            LegacySnapshot {
                id: 0,
                ..base.clone()
            },
            CinemaError::InvalidShowId,
        ),
        (
            LegacySnapshot {
                title: " \t".into(),
                ..base.clone()
            },
            CinemaError::EmptyTitle,
        ),
        (
            LegacySnapshot {
                capacity: 0,
                ..base.clone()
            },
            CinemaError::InvalidCapacity,
        ),
        (
            LegacySnapshot {
                published: false,
                ..base.clone()
            },
            CinemaError::ReservationsBeforePublication,
        ),
        (
            LegacySnapshot {
                reserved_seats: vec![1, 1],
                ..base.clone()
            },
            CinemaError::SeatTaken(1),
        ),
        (
            LegacySnapshot {
                reserved_seats: vec![1, 4],
                ..base.clone()
            },
            CinemaError::InvalidSeat(4),
        ),
        (
            LegacySnapshot {
                reserved_seats: vec![0],
                ..base
            },
            CinemaError::InvalidSeat(0),
        ),
    ];
    for (source, expected_error) in cases {
        let original = source.clone();
        assert_eq!(
            MigratingShow::from_snapshot(&source).err(),
            Some(expected_error)
        );
        assert_eq!(source, original);
    }
}

#[test]
fn replay_does_not_erase_reservations_and_conflicts_do_not_change_them() -> Result<(), CinemaError>
{
    let event = ScheduledShow::new(42, "Cinema at Dawn".into(), 3)?.publish()?;
    assert_eq!(event.show_id().get(), 42);
    let mut sales = SeatingPlan::open(event);
    sales.reserve(SeatNumber::try_from(2)?)?;
    sales.apply_publication(event)?;
    let different_id = ScheduledShow::new(99, "Cinema at Dawn".into(), 3)?.publish()?;
    let different_capacity = ScheduledShow::new(42, "Cinema at Dawn".into(), 4)?.publish()?;
    for conflicting_event in [different_id, different_capacity] {
        assert_eq!(
            sales.apply_publication(conflicting_event),
            Err(CinemaError::ConflictingPublication)
        );
        assert_eq!(sales.reserved_seats(), vec![2]);
    }
    assert_eq!(
        sales.reserve(SeatNumber::try_from(2)?),
        Err(CinemaError::SeatTaken(2))
    );
    Ok(())
}

#[test]
fn primitive_values_and_contextual_limits_are_checked_separately() -> Result<(), CinemaError> {
    assert_eq!(ShowId::try_from(0), Err(CinemaError::InvalidShowId));
    assert_eq!(SeatNumber::try_from(0), Err(CinemaError::InvalidSeat(0)));
    let event = ScheduledShow::new(u32::MAX, "Max".into(), u16::MAX)?.publish()?;
    let mut sales = SeatingPlan::open(event);
    sales.reserve(SeatNumber::try_from(u16::MAX)?)?;
    assert_eq!(sales.reserved_seats(), vec![u16::MAX]);
    let small_event = ScheduledShow::new(1, "Small".into(), 1)?.publish()?;
    let mut small = SeatingPlan::open(small_event);
    assert_eq!(
        small.reserve(SeatNumber::try_from(2)?),
        Err(CinemaError::InvalidSeat(2))
    );
    assert!(small.reserved_seats().is_empty());
    Ok(())
}
