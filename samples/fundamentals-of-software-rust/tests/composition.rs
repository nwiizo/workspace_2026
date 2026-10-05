use fundamentals_of_software_rust::composition::{Powertrain, Vehicle};

#[test]
fn vehicles_delegate_range_to_different_powertrains() {
    let car = Vehicle {
        powertrain: Powertrain::Combustion {
            fuel_liters: 40,
            km_per_liter: 15,
        },
    };
    let ev = Vehicle {
        powertrain: Powertrain::Electric {
            charge_kwh: 60,
            km_per_kwh: 6,
        },
    };
    assert_eq!(car.range_km(), 600);
    assert_eq!(ev.range_km(), 360);
}

#[test]
fn an_empty_battery_has_no_range() {
    let ev = Vehicle {
        powertrain: Powertrain::Electric {
            charge_kwh: 0,
            km_per_kwh: 6,
        },
    };
    assert_eq!(ev.range_km(), 0);
}
