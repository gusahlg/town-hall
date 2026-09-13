//! A seeded baseline distribution, independent of resource-property geometry.
//! Extraction and recovery are future simulation rules; generation only places
//! unowned material and describes each deposit's extraction parameters.

use neuro_ash::Random;

use crate::{DEFAULT_RANDOM_SEED, ResourceId};

use super::{DepositProperties, ResourceDeposit, ResourceInfo, World};

pub trait Generator {
    fn generate(&self, width: u32, height: u32) -> World;
}

/// Independent tiles, with 1–3 deposits on each occupied tile and 1–3 distinct
/// resources per deposit. All 256 resource IDs are equally eligible; small
/// worlds need not contain every resource. Geography does not imply similarity.
///
/// Tile (0, 0), the first spawn position, always has a deposit requiring no gear.
/// This provides starting material even on a 1×1 map or at zero random density.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomGenerator {
    pub seed: u64,
    /// Chance in percent that a tile has deposits, in `0..=100`, excluding the
    /// guaranteed starting deposit. The default is 35%.
    pub deposit_chance: u8,
}

impl RandomGenerator {
    pub const fn new(seed: u64) -> Self {
        Self {
            seed,
            deposit_chance: 35,
        }
    }
}

impl Default for RandomGenerator {
    fn default() -> Self {
        Self::new(DEFAULT_RANDOM_SEED)
    }
}

impl Generator for RandomGenerator {
    fn generate(&self, width: u32, height: u32) -> World {
        assert!(
            self.deposit_chance <= 100,
            "deposit chance must be in 0..=100"
        );
        let mut world = World::empty(width, height);
        // A fresh stream makes generation repeatable and avoids consuming any
        // randomness used to initialize people or their neural networks.
        let mut random = Random::new(self.seed);
        for tile in &mut world.tiles {
            if between(&mut random, 0, 99) < u32::from(self.deposit_chance) {
                for _ in 0..between(&mut random, 1, 3) {
                    tile.deposits.push(deposit(&mut random));
                }
            }
        }

        let start = &mut world.tiles[0];
        if start.deposits.is_empty() {
            start.deposits.push(deposit(&mut random));
        }
        start.deposits[0].properties = DepositProperties {
            time: 1,
            parallelisation: 1,
            gear: 0,
            efficiency: 100,
            batch_size: 1,
        };
        world
    }
}

fn deposit(random: &mut Random) -> ResourceDeposit {
    let resource_count = between(random, 1, 3) as usize;
    let mut resources = Vec::<ResourceInfo>::with_capacity(resource_count);
    while resources.len() < resource_count {
        let resource_id = ResourceId(between(random, 0, 255) as u8);
        if resources.iter().any(|info| info.resource_id == resource_id) {
            continue;
        }
        resources.push(ResourceInfo {
            resource_id,
            quantity: between(random, 100, 10_000),
            recovery_rate: if between(random, 0, 3) == 0 {
                between(random, 1, 10) as u16
            } else {
                0
            },
        });
    }
    ResourceDeposit {
        resources,
        properties: DepositProperties {
            time: between(random, 1, 16) as u8,
            parallelisation: between(random, 1, 8) as u8,
            gear: between(random, 0, 4) as u8,
            efficiency: between(random, 25, 100) as u8,
            batch_size: between(random, 1, 16) as u8,
        },
    }
}

/// Inclusive small integer range using the simulation's deterministic RNG.
fn between(random: &mut Random, min: u32, max: u32) -> u32 {
    min + (random.next_f32() * (max - min + 1) as f32) as u32
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::Position;

    use super::*;

    #[test]
    fn generation_repeats_and_the_seed_changes_the_world() {
        let generator = RandomGenerator::new(42);
        let world = generator.generate(12, 7);
        assert_eq!(world, generator.generate(12, 7));
        assert_ne!(world, World::with_seed(12, 7, 43));
        assert_eq!((world.width(), world.height()), (12, 7));
        assert!(world.tile(Position::new(11, 6)).is_some());
        assert!(world.tile(Position::new(12, 6)).is_none());
        assert!(world.tile(Position::new(11, 7)).is_none());
        assert_eq!(
            World::new(12, 7),
            RandomGenerator::default().generate(12, 7)
        );
    }

    #[test]
    fn density_extremes_preserve_a_starting_deposit() {
        for (width, height) in [(1, 1), (1, 9), (9, 1), (12, 7)] {
            for deposit_chance in [0, 100] {
                let world = RandomGenerator {
                    seed: 0,
                    deposit_chance,
                }
                .generate(width, height);
                let occupied = world
                    .tiles
                    .iter()
                    .filter(|tile| !tile.deposits.is_empty())
                    .count();
                assert_eq!(
                    occupied,
                    if deposit_chance == 0 {
                        1
                    } else {
                        world.tiles.len()
                    }
                );
                let starter = &world.tile(Position::new(0, 0)).unwrap().deposits()[0];
                assert_eq!(starter.properties.gear, 0);
                assert_eq!(starter.properties.time, 1);
                assert_eq!(starter.properties.batch_size, 1);
                assert_eq!(starter.properties.efficiency, 100);
                assert!(starter.resources.iter().all(|info| info.quantity > 0));
            }
        }
    }

    #[test]
    fn deposits_have_distinct_materials_and_usable_parameters() {
        let world = World::default();
        let mut total = 0u128;
        let mut all_resources = HashSet::new();
        let mut renewable = false;
        let mut finite = false;
        let mut mixed = false;
        assert!(world.tiles.iter().any(|tile| tile.deposits.is_empty()));
        assert!(world.tiles.iter().any(|tile| tile.deposits.len() > 1));
        for tile in &world.tiles {
            assert!(tile.deposits.len() <= 3);
            for deposit in tile.deposits() {
                assert!((1..=3).contains(&deposit.resources.len()));
                let mut distinct = HashSet::new();
                for info in &deposit.resources {
                    assert!(distinct.insert(info.resource_id));
                    assert!((100..=10_000).contains(&info.quantity));
                    assert!(info.recovery_rate <= 10);
                    all_resources.insert(info.resource_id);
                    renewable |= info.recovery_rate > 0;
                    finite |= info.recovery_rate == 0;
                    total += u128::from(info.quantity);
                }
                mixed |= deposit.resources.len() > 1;
                let properties = &deposit.properties;
                assert!((1..=16).contains(&properties.time));
                assert!((1..=8).contains(&properties.parallelisation));
                assert!(properties.gear <= 4);
                assert!((25..=100).contains(&properties.efficiency));
                assert!((1..=16).contains(&properties.batch_size));
            }
        }
        // Coverage of the fixed default scenario, not a promise for every seed.
        assert_eq!(all_resources.len(), 256);
        assert!(mixed && renewable && finite);
        assert_eq!(world.total_deposited_material(), total);
    }

    #[test]
    fn empty_world_supports_hand_built_scenarios() {
        let mut world = World::empty(2, 3);
        assert_eq!(world.total_deposited_material(), 0);
        let deposit = deposit(&mut Random::new(5));
        let total: u128 = deposit
            .resources
            .iter()
            .map(|info| u128::from(info.quantity))
            .sum();
        world
            .tile_mut(Position::new(1, 2))
            .unwrap()
            .deposits_mut()
            .push(deposit);
        assert_eq!(world.total_deposited_material(), total);
        assert!(world.tile_mut(Position::new(2, 2)).is_none());
    }

    #[test]
    #[should_panic(expected = "world dimensions must be positive")]
    fn zero_width_is_rejected() {
        World::new(0, 1);
    }

    #[test]
    #[should_panic(expected = "world dimensions must be positive")]
    fn zero_height_is_rejected() {
        World::new(1, 0);
    }

    #[test]
    #[should_panic(expected = "deposit chance must be in 0..=100")]
    fn invalid_density_is_rejected() {
        RandomGenerator {
            seed: 1,
            deposit_chance: 101,
        }
        .generate(1, 1);
    }
}
