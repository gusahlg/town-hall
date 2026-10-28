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
    /// Number of consecutive copies of this resource in one unit of the good.
    pub quantity: Quantity,
}

/// An exact ordered resource sequence, stored as runs of identical resources.
/// Adjacent identical runs are merged: `[(A, 3), (A, 2)]` and `[(A, 5)]`
/// identify the same good. Separated runs and resource order remain significant.
/// Every configuration is non-empty and every run has a positive quantity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Good {
    configuration: Box<[Component]>,
}

impl Good {
    pub(crate) fn new(mut configuration: Vec<Component>) -> Result<Self> {
        ensure!(
            !configuration.is_empty(),
            "good configuration must be non-empty"
        );
        ensure!(
            configuration.iter().all(|component| component.quantity > 0),
            "good component quantities must be positive"
        );

        // Compact the runs in place without expanding primitive positions.
        // dedup_by passes the later element first and keeps the earlier one.
        configuration.dedup_by(|later, earlier| {
            if later.resource != earlier.resource {
                return false;
            }
            assert!(
                earlier.quantity <= Quantity::MAX - later.quantity,
                "adjacent resource run quantity exceeds u32::MAX"
            );
            earlier.quantity += later.quantity;
            true
        });
        Ok(Self {
            configuration: configuration.into_boxed_slice(),
        })
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

/// Owned counts of separate goods. Fifty units of `[(A, 1)]` remain fifty
/// objects; combining them into one unit of `[(A, 50)]` requires production.
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
