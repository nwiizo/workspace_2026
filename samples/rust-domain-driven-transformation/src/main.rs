use rust_domain_driven_transformation::{
    CinemaError, ShowApi, legacy::LegacyShow, migration::MigratingShow,
};

fn main() -> Result<(), CinemaError> {
    let mut legacy = LegacyShow::new(42, "Cinema at Dawn".into(), 3)?;
    legacy.publish()?;
    legacy.reserve(2)?;
    let source = legacy.snapshot();
    println!("Before migration: {source:?}");

    let mut migrated = MigratingShow::from_snapshot(&source)?;
    println!("After migration:  {:?}", migrated.snapshot());
    if let Err(error) = migrated.reserve(2) {
        println!("Existing reservation is protected: {error}");
    }
    migrated.reserve(1)?;
    println!("Next reservation: {:?}", migrated.snapshot().reserved_seats);
    Ok(())
}
