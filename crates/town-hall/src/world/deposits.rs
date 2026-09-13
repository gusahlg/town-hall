use crate::{Quantity, ResourceId};

/// Unowned material on one tile. Extraction properties apply to the whole
/// deposit; amounts and recovery belong to its individual primitive resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDeposit {
    pub resources: Vec<ResourceInfo>,
    pub properties: DepositProperties,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceInfo {
    pub resource_id: ResourceId,
    /// Available primitive units, not inventory goods or a good configuration.
    pub quantity: Quantity,
    /// Nominal primitive units recovered per tick; zero marks finite material.
    /// Recovery is metadata until the extraction/recovery rules are implemented.
    pub recovery_rate: u16,
}

/// Initial extraction parameters. Generation describes them but does not run
/// extraction, reserve workers, resolve equipment, or regenerate resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositProperties {
    /// Nominal ticks per extraction batch; positive in generated deposits.
    pub time: u8,
    /// Number of simultaneous extraction slots; positive in generated deposits.
    pub parallelisation: u8,
    /// Abstract gear requirement, with zero meaning no gear. This is not a GoodId.
    pub gear: u8,
    /// Nominal yield percentage, in `1..=100` for generated deposits.
    pub efficiency: u8,
    /// Nominal primitive units per batch; positive in generated deposits.
    pub batch_size: u8,
}
