use anyhow::{Result, ensure};

use super::{PropertyPoint, Quantity, DIMENSIONS};

#[derive(Debug, Clone, PartialEq)]
pub struct NeedComponent {
    pub ideal: PropertyPoint,
    /// Number of consecutive positions with this ideal point in the target.
    pub quantity: Quantity,
}

// Assigns a u8 importance value to each propoerty/dimension for each specific need. The motivation
// is that my need for chocolate milk might not be satisfied more just because the chocolate milk is
// resistant to flamethrowers, that is an almost irrelevant property in this case.
type ImportanceList = [u8; DIMENSIONS];
type Target =  Box<[NeedComponent]>; 

/// A fixed ideal target with changing intensity. Targets are non-empty, have
/// positive component quantities, and remain unchanged throughout an entity's
/// lifetime. Construction is the only way to supply a target.
#[derive(Debug, Clone, PartialEq)]
pub struct Need {
    target: Target,
    focuses: ImportanceList,
    pub intensity: f32,
    pub recovery_rate: f32,
}

impl Need {
    pub fn new(target: Vec<NeedComponent>, focuses: ImportanceList, intensity: f32, recovery_rate: f32) -> Result<Self> {
        ensure!(!target.is_empty(), "need target must be non-empty");
        ensure!(
            target.iter().all(|component| component.quantity > 0),
            "need target quantities must be positive"
        );
        ensure!(
            intensity.is_finite() && (0.0..=1.0).contains(&intensity),
            "need intensity must be finite and between zero and one"
        );
        ensure!(
            recovery_rate.is_finite() && recovery_rate >= 0.0,
            "need recovery rate must be finite and nonnegative"
        );
        Ok(Self {
            target: target.into_boxed_slice(),
            focuses,
            intensity,
            recovery_rate,
        })
    }

    /// The entity's immutable ideal target, in its original component order.
    #[inline]
    pub fn target(&self) -> &[NeedComponent] {
        &self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DIMENSIONS;

    fn target() -> Vec<NeedComponent> {
        [(0.2, 1), (0.8, Quantity::MAX), (0.2, 2)]
            .into_iter()
            .map(|(coordinate, quantity)| NeedComponent {
                ideal: PropertyPoint::new([coordinate; DIMENSIONS]).unwrap(),
                quantity,
            })
            .collect()
    }

    #[test]
    fn empty_targets_are_rejected() {
        assert!(Need::new(vec![], [0; DIMENSIONS], 0.2, 0.1).is_err());
    }

    #[test]
    fn zero_quantities_are_rejected_at_every_position() {
        let valid = target();
        for index in 0..valid.len() {
            let mut invalid = valid.clone();
            invalid[index].quantity = 0;
            assert!(Need::new(invalid, [0; DIMENSIONS], 0.2, 0.1).is_err());
        }
    }

    #[test]
    fn valid_targets_preserve_ideal_points_order_and_quantities() {
        let target = target();
        let need = Need::new(target.clone(), [0; DIMENSIONS], 0.2, 0.1).unwrap();
        assert_eq!(need.target(), target);
        assert_eq!(need.intensity, 0.2);
        assert_eq!(need.recovery_rate, 0.1);
    }

    #[test]
    fn numeric_state_validation_is_preserved() {
        for intensity in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
            assert!(Need::new(target(), [0; DIMENSIONS], intensity, 0.1).is_err());
        }
        for recovery_rate in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1] {
            assert!(Need::new(target(), [0; DIMENSIONS], 0.2, recovery_rate).is_err());
        }
        assert!(Need::new(target(), [0; DIMENSIONS], 0.0, 0.0).is_ok());
        assert!(Need::new(target(), [0; DIMENSIONS], 1.0, 0.1).is_ok());
    }
}
