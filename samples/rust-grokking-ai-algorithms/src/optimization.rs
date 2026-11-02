//! Compatibility entry point for the original OneMax example.
use crate::evolution::{GeneticConfig, binary_genetic};

pub fn genetic_one_max(
    width: usize,
    population_size: usize,
    generations: usize,
    mutation_rate: f64,
    seed: u64,
) -> crate::Result<Vec<bool>> {
    binary_genetic(
        width,
        GeneticConfig {
            population: population_size,
            generations,
            mutation: mutation_rate,
        },
        seed,
        |bits| bits.iter().filter(|&&v| v).count() as f64,
    )
    .map(|result| result.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn genetic_search_is_repeatable_and_improves_fitness() {
        let first = genetic_one_max(24, 32, 100, 0.03, 42).unwrap();
        let second = genetic_one_max(24, 32, 100, 0.03, 42).unwrap();
        assert_eq!(first, second);
        assert!(first.iter().filter(|&&v| v).count() >= 20);
    }
    #[test]
    fn genetic_search_rejects_invalid_configuration() {
        assert!(genetic_one_max(0, 10, 2, 0.1, 1).is_err());
        assert!(genetic_one_max(4, 1, 2, 0.1, 1).is_err());
        assert!(genetic_one_max(4, 2, 2, 1.1, 1).is_err());
    }
}
