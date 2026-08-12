//! Neural-functional readers. The simulation is rigidly configured to use the
//! Monomial-NFN variant today, while this enum is the narrow seam for future
//! MAGEP/ScaleGMN experiments.

mod monomial;

use anyhow::Result;
use neuro_ash::{DenseNetworkPool, Random};

pub use monomial::MonomialNfnPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderKind {
    Monomial,
}

#[derive(Debug)]
pub enum ReaderPool {
    Monomial(MonomialNfnPool),
}

impl ReaderPool {
    pub fn monomial_with_capacity(
        action_brains: &DenseNetworkPool,
        reader_capacity: usize,
        random: &mut Random,
    ) -> Self {
        Self::Monomial(MonomialNfnPool::with_capacity(
            action_brains,
            reader_capacity,
            random,
        ))
    }

    pub fn kind(&self) -> ReaderKind {
        match self {
            Self::Monomial(_) => ReaderKind::Monomial,
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Monomial(pool) => pool.len(),
        }
    }

    pub fn output_size(&self) -> usize {
        match self {
            Self::Monomial(pool) => pool.output_size(),
        }
    }

    pub fn push_random(&mut self, random: &mut Random) {
        match self {
            Self::Monomial(pool) => pool.push_random(random),
        }
    }

    pub fn swap_remove(&mut self, observer: usize) {
        match self {
            Self::Monomial(pool) => pool.swap_remove(observer),
        }
    }

    pub fn shrink_to_fit(&mut self) {
        match self {
            Self::Monomial(pool) => pool.shrink_to_fit(),
        }
    }

    /// Visit one entity reader's evolvable scalar parameters without exposing
    /// the concrete NFN representation to future selection code.
    pub fn for_each_parameter_mut(&mut self, observer: usize, visitor: impl FnMut(&mut f32)) {
        match self {
            Self::Monomial(pool) => pool.for_each_parameter_mut(observer, visitor),
        }
    }

    pub fn read_into(
        &self,
        observer: usize,
        observed_brains: &DenseNetworkPool,
        observed: usize,
        output: &mut [f32],
    ) -> Result<()> {
        match self {
            Self::Monomial(pool) => pool.read_into(observer, observed_brains, observed, output),
        }
    }
}
