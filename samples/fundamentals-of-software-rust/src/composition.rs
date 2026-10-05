//! Chapter 3: a vehicle contains a powertrain instead of inheriting an engine.

#[derive(Debug, Clone, Copy)]
pub enum Powertrain {
    Combustion { fuel_liters: u16, km_per_liter: u16 },
    Electric { charge_kwh: u16, km_per_kwh: u16 },
}

impl Powertrain {
    /// A deliberately simple estimate, not a physical vehicle simulation.
    pub fn range_km(self) -> u32 {
        match self {
            Self::Combustion {
                fuel_liters,
                km_per_liter,
            } => u32::from(fuel_liters) * u32::from(km_per_liter),
            Self::Electric {
                charge_kwh,
                km_per_kwh,
            } => u32::from(charge_kwh) * u32::from(km_per_kwh),
        }
    }
}

#[derive(Debug)]
pub struct Vehicle {
    pub powertrain: Powertrain,
}

impl Vehicle {
    pub fn range_km(&self) -> u32 {
        self.powertrain.range_km()
    }
}
