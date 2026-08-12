use anyhow::{Result, ensure};

use super::{PropertyPoint, Quantity};

#[derive(Debug, Clone, PartialEq)]
pub struct NeedComponent {
    pub ideal: PropertyPoint,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Need {
    target: Box<[NeedComponent]>,
    intensity: f32,
    recovery_rate: f32,
}

impl Need {
    pub fn new(target: Vec<NeedComponent>, intensity: f32, recovery_rate: f32) -> Result<Self> {
        ensure!(
            intensity.is_finite() && (0.0..=1.0).contains(&intensity),
            "need intensity must be finite and between zero and one"
        );
        ensure!(
            recovery_rate.is_finite() && recovery_rate >= 0.0,
            "need recovery rate must be finite and nonnegative"
        );
        if let Some(first) = target.first() {
            ensure!(
                target
                    .iter()
                    .all(|component| component.ideal.coordinates().len()
                        == first.ideal.coordinates().len()),
                "all need components must occupy the same property space"
            );
        }
        Ok(Self {
            target: target.into_boxed_slice(),
            intensity,
            recovery_rate,
        })
    }

    #[inline]
    pub fn target(&self) -> &[NeedComponent] {
        &self.target
    }

    #[inline]
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    #[inline]
    pub fn recovery_rate(&self) -> f32 {
        self.recovery_rate
    }
}
