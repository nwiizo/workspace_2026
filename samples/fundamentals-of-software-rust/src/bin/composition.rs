use fundamentals_of_software_rust::composition::{Powertrain, Vehicle};

fn main() {
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
    println!("Combustion vehicle: {} km", car.range_km());
    println!("Electric vehicle: {} km", ev.range_km());
}
