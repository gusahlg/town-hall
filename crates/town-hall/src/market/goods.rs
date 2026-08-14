use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use super::ResourceId;

/// Universal, indivisible material and inventory unit.
pub type Quantity = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GoodId(u32);

impl GoodId {
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_index(index: usize) -> Option<Self> {
        u32::try_from(index).ok().map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Component {
    pub resource: ResourceId,
    /// Primitive units embodied at this position in one unit of the good.
    pub quantity: Quantity,
}

/// One exact ordered physical configuration. Component boundaries, order, and
/// quantities are identity: configurations are never flattened or combined for
/// `GoodId` deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Good {
    configuration: Box<[Component]>,
}

impl Good {
    pub(crate) fn new(configuration: impl Into<Box<[Component]>>) -> Self {
        Self {
            configuration: configuration.into(),
        }
    }

    #[inline]
    pub fn configuration(&self) -> &[Component] {
        &self.configuration
    }
}

pub(crate) fn canonical_amounts(mut amounts: Vec<GoodAmount>) -> Result<Vec<GoodAmount>> {
    ensure!(
        amounts.iter().all(|amount| amount.quantity > 0),
        "requirements must have positive quantities"
    );
    amounts.sort_unstable_by_key(|amount| amount.good);
    let mut canonical: Vec<GoodAmount> = Vec::with_capacity(amounts.len());
    for amount in amounts {
        if let Some(previous) = canonical.last_mut().filter(|item| item.good == amount.good) {
            previous.quantity = previous
                .quantity
                .checked_add(amount.quantity)
                .ok_or_else(|| anyhow::anyhow!("requirement quantity overflow"))?;
        } else {
            canonical.push(amount);
        }
    }
    Ok(canonical)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GoodAmount {
    pub good: GoodId,
    /// Number of separate units of this exact good.
    pub quantity: Quantity,
}

/// Owned counts of exact goods. Inventory multiplicity never changes a good's
/// internal configuration: fifty units of `[(A, 1)]` remain fifty objects, not
/// one unit of `[(A, 50)]`.
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    quantities: BTreeMap<GoodId, Quantity>,
}

impl Inventory {
    #[inline]
    pub fn quantity(&self, good: GoodId) -> Quantity {
        self.quantities.get(&good).copied().unwrap_or(0)
    }

    pub fn set(&mut self, good: GoodId, quantity: Quantity) {
        if quantity == 0 {
            self.quantities.remove(&good);
        } else {
            self.quantities.insert(good, quantity);
        }
    }

    #[inline]
    pub fn entries(&self) -> impl Iterator<Item = GoodAmount> + '_ {
        self.quantities
            .iter()
            .map(|(&good, &quantity)| GoodAmount { good, quantity })
    }

    pub fn total_units(&self) -> u128 {
        self.quantities
            .values()
            .map(|&quantity| quantity as u128)
            .sum()
    }
}
