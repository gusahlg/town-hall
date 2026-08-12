use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use super::ResourceId;

/// Temporary universal material unit. Its exact numeric representation remains
/// deliberately replaceable while quantities are still being designed.
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
    pub quantity: Quantity,
}

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
    pub quantity: Quantity,
}

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
