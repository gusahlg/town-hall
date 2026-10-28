//! Shared physical and technological knowledge. Price discovery and exchange
//! are intentionally separate future systems.

mod goods;
mod needs;
mod recipes;
mod resources;

pub use goods::*;
pub use needs::*;
pub use recipes::*;
pub use resources::*;

use std::collections::{BTreeMap, HashMap};

use anyhow::{Result, ensure};

use goods::canonical_amounts;

#[derive(Debug)]
pub struct Economy {
    resource_unit_goods: Vec<GoodId>,
    goods: Vec<GoodEntry>,
    good_ids: HashMap<Good, GoodId>,
}

impl Economy {
    /// Every possible `u8` is a valid primitive resource, so the unit good of
    /// each lattice point exists from the start.
    pub fn new() -> Self {
        let mut economy = Self {
            resource_unit_goods: Vec::with_capacity(RESOURCE_COUNT),
            goods: Vec::new(),
            good_ids: HashMap::new(),
        };
        for raw in 0..=u8::MAX {
            let unit_good = economy
                .register_good(vec![Component {
                    resource: ResourceId(raw),
                    quantity: 1,
                }])
                .expect("the good ID space holds every resource unit good");
            economy.resource_unit_goods.push(unit_good);
        }
        economy
    }

    /// The canonical `[(resource, 1)]` configuration used when one extracted
    /// unit enters the economy. Provenance does not affect this `GoodId`.
    pub fn resource_unit_good(&self, resource: ResourceId) -> GoodId {
        self.resource_unit_goods[resource.index()]
    }

    /// Registers a design by its exact expanded resource sequence, merging
    /// adjacent identical runs before looking up its ID. Empty configurations
    /// and zero quantities are rejected before the registry changes.
    /// This does not produce an inventory unit; manufacture still requires a
    /// recipe and process.
    ///
    /// # Panics
    /// Panics if merging an adjacent resource run exceeds `u32::MAX`.
    pub fn register_good(&mut self, configuration: Vec<Component>) -> Result<GoodId> {
        let good = Good::new(configuration)?;
        if let Some(&id) = self.good_ids.get(&good) {
            return Ok(id);
        }
        let id = GoodId::from_index(self.goods.len())
            .ok_or_else(|| anyhow::anyhow!("good ID space is exhausted"))?;
        self.good_ids.insert(good.clone(), id);
        self.goods.push(GoodEntry {
            good,
            recipes: Vec::new(),
        });
        Ok(id)
    }

    #[inline]
    pub fn good(&self, id: GoodId) -> Option<&Good> {
        self.goods.get(id.index()).map(|entry| &entry.good)
    }

    /// Registers a net recipe for one unit of `output`. Inputs are canonicalized
    /// as an unordered multiset and must contain exactly the output's primitive
    /// material. Process-specific consumables and losses are not part of this
    /// accounting.
    pub fn register_recipe(&mut self, output: GoodId, inputs: Vec<GoodAmount>) -> Result<RecipeId> {
        ensure!(
            self.good(output).is_some(),
            "unknown output good {output:?}"
        );
        let inputs = canonical_amounts(inputs)?;
        for input in &inputs {
            ensure!(
                self.good(input.good).is_some(),
                "unknown input good {:?}",
                input.good
            );
        }
        ensure!(
            self.material_totals(&inputs)? == self.output_material_totals(output)?,
            "recipe does not conserve primitive material"
        );

        let recipes = &mut self.goods[output.index()].recipes;
        if let Some(index) = recipes.iter().position(|recipe| recipe.inputs() == inputs) {
            return recipe_id(output, index);
        }
        let index = recipes.len();
        recipes.push(Recipe::new(inputs));
        recipe_id(output, index)
    }

    pub fn register_process(&mut self, recipe: RecipeId, process: Process) -> Result<()> {
        for requirement in process
            .equipment()
            .iter()
            .chain(process.consumables())
            .chain(process.inefficiency_inputs())
        {
            ensure!(
                self.good(requirement.good).is_some(),
                "process refers to unknown good {:?}",
                requirement.good
            );
        }
        self.recipe_mut(recipe)?.add_process(process);
        Ok(())
    }

    pub fn recipe(&self, id: RecipeId) -> Option<&Recipe> {
        self.goods
            .get(id.output.index())?
            .recipes
            .get(id.index as usize)
    }

    pub fn recipes_for(&self, output: GoodId) -> Option<&[Recipe]> {
        self.goods
            .get(output.index())
            .map(|entry| entry.recipes.as_slice())
    }

    fn recipe_mut(&mut self, id: RecipeId) -> Result<&mut Recipe> {
        self.goods
            .get_mut(id.output.index())
            .and_then(|entry| entry.recipes.get_mut(id.index as usize))
            .ok_or_else(|| anyhow::anyhow!("unknown recipe {id:?}"))
    }

    fn material_totals(&self, inputs: &[GoodAmount]) -> Result<BTreeMap<ResourceId, u64>> {
        let mut totals = BTreeMap::new();
        for input in inputs {
            let good = self
                .good(input.good)
                .ok_or_else(|| anyhow::anyhow!("unknown input good {:?}", input.good))?;
            add_material(&mut totals, good, input.quantity)?;
        }
        Ok(totals)
    }

    fn output_material_totals(&self, output: GoodId) -> Result<BTreeMap<ResourceId, u64>> {
        let mut totals = BTreeMap::new();
        let good = self
            .good(output)
            .ok_or_else(|| anyhow::anyhow!("unknown output good {output:?}"))?;
        add_material(&mut totals, good, 1)?;
        Ok(totals)
    }
}

impl Default for Economy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct GoodEntry {
    good: Good,
    recipes: Vec<Recipe>,
}

fn add_material(
    totals: &mut BTreeMap<ResourceId, u64>,
    good: &Good,
    multiplier: Quantity,
) -> Result<()> {
    for component in good.configuration() {
        let amount = u64::from(component.quantity)
            .checked_mul(u64::from(multiplier))
            .ok_or_else(|| anyhow::anyhow!("primitive material quantity overflow"))?;
        if amount == 0 {
            continue;
        }
        let total = totals.entry(component.resource).or_default();
        *total = total
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("primitive material quantity overflow"))?;
    }
    Ok(())
}

fn recipe_id(output: GoodId, index: usize) -> Result<RecipeId> {
    Ok(RecipeId {
        output,
        index: u32::try_from(index).map_err(|_| anyhow::anyhow!("recipe ID space is exhausted"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn components(runs: &[(u8, Quantity)]) -> Vec<Component> {
        runs.iter()
            .map(|&(resource, quantity)| Component {
                resource: ResourceId(resource),
                quantity,
            })
            .collect()
    }

    #[test]
    fn good_identity_merges_adjacent_runs_and_preserves_order() {
        let mut economy = Economy::new();
        let canonical = components(&[(0, 5), (1, 3), (0, 2)]);
        let id = economy.register_good(canonical.clone()).unwrap();
        let equivalent = economy
            .register_good(components(&[
                (0, 1),
                (0, 2),
                (0, 2),
                (1, 1),
                (1, 2),
                (0, 1),
                (0, 1),
            ]))
            .unwrap();
        assert_eq!(id, equivalent);
        assert_eq!(economy.good(id).unwrap().configuration(), canonical);

        // All three use the same primitive totals, but different positions.
        for different in [
            components(&[(0, 7), (1, 3)]),
            components(&[(1, 3), (0, 7)]),
            components(&[(0, 2), (1, 3), (0, 5)]),
        ] {
            assert_ne!(id, economy.register_good(different).unwrap());
        }
    }

    #[test]
    fn equivalent_encodings_share_recipes_and_conserve_material() {
        let mut economy = Economy::new();
        let output = economy
            .register_good(components(&[(0, 3), (0, 2)]))
            .unwrap();
        let unit = economy.resource_unit_good(ResourceId(0));
        let inputs = vec![GoodAmount {
            good: unit,
            quantity: 5,
        }];
        let recipe = economy.register_recipe(output, inputs.clone()).unwrap();
        let goods_before = economy.goods.len();

        let equivalent = economy.register_good(components(&[(0, 5)])).unwrap();
        assert_eq!(output, equivalent);
        assert_eq!(economy.goods.len(), goods_before);
        assert_eq!(
            economy.good(equivalent).unwrap().configuration(),
            components(&[(0, 5)])
        );
        assert_eq!(economy.register_recipe(equivalent, inputs).unwrap(), recipe);
        assert_eq!(economy.recipes_for(equivalent).unwrap().len(), 1);
        assert!(
            economy
                .register_recipe(
                    output,
                    vec![GoodAmount {
                        good: unit,
                        quantity: 4
                    }]
                )
                .is_err()
        );
        assert!(
            economy
                .register_recipe(
                    output,
                    vec![GoodAmount {
                        good: unit,
                        quantity: 6
                    }]
                )
                .is_err()
        );
    }

    #[test]
    fn invalid_goods_do_not_receive_ids_or_change_the_registry() {
        let mut economy = Economy::new();
        let goods_before = economy.goods.len();
        let ids_before = economy.good_ids.clone();
        for invalid in [
            vec![],
            components(&[(0, 0)]),
            components(&[(0, 1), (0, 0)]),
            components(&[(0, 0), (0, 1)]),
            components(&[(0, 3), (1, 0), (0, 2)]),
        ] {
            assert!(economy.register_good(invalid).is_err());
            assert_eq!(economy.goods.len(), goods_before);
            assert_eq!(economy.good_ids, ids_before);
        }
        let next = economy.register_good(components(&[(0, 2)])).unwrap();
        assert_eq!(next.index(), goods_before);
    }

    #[test]
    fn maximum_runs_stay_compressed_and_separated_runs_are_not_added() {
        let mut economy = Economy::new();
        let id = economy
            .register_good(components(&[
                (0, Quantity::MAX - 1),
                (0, 1),
                (1, 1),
                (0, Quantity::MAX),
            ]))
            .unwrap();
        assert_eq!(
            economy.good(id).unwrap().configuration(),
            components(&[(0, Quantity::MAX), (1, 1), (0, Quantity::MAX)])
        );
    }

    #[test]
    #[should_panic(expected = "adjacent resource run quantity exceeds u32::MAX")]
    fn overflowing_adjacent_run_panics() {
        let mut economy = Economy::new();
        let _ = economy.register_good(components(&[(0, Quantity::MAX), (0, 1)]));
    }

    #[test]
    fn inventory_multiplicity_is_not_product_structure() {
        let mut economy = Economy::new();
        let resource = ResourceId(0);
        let unit = economy.resource_unit_good(resource);
        let duplicate_unit = economy
            .register_good(vec![Component {
                resource,
                quantity: 1,
            }])
            .unwrap();
        let combined = economy
            .register_good(vec![Component {
                resource,
                quantity: 50,
            }])
            .unwrap();

        assert_eq!(unit, duplicate_unit);
        assert_ne!(unit, combined);
        let mut inventory = Inventory::default();
        inventory.set(unit, 50);
        inventory.set(combined, 1);
        assert_eq!(inventory.quantity(unit), 50);
        assert_eq!(inventory.quantity(combined), 1);

        let recipe = economy
            .register_recipe(
                combined,
                vec![GoodAmount {
                    good: unit,
                    quantity: 50,
                }],
            )
            .unwrap();
        economy
            .register_process(recipe, Process::new(1, 1, vec![], vec![], vec![]).unwrap())
            .unwrap();
        assert_eq!(economy.recipe(recipe).unwrap().processes().len(), 1);
    }
}
