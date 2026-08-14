use anyhow::Result;

use super::{GoodAmount, GoodId, canonical_amounts};

pub type Time = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecipeId {
    pub(crate) output: GoodId,
    pub(crate) index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    inputs: Box<[GoodAmount]>,
    processes: Vec<Process>,
}

impl Recipe {
    pub(crate) fn new(inputs: Vec<GoodAmount>) -> Self {
        Self {
            inputs: inputs.into_boxed_slice(),
            processes: Vec::new(),
        }
    }

    #[inline]
    pub fn inputs(&self) -> &[GoodAmount] {
        &self.inputs
    }

    #[inline]
    pub fn processes(&self) -> &[Process] {
        &self.processes
    }

    pub(crate) fn add_process(&mut self, process: Process) {
        if !self.processes.contains(&process) {
            self.processes.push(process);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    workers_required: u16,
    duration: Time,
    equipment: Box<[GoodAmount]>,
    consumables: Box<[GoodAmount]>,
    inefficiency_inputs: Box<[GoodAmount]>,
}

impl Process {
    pub fn new(
        workers_required: u16,
        duration: Time,
        equipment: Vec<GoodAmount>,
        consumables: Vec<GoodAmount>,
        inefficiency_inputs: Vec<GoodAmount>,
    ) -> Result<Self> {
        Ok(Self {
            workers_required,
            duration,
            equipment: canonical_amounts(equipment)?.into_boxed_slice(),
            consumables: canonical_amounts(consumables)?.into_boxed_slice(),
            inefficiency_inputs: canonical_amounts(inefficiency_inputs)?.into_boxed_slice(),
        })
    }

    #[inline]
    pub fn workers_required(&self) -> u16 {
        self.workers_required
    }

    #[inline]
    pub fn duration(&self) -> Time {
        self.duration
    }

    #[inline]
    pub fn equipment(&self) -> &[GoodAmount] {
        &self.equipment
    }

    #[inline]
    pub fn consumables(&self) -> &[GoodAmount] {
        &self.consumables
    }

    #[inline]
    pub fn inefficiency_inputs(&self) -> &[GoodAmount] {
        &self.inefficiency_inputs
    }
}
