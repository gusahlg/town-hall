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
    resources: Resources,
    resource_unit_goods: Vec<GoodId>,
    goods: Vec<GoodEntry>,
    good_ids: HashMap<Good, GoodId>,
}

impl Economy {
    pub fn new(resource_dimensions: usize) -> Self {
        Self {
            resources: Resources::new(resource_dimensions),
            resource_unit_goods: Vec::new(),
            goods: Vec::new(),
            good_ids: HashMap::new(),
        }
    }

    #[inline]
    pub fn resources(&self) -> &Resources {
        &self.resources
    }

    pub fn register_resource(&mut self, properties: PropertyPoint) -> Result<ResourceId> {
        ensure!(
            GoodId::from_index(self.goods.len()).is_some(),
            "cannot register a resource because the good ID space is exhausted"
        );
        let resource = self.resources.register(properties)?;
        debug_assert_eq!(resource.index(), self.resource_unit_goods.len());
        let unit_good = self.register_good(vec![Component {
            resource,
            quantity: 1,
        }])?;
        self.resource_unit_goods.push(unit_good);
        Ok(resource)
    }

    /// The canonical `[(resource, 1)]` configuration used when one extracted
    /// unit enters the economy. Provenance does not affect this `GoodId`.
    pub fn resource_unit_good(&self, resource: ResourceId) -> Option<GoodId> {
        self.resource_unit_goods.get(resource.index()).copied()
    }

    /// Registers a design by exact ordered configuration. This does not produce
    /// an inventory unit; manufacture still requires a recipe and process.
    pub fn register_good(&mut self, configuration: Vec<Component>) -> Result<GoodId> {
        for component in &configuration {
            ensure!(
                self.resources.get(component.resource).is_some(),
                "good contains unknown resource {:?}",
                component.resource
            );
        }
        let good = Good::new(configuration);
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
        Self::new(DEFAULT_RESOURCE_DIMENSIONS)
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

    #[test]
    fn inventory_multiplicity_is_not_product_structure() {
        let mut economy = Economy::new(1);
        let resource = economy
            .register_resource(PropertyPoint::new([0.5].as_slice()).unwrap())
            .unwrap();
        let unit = economy.resource_unit_good(resource).unwrap();
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
