use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;

use anyhow::{Result, ensure};
use neuro_ash::{BatchEvaluator, DenseNetworkPool, NetworkShape, Random};
use rayon::prelude::*;

use crate::{
    ACTION_INPUT_COUNT, ACTION_LAYER_SIZES, ACTION_OUTPUT_COUNT, Action, BASE_INPUT_COUNT,
    CONNECTION_BUNDLE_SIZE, CONSERVE_OUTPUT, CONSUMPTION_PER_ITERATION, ConnectionBundle,
    DEFAULT_RANDOM_SEED, Entity, EntityId, EntityKind, GET_TO_KNOW_LAYER_SIZES, GIVE_OUTPUT_START,
    KNOWN_PERSON_COUNT, KNOWN_PERSON_MASK_WORDS, KnownPeoplePool, MONOMIAL_FEATURE_COUNT,
    PREDICT_OUTPUT_START, ReaderKind, ReaderPool, STARTING_POPULATION, give_output, predict_output,
    Market, World,
};

const INITIAL_ENTITY_RESOURCES: u64 = 100;
const INITIAL_WORLD_RESOURCES: u128 = 1_000;
const INITIAL_GROWTH_RATE: u64 = 100;
const PARALLEL_READER_THRESHOLD: usize = 128;

#[derive(Debug, Clone)]
pub struct TickReport {
    pub iteration: u128,
    pub replaced_slots: usize,
    pub consumed_resources: u128,
    pub deaths: Vec<EntityId>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Copy)]
struct SlotReplacement {
    observer: usize,
    slot: usize,
    incoming: EntityId,
}

#[derive(Debug)]
struct SlotUpdatePlan {
    replacements: Vec<SlotReplacement>,
    observer_offsets: Vec<usize>,
    active_masks: Vec<[u64; KNOWN_PERSON_MASK_WORDS]>,
    rounds: usize,
}

impl SlotUpdatePlan {
    fn event_count(&self, observer: usize) -> usize {
        self.observer_offsets[observer + 1] - self.observer_offsets[observer]
    }

    fn event_in_round(&self, observer: usize, round: usize) -> Option<(usize, SlotReplacement)> {
        let start = self.observer_offsets[observer];
        let end = self.observer_offsets[observer + 1];
        (start + round < end).then(|| {
            let index = start + round;
            (index, self.replacements[index])
        })
    }
}

pub struct Village {
    population: Vec<Entity>,
    entity_indices: HashMap<EntityId, usize>,
    known_people: KnownPeoplePool,
    action_brains: DenseNetworkPool,
    get_to_know_brains: DenseNetworkPool,
    readers: ReaderPool,
    random: Random,
    next_entity_id: u128,
    iterations: u128,
    // Immutable representation of market metadata
    market: Market,
    // Resources (described in market) distribution over a map
    world: World,
    growth_rate: u64,
}

impl Default for Village {
    fn default() -> Self {
        Self::new()
    }
}

impl Village {
    pub fn new() -> Self {
        Self::with_population(STARTING_POPULATION, DEFAULT_RANDOM_SEED)
    }

    pub fn with_population(population: usize, seed: u64) -> Self {
        let action_shape = NetworkShape::new(ACTION_LAYER_SIZES.to_vec())
            .expect("compile-time action shape is valid");
        let get_to_know_shape = NetworkShape::new(GET_TO_KNOW_LAYER_SIZES.to_vec())
            .expect("compile-time get-to-know shape is valid");
        let mut random = Random::new(seed);
        let action_brains = DenseNetworkPool::with_capacity(action_shape, population);
        let readers = ReaderPool::monomial_with_capacity(&action_brains, population, &mut random);
        let mut village = Self {
            population: Vec::with_capacity(population),
            entity_indices: HashMap::with_capacity(population),
            known_people: KnownPeoplePool::with_capacity(population),
            action_brains,
            get_to_know_brains: DenseNetworkPool::with_capacity(get_to_know_shape, population),
            readers,
            random,
            next_entity_id: 0,
            iterations: 0,
            market: Market::default(),
            world: world::generate(),
            growth_rate: INITIAL_GROWTH_RATE,
        };
        for _ in 0..population {
            village.spawn(EntityKind::Person, INITIAL_ENTITY_RESOURCES, None);
        }
        village
    }

    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.population
    }

    #[inline]
    pub fn known_people(&self) -> &KnownPeoplePool {
        &self.known_people
    }

    #[inline]
    pub fn action_brains(&self) -> &DenseNetworkPool {
        &self.action_brains
    }

    #[inline]
    pub fn get_to_know_brains(&self) -> &DenseNetworkPool {
        &self.get_to_know_brains
    }

    #[inline]
    pub fn reader_kind(&self) -> ReaderKind {
        self.readers.kind()
    }

    #[inline]
    pub fn iterations(&self) -> u128 {
        self.iterations
    }
    }

    #[inline]
    pub fn growth_rate(&self) -> u64 {
        self.growth_rate
    }

    pub fn total_entity_resources(&self) -> u128 {
        self.population
            .iter()
            .map(|entity| entity.resources() as u128)
            .sum()
    }

    pub fn spawn_person(&mut self, resources: u64) -> EntityId {
        self.spawn(EntityKind::Person, resources, None)
    }

    pub fn remove_entity(&mut self, id: EntityId) -> Result<Entity> {
        self.assert_aligned()?;
        let index = self.entity_index(id)?;
        let removed = self.swap_remove_entity(index)?;
        self.shrink_storage_if_sparse();
        self.assert_aligned()?;
        Ok(removed)
    }

    pub fn set_entity_resources(&mut self, id: EntityId, resources: u64) -> Result<()> {
        let index = self.entity_index(id)?;
        self.population[index].set_resources(resources);
        Ok(())
    }

    pub fn action_weights_mut(&mut self, id: EntityId, layer: usize) -> Result<&mut [f32]> {
        let entity = self.entity_index(id)?;
        self.action_brains.network_weights_mut(layer, entity)
    }

    pub fn action_biases_mut(&mut self, id: EntityId, layer: usize) -> Result<&mut [f32]> {
        let entity = self.entity_index(id)?;
        self.action_brains.network_biases_mut(layer, entity)
    }

    pub fn get_to_know_weights_mut(&mut self, id: EntityId, layer: usize) -> Result<&mut [f32]> {
        let entity = self.entity_index(id)?;
        self.get_to_know_brains.network_weights_mut(layer, entity)
    }

    pub fn get_to_know_biases_mut(&mut self, id: EntityId, layer: usize) -> Result<&mut [f32]> {
        let entity = self.entity_index(id)?;
        self.get_to_know_brains.network_biases_mut(layer, entity)
    }

    pub fn for_each_reader_parameter_mut(
        &mut self,
        id: EntityId,
        visitor: impl FnMut(&mut f32),
    ) -> Result<()> {
        let entity = self.entity_index(id)?;
        self.readers.for_each_parameter_mut(entity, visitor);
        Ok(())
    }

    /// One synchronous simulation iteration:
    /// snapshot/rank -> learn replacements -> atomically patch -> act ->
    /// apply -> consume -> remove depleted entities.
    pub fn step(&mut self, evaluator: &mut impl BatchEvaluator) -> Result<TickReport> {
        self.assert_aligned()?;
        if self.population.is_empty() {
            evaluator.release(&self.action_brains);
            evaluator.release(&self.get_to_know_brains);
            self.finish_world_update();
            return Ok(TickReport {
                iteration: self.iterations,
                replaced_slots: 0,
                consumed_resources: 0,
                deaths: Vec::new(),
                actions: Vec::new(),
            });
        }

        let plan = self.plan_slot_updates();
        if plan.rounds > 0 {
            // The action pool is read on the CPU while impressions are formed.
            // Its device copy cannot be reused after the generated patches, so
            // release it before allocating get-to-know tensors.
            evaluator.release(&self.action_brains);
        }
        let mut pending = vec![None; plan.replacements.len()];
        let population = self.population.len();

        // The action pool remains untouched throughout every round, so all
        // impressions observe the same pre-update brain snapshot. Observer
        // chunks are outermost so a streamed GPU brain chunk remains resident
        // across all of its replacement rounds.
        let evaluator_batch_size = evaluator.max_batch_size().max(1);
        for observer_start in (0..population).step_by(evaluator_batch_size) {
            let observer_end = observer_start
                .saturating_add(evaluator_batch_size)
                .min(population);
            let observer_range = observer_start..observer_end;
            let chunk_rounds = observer_range
                .clone()
                .map(|observer| plan.event_count(observer))
                .max()
                .unwrap_or(0);
            if chunk_rounds == 0 {
                continue;
            }
            let mut reader_outputs = vec![0.0; observer_range.len() * MONOMIAL_FEATURE_COUNT];

            for round in 0..chunk_rounds {
                reader_outputs.fill(0.0);
                self.read_impressions(&plan, observer_range.clone(), round, &mut reader_outputs)?;

                let generated = evaluator.evaluate_range(
                    &self.get_to_know_brains,
                    observer_range.clone(),
                    &reader_outputs,
                )?;
                ensure!(
                    generated.len() == observer_range.len() * CONNECTION_BUNDLE_SIZE,
                    "get-to-know evaluator returned the wrong output size"
                );
                for observer in observer_range.clone() {
                    let Some((event_index, _)) = plan.event_in_round(observer, round) else {
                        continue;
                    };
                    let local_observer = observer - observer_start;
                    let row = &generated[local_observer * CONNECTION_BUNDLE_SIZE
                        ..(local_observer + 1) * CONNECTION_BUNDLE_SIZE];
                    pending[event_index] = Some(ConnectionBundle::from_output(row)?);
                }
            }
        }
        // Get-to-know and action inference are sequential phases. Keeping both
        // parameter families resident would consume device memory for a pool
        // that cannot run again during this tick.
        evaluator.release(&self.get_to_know_brains);

        // Atomic at the simulation level: metadata remains unchanged until all
        // generated bundles exist, and no action runs until every changed slot
        // has received all three heads.
        ensure!(
            pending.iter().all(Option::is_some),
            "a planned slot replacement was not learned"
        );
        let replaced_slots = plan.replacements.len();
        self.apply_slot_metadata(&plan);
        for (replacement, bundle) in plan.replacements.iter().zip(pending) {
            bundle.expect("validated above").install(
                &mut self.action_brains,
                replacement.observer,
                replacement.slot,
            )?;
        }
        drop(plan);

        let action_inputs = self.build_action_inputs();
        let logits = evaluator.evaluate(&self.action_brains, &action_inputs)?;
        ensure!(
            logits.len() == population * ACTION_OUTPUT_COUNT,
            "action evaluator returned the wrong output size"
        );
        let actions = (0..population)
            .map(|entity| {
                let row = &logits[entity * ACTION_OUTPUT_COUNT..(entity + 1) * ACTION_OUTPUT_COUNT];
                self.decode_action(entity, row)
            })
            .collect::<Vec<_>>();
        drop(logits);
        drop(action_inputs);

        self.apply_placeholder_actions(&actions);
        let consumed_resources = self.apply_consumption();
        for (entity, &action) in self.population.iter_mut().zip(&actions) {
            entity.set_last_action(action);
        }
        let deaths = self.remove_depleted_entities()?;
        if self.population.is_empty() {
            evaluator.release(&self.action_brains);
        }
        self.finish_world_update();

        Ok(TickReport {
            iteration: self.iterations,
            replaced_slots,
            consumed_resources,
            deaths,
            actions,
        })
    }

    fn spawn(&mut self, kind: EntityKind, resources: u64, owners: Option<EntityId>) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id = self
            .next_entity_id
            .checked_add(1)
            .expect("entity ID space exhausted");
        let index = self.population.len();
        self.population
            .push(Entity::new(kind, resources, owners, id));
        self.entity_indices.insert(id, index);
        self.known_people.push_empty();
        self.action_brains.push_random(&mut self.random);
        self.get_to_know_brains.push_random(&mut self.random);
        self.readers.push_random(&mut self.random);
        id
    }

    fn swap_remove_entity(&mut self, index: usize) -> Result<Entity> {
        ensure!(
            index < self.population.len(),
            "entity index is out of bounds"
        );
        let id = self.population[index].id();
        ensure!(
            self.entity_indices.remove(&id) == Some(index),
            "entity index map is inconsistent for {id:?}"
        );
        let removed = self.population.swap_remove(index);
        self.known_people.swap_remove(index);
        self.action_brains.swap_remove(index)?;
        self.get_to_know_brains.swap_remove(index)?;
        self.readers.swap_remove(index);
        if index < self.population.len() {
            self.entity_indices
                .insert(self.population[index].id(), index);
        }
        Ok(removed)
    }

    fn shrink_storage_if_sparse(&mut self) {
        if self.population.len() > self.population.capacity() / 4 {
            return;
        }
        self.population.shrink_to_fit();
        self.entity_indices.shrink_to_fit();
        self.known_people.shrink_to_fit();
        self.action_brains.shrink_to_fit();
        self.get_to_know_brains.shrink_to_fit();
        self.readers.shrink_to_fit();
    }

    fn entity_index(&self, id: EntityId) -> Result<usize> {
        self.entity_indices
            .get(&id)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("unknown entity {id:?}"))
    }

    fn read_impressions(
        &self,
        plan: &SlotUpdatePlan,
        observers: Range<usize>,
        round: usize,
        output: &mut [f32],
    ) -> Result<()> {
        let read_row = |observer: usize, row: &mut [f32]| -> Result<()> {
            let Some((_, replacement)) = plan.event_in_round(observer, round) else {
                return Ok(());
            };
            let observed = self.entity_indices[&replacement.incoming];
            self.readers
                .read_into(observer, &self.action_brains, observed, row)
        };

        ensure!(
            output.len() == observers.len() * MONOMIAL_FEATURE_COUNT,
            "reader output chunk has the wrong size"
        );
        if observers.len() >= PARALLEL_READER_THRESHOLD {
            output
                .par_chunks_mut(MONOMIAL_FEATURE_COUNT)
                .enumerate()
                .try_for_each(|(local, row)| read_row(observers.start + local, row))?;
        } else {
            for (local, row) in output.chunks_mut(MONOMIAL_FEATURE_COUNT).enumerate() {
                read_row(observers.start + local, row)?;
            }
        }
        Ok(())
    }

    fn plan_slot_updates(&self) -> SlotUpdatePlan {
        let ranked = self.richest_indices(KNOWN_PERSON_COUNT + 1);
        let ranked_ids = ranked
            .iter()
            .map(|&index| self.population[index].id())
            .collect::<Vec<_>>();
        let rank_positions = ranked_ids
            .iter()
            .enumerate()
            .map(|(rank, &id)| (id, rank))
            .collect::<HashMap<_, _>>();

        let mut replacements = Vec::new();
        let mut observer_offsets = Vec::with_capacity(self.population.len() + 1);
        let mut active_masks = Vec::with_capacity(self.population.len());
        observer_offsets.push(0);
        let mut rounds = 0;

        for observer in 0..self.population.len() {
            let observer_id = self.population[observer].id();
            let observer_rank = rank_positions.get(&observer_id).copied();
            let desired_len =
                (ranked_ids.len() - usize::from(observer_rank.is_some())).min(KNOWN_PERSON_COUNT);
            let desired_candidate = |position: usize| {
                let rank = position
                    + usize::from(observer_rank.is_some_and(|own_rank| own_rank <= position));
                ranked_ids[rank]
            };
            let mut present = [false; KNOWN_PERSON_COUNT];
            let mut active_mask = [0_u64; KNOWN_PERSON_MASK_WORDS];

            for slot in 0..KNOWN_PERSON_COUNT {
                let Some(occupant) = self.known_people.occupant(observer, slot) else {
                    continue;
                };
                let Some(&rank) = rank_positions.get(&occupant) else {
                    continue;
                };
                if occupant == observer_id {
                    continue;
                }
                let position =
                    rank - usize::from(observer_rank.is_some_and(|own_rank| own_rank < rank));
                if position < desired_len && !present[position] {
                    present[position] = true;
                    set_mask_bit(&mut active_mask, slot);
                }
            }

            let start = replacements.len();
            let mut free_slot = 0;
            for (position, is_present) in present.iter().copied().take(desired_len).enumerate() {
                if is_present {
                    continue;
                }
                while free_slot < KNOWN_PERSON_COUNT && mask_bit(&active_mask, free_slot) {
                    free_slot += 1;
                }
                assert!(
                    free_slot < KNOWN_PERSON_COUNT,
                    "slot reconciliation produced more than {KNOWN_PERSON_COUNT} occupants"
                );
                set_mask_bit(&mut active_mask, free_slot);
                replacements.push(SlotReplacement {
                    observer,
                    slot: free_slot,
                    incoming: desired_candidate(position),
                });
                free_slot += 1;
            }
            rounds = rounds.max(replacements.len() - start);
            observer_offsets.push(replacements.len());
            active_masks.push(active_mask);
        }

        SlotUpdatePlan {
            replacements,
            observer_offsets,
            active_masks,
            rounds,
        }
    }

    fn apply_slot_metadata(&mut self, plan: &SlotUpdatePlan) {
        debug_assert_eq!(plan.active_masks.len(), self.population.len());
        for observer in 0..self.population.len() {
            self.known_people
                .set_active_mask(observer, plan.active_masks[observer]);
        }
        for replacement in &plan.replacements {
            self.known_people
                .replace(replacement.observer, replacement.slot, replacement.incoming);
        }
    }

    fn richest_indices(&self, limit: usize) -> Vec<usize> {
        let mut indices = (0..self.population.len()).collect::<Vec<_>>();
        let keep = limit.min(indices.len());
        let compare = |&left: &usize, &right: &usize| {
            self.population[right]
                .resources()
                .cmp(&self.population[left].resources())
                .then_with(|| self.population[left].id().cmp(&self.population[right].id()))
        };
        if keep < indices.len() {
            indices.select_nth_unstable_by(keep, compare);
            indices.truncate(keep);
        }
        indices.sort_unstable_by(compare);
        indices
    }

    fn build_action_inputs(&self) -> Vec<f32> {
        let mut inputs = vec![0.0; self.population.len() * ACTION_INPUT_COUNT];
        for (entity_index, entity) in self.population.iter().enumerate() {
            let row = &mut inputs
                [entity_index * ACTION_INPUT_COUNT..(entity_index + 1) * ACTION_INPUT_COUNT];
            row[0] = log_normalize(entity.resources() as u128);
            row[1] = log_normalize(self.resources);
            row[2] = log_normalize(self.growth_rate as u128);
            for slot in 0..KNOWN_PERSON_COUNT {
                if !self.known_people.is_active(entity_index, slot) {
                    continue;
                }
                let Some(known_id) = self.known_people.occupant(entity_index, slot) else {
                    continue;
                };
                if let Some(&known_index) = self.entity_indices.get(&known_id) {
                    row[BASE_INPUT_COUNT + slot] =
                        log_normalize(self.population[known_index].resources() as u128);
                }
            }
        }
        inputs
    }

    fn decode_action(&self, entity: usize, logits: &[f32]) -> Action {
        debug_assert_eq!(logits.len(), ACTION_OUTPUT_COUNT);
        let mut best_output = CONSERVE_OUTPUT;
        let mut best = finite_logit(logits[CONSERVE_OUTPUT]);
        let mut second = f32::NEG_INFINITY;

        let mut consider = |output: usize| {
            let value = finite_logit(logits[output]);
            match value.total_cmp(&best) {
                Ordering::Greater => {
                    second = best;
                    best = value;
                    best_output = output;
                }
                _ if value.total_cmp(&second) == Ordering::Greater => second = value,
                _ => {}
            }
        };
        for slot in 0..KNOWN_PERSON_COUNT {
            if self.known_people.is_active(entity, slot) {
                consider(predict_output(slot));
                consider(give_output(slot));
            }
        }

        if best_output == CONSERVE_OUTPUT {
            return Action::Conserve;
        }
        let margin = if second.is_finite() {
            (best - second).max(0.0)
        } else {
            f32::INFINITY
        };
        let confidence = if margin.is_infinite() {
            1.0
        } else {
            -(-margin).exp_m1()
        };
        let available = self.population[entity].resources();
        let amount = ((available as f64) * confidence as f64).floor() as u64;

        if best_output < GIVE_OUTPUT_START {
            let slot = best_output - PREDICT_OUTPUT_START;
            Action::Predict {
                amount,
                target: self.known_people.occupant(entity, slot).unwrap(),
            }
        } else {
            let slot = best_output - GIVE_OUTPUT_START;
            Action::Give {
                amount,
                target: self.known_people.occupant(entity, slot).unwrap(),
            }
        }
    }

    /// Deliberately thin placeholder. Give transfers are staged; Predict has
    /// no world effect yet. The neural architecture does not depend on these
    /// provisional economics.
    fn apply_placeholder_actions(&mut self, actions: &[Action]) {
        assert_eq!(
            actions.len(),
            self.population.len(),
            "every live entity must contribute exactly one action"
        );
        let before = self
            .population
            .iter()
            .map(Entity::resources)
            .collect::<Vec<_>>();
        let mut outgoing = vec![0_u64; self.population.len()];
        let mut incoming = vec![0_u128; self.population.len()];
        for (actor, action) in actions.iter().enumerate() {
            let Action::Give { amount, target } = *action else {
                continue;
            };
            let Some(&target_index) = self.entity_indices.get(&target) else {
                continue;
            };
            let amount = amount.min(before[actor]);
            outgoing[actor] = amount;
            incoming[target_index] += amount as u128;
        }
        for entity in 0..self.population.len() {
            let remaining = before[entity] - outgoing[entity];
            let updated = (remaining as u128 + incoming[entity]).min(u64::MAX as u128) as u64;
            self.population[entity].set_resources(updated);
        }
    }

    fn apply_consumption(&mut self) -> u128 {
        self.population
            .iter_mut()
            .map(|entity| {
                let consumed = entity.resources().min(CONSUMPTION_PER_ITERATION);
                entity.set_resources(entity.resources() - consumed);
                consumed as u128
            })
            .sum()
    }

    fn remove_depleted_entities(&mut self) -> Result<Vec<EntityId>> {
        let deaths = self
            .population
            .iter()
            .filter(|entity| entity.resources() == 0)
            .map(Entity::id)
            .collect::<Vec<_>>();
        for &id in &deaths {
            let index = self.entity_indices[&id];
            self.swap_remove_entity(index)?;
        }
        if !deaths.is_empty() {
            self.shrink_storage_if_sparse();
            self.assert_aligned()?;
        }
        Ok(deaths)
    }

    fn finish_world_update(&mut self) {
        self.resources = self.resources.saturating_add(self.growth_rate as u128);
        self.iterations = self.iterations.saturating_add(1);
    }

    // Makes sure that the networks and the population are aligned properly
    fn assert_aligned(&self) -> Result<()> {
        let population = self.population.len();
        ensure!(
            self.entity_indices.len() == population,
            "entity index map is misaligned"
        );
        for (index, entity) in self.population.iter().enumerate() {
            ensure!(
                self.entity_indices.get(&entity.id()) == Some(&index),
                "entity index map has the wrong row for {:?}",
                entity.id()
            );
        }
        ensure!(
            self.known_people.len() == population,
            "known-person pool is misaligned"
        );
        ensure!(
            self.action_brains.len() == population,
            "action-brain pool is misaligned"
        );
        ensure!(
            self.action_brains.shape().widths() == ACTION_LAYER_SIZES,
            "action-brain shape changed unexpectedly"
        );
        ensure!(
            self.get_to_know_brains.len() == population,
            "get-to-know pool is misaligned"
        );
        ensure!(
            self.get_to_know_brains.shape().widths() == GET_TO_KNOW_LAYER_SIZES,
            "get-to-know brain shape changed unexpectedly"
        );
        ensure!(
            self.readers.len() == population,
            "reader pool is misaligned"
        );
        ensure!(
            self.readers.output_size() == self.get_to_know_brains.shape().input_size(),
            "reader output and get-to-know input disagree"
        );
        Ok(())
    }
}

#[inline]
fn finite_logit(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        f32::NEG_INFINITY
    }
}

#[inline]
fn mask_bit(mask: &[u64; KNOWN_PERSON_MASK_WORDS], slot: usize) -> bool {
    mask[slot / 64] & (1_u64 << (slot % 64)) != 0
}

#[inline]
fn set_mask_bit(mask: &mut [u64; KNOWN_PERSON_MASK_WORDS], slot: usize) {
    mask[slot / 64] |= 1_u64 << (slot % 64);
}

/// Fixed log normalization keeps a resource value comparable between ticks and
/// maps the full u64 resource range to `[0, 1]`.
fn log_normalize(value: u128) -> f32 {
    let capped = value.min(u64::MAX as u128) as f64;
    (capped.ln_1p() / (u64::MAX as f64).ln_1p()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuro_ash::CpuEvaluator;

    struct FailingEvaluator;

    impl BatchEvaluator for FailingEvaluator {
        fn evaluate(&mut self, _pool: &DenseNetworkPool, _inputs: &[f32]) -> Result<Vec<f32>> {
            anyhow::bail!("synthetic evaluator failure")
        }
    }

    struct ChunkedCpuEvaluator {
        inner: CpuEvaluator,
        max_batch_size: usize,
    }

    impl BatchEvaluator for ChunkedCpuEvaluator {
        fn evaluate(&mut self, pool: &DenseNetworkPool, inputs: &[f32]) -> Result<Vec<f32>> {
            self.inner.evaluate(pool, inputs)
        }

        fn evaluate_range(
            &mut self,
            pool: &DenseNetworkPool,
            networks: Range<usize>,
            inputs: &[f32],
        ) -> Result<Vec<f32>> {
            self.inner.evaluate_range(pool, networks, inputs)
        }

        fn max_batch_size(&self) -> usize {
            self.max_batch_size
        }
    }

    #[test]
    fn initial_plan_fills_every_available_non_self_slot_once() {
        let mut village = Village::with_population(4, 1);
        let plan = village.plan_slot_updates();
        assert_eq!(plan.replacements.len(), 12);
        assert_eq!(plan.rounds, 3);
        village.apply_slot_metadata(&plan);
        for observer in 0..4 {
            assert_eq!(
                (0..KNOWN_PERSON_COUNT)
                    .filter(|&slot| village.known_people.is_active(observer, slot))
                    .count(),
                3
            );
            assert!((0..KNOWN_PERSON_COUNT).all(|slot| {
                village.known_people.occupant(observer, slot)
                    != Some(village.population[observer].id())
            }));
        }
        assert!(village.plan_slot_updates().replacements.is_empty());
    }

    #[test]
    fn chunked_learning_matches_full_population_learning() {
        let mut full = Village::with_population(5, 101);
        let mut chunked = Village::with_population(5, 101);
        let full_report = full.step(&mut CpuEvaluator).unwrap();
        let chunked_report = chunked
            .step(&mut ChunkedCpuEvaluator {
                inner: CpuEvaluator,
                max_batch_size: 2,
            })
            .unwrap();

        assert_eq!(chunked_report.actions, full_report.actions);
        assert_eq!(chunked_report.replaced_slots, full_report.replaced_slots);
        for (full_layer, chunked_layer) in full
            .action_brains
            .layers()
            .iter()
            .zip(chunked.action_brains.layers())
        {
            assert_eq!(chunked_layer.weights(), full_layer.weights());
            assert_eq!(chunked_layer.biases(), full_layer.biases());
        }
    }

    #[test]
    fn dead_occupants_become_inactive_and_can_be_replaced_later() {
        let mut village = Village::with_population(4, 2);
        let initial = village.plan_slot_updates();
        village.apply_slot_metadata(&initial);
        let dead = village.population[3].id();
        village.remove_entity(dead).unwrap();
        let after_death = village.plan_slot_updates();
        assert!(after_death.replacements.is_empty());
        village.apply_slot_metadata(&after_death);
        for observer in 0..3 {
            assert_eq!(
                (0..KNOWN_PERSON_COUNT)
                    .filter(|&slot| village.known_people.is_active(observer, slot))
                    .count(),
                2
            );
        }

        village.spawn_person(100);
        let after_birth = village.plan_slot_updates();
        assert_eq!(after_birth.replacements.len(), 6);
    }

    #[test]
    fn a_complete_cpu_tick_runs_the_decided_neural_flow() {
        let mut village = Village::with_population(4, 3);
        let mut evaluator = CpuEvaluator;
        let report = village.step(&mut evaluator).unwrap();

        assert_eq!(report.iteration, 1);
        assert_eq!(report.replaced_slots, 12);
        assert!(report.consumed_resources <= 4 * CONSUMPTION_PER_ITERATION as u128);
        assert_eq!(report.actions.len(), 4);
        assert!(village.action_brains.all_finite());

        let second = village.step(&mut evaluator).unwrap();
        assert_eq!(second.iteration, 2);
        assert_eq!(second.replaced_slots, 0);
    }

    #[test]
    fn failed_learning_does_not_commit_slot_metadata() {
        let mut village = Village::with_population(4, 31);
        let error = village.step(&mut FailingEvaluator).unwrap_err();
        assert!(error.to_string().contains("synthetic"));
        assert_eq!(village.iterations(), 0);
        for observer in 0..village.population.len() {
            for slot in 0..KNOWN_PERSON_COUNT {
                assert!(!village.known_people.is_active(observer, slot));
                assert_eq!(village.known_people.occupant(observer, slot), None);
            }
        }
    }

    #[test]
    fn top_two_hundred_churn_replaces_slots_without_reordering_survivors() {
        let mut village = Village::with_population(KNOWN_PERSON_COUNT + 2, 13);
        let initial = village.plan_slot_updates();
        assert_eq!(
            initial.replacements.len(),
            (KNOWN_PERSON_COUNT + 2) * KNOWN_PERSON_COUNT
        );
        village.apply_slot_metadata(&initial);

        let observer = village.entity_indices[&EntityId(0)];
        let survivor = EntityId(1);
        let survivor_slot = (0..KNOWN_PERSON_COUNT)
            .find(|&slot| village.known_people.occupant(observer, slot) == Some(survivor))
            .unwrap();
        village
            .set_entity_resources(EntityId((KNOWN_PERSON_COUNT + 1) as u128), u64::MAX)
            .unwrap();
        let churn = village.plan_slot_updates();

        assert_eq!(churn.replacements.len(), KNOWN_PERSON_COUNT + 1);
        assert_eq!(
            village.known_people.occupant(observer, survivor_slot),
            Some(survivor)
        );
    }

    #[test]
    fn action_decoder_masks_inactive_slots_and_uses_logit_margin_for_amount() {
        let mut village = Village::with_population(2, 14);
        let plan = village.plan_slot_updates();
        village.apply_slot_metadata(&plan);
        let active_slot = (0..KNOWN_PERSON_COUNT)
            .find(|&slot| village.known_people.is_active(0, slot))
            .unwrap();
        let inactive_slot = (0..KNOWN_PERSON_COUNT)
            .find(|&slot| !village.known_people.is_active(0, slot))
            .unwrap();
        let mut logits = vec![-10.0; ACTION_OUTPUT_COUNT];
        logits[CONSERVE_OUTPUT] = 0.0;
        logits[predict_output(active_slot)] = 2.0;
        logits[give_output(active_slot)] = 1.0;
        logits[give_output(inactive_slot)] = 1_000.0;

        let action = village.decode_action(0, &logits);
        let Action::Predict { amount, target } = action else {
            panic!("expected Predict, got {action:?}");
        };
        assert_eq!(
            target,
            village.known_people.occupant(0, active_slot).unwrap()
        );
        assert_eq!(amount, 63);
    }

    #[test]
    fn action_decoder_treats_non_finite_logits_as_unavailable() {
        let mut village = Village::with_population(2, 15);
        let plan = village.plan_slot_updates();
        village.apply_slot_metadata(&plan);
        let active_slot = (0..KNOWN_PERSON_COUNT)
            .find(|&slot| village.known_people.is_active(0, slot))
            .unwrap();
        let mut logits = vec![f32::NEG_INFINITY; ACTION_OUTPUT_COUNT];
        logits[CONSERVE_OUTPUT] = 0.0;
        logits[predict_output(active_slot)] = f32::NAN;
        logits[give_output(active_slot)] = f32::INFINITY;

        assert_eq!(village.decode_action(0, &logits), Action::Conserve);
    }

    #[test]
    fn reciprocal_give_actions_are_applied_from_one_snapshot() {
        let mut village = Village::with_population(2, 16);
        let first = village.population[0].id();
        let second = village.population[1].id();
        village.population[0].set_resources(100);
        village.population[1].set_resources(40);

        village.apply_placeholder_actions(&[
            Action::Give {
                amount: 100,
                target: second,
            },
            Action::Give {
                amount: 40,
                target: first,
            },
        ]);

        assert_eq!(village.population[0].resources(), 40);
        assert_eq!(village.population[1].resources(), 100);
        assert_eq!(village.total_entity_resources(), 140);
    }

    #[test]
    fn removing_a_middle_entity_keeps_every_private_pool_aligned() {
        let mut village = Village::with_population(3, 17);
        let plan = village.plan_slot_updates();
        village.apply_slot_metadata(&plan);
        let removed = village.population[1].id();
        let moved = village.population[2].id();
        let expected_known = (0..KNOWN_PERSON_COUNT)
            .map(|slot| {
                (
                    village.known_people.occupant(2, slot),
                    village.known_people.is_active(2, slot),
                )
            })
            .collect::<Vec<_>>();
        let expected_action = village
            .action_brains
            .layers()
            .iter()
            .map(|layer| {
                (
                    layer.network_weights(2).to_vec(),
                    layer.network_biases(2).to_vec(),
                )
            })
            .collect::<Vec<_>>();
        let expected_get_to_know = village
            .get_to_know_brains
            .layers()
            .iter()
            .map(|layer| {
                (
                    layer.network_weights(2).to_vec(),
                    layer.network_biases(2).to_vec(),
                )
            })
            .collect::<Vec<_>>();
        let mut expected_reader = vec![0.0; MONOMIAL_FEATURE_COUNT];
        village
            .readers
            .read_into(2, &village.action_brains, 2, &mut expected_reader)
            .unwrap();

        village.remove_entity(removed).unwrap();

        assert_eq!(village.population[1].id(), moved);
        assert_eq!(village.entity_indices[&moved], 1);
        for (slot, expected) in expected_known.into_iter().enumerate() {
            assert_eq!(
                (
                    village.known_people.occupant(1, slot),
                    village.known_people.is_active(1, slot),
                ),
                expected
            );
        }
        for (layer, (weights, biases)) in village.action_brains.layers().iter().zip(expected_action)
        {
            assert_eq!(layer.network_weights(1), weights);
            assert_eq!(layer.network_biases(1), biases);
        }
        for (layer, (weights, biases)) in village
            .get_to_know_brains
            .layers()
            .iter()
            .zip(expected_get_to_know)
        {
            assert_eq!(layer.network_weights(1), weights);
            assert_eq!(layer.network_biases(1), biases);
        }
        let mut actual_reader = vec![0.0; MONOMIAL_FEATURE_COUNT];
        village
            .readers
            .read_into(1, &village.action_brains, 1, &mut actual_reader)
            .unwrap();
        assert_eq!(actual_reader, expected_reader);
        village.assert_aligned().unwrap();
    }

    #[test]
    fn log_resources_are_bounded_and_monotonic() {
        assert_eq!(log_normalize(0), 0.0);
        assert!(log_normalize(10) < log_normalize(1_000));
        assert_eq!(log_normalize(u64::MAX as u128), 1.0);
        assert_eq!(log_normalize(u128::MAX), 1.0);
    }

    #[test]
    fn consumption_removes_up_to_twenty_five_resources_per_entity() {
        let mut village = Village::with_population(2, 41);
        village.population[0].set_resources(100);
        village.population[1].set_resources(10);

        let consumed = village.apply_consumption();

        assert_eq!(consumed, 35);
        assert_eq!(village.population[0].resources(), 75);
        assert_eq!(village.population[1].resources(), 0);
    }

    #[test]
    fn entity_dies_when_consumption_exhausts_its_resources() {
        let mut village = Village::with_population(1, 42);
        let doomed = village.population[0].id();
        village.population[0].set_resources(CONSUMPTION_PER_ITERATION);
        let mut evaluator = CpuEvaluator;

        let report = village.step(&mut evaluator).unwrap();

        assert_eq!(report.consumed_resources, CONSUMPTION_PER_ITERATION as u128);
        assert_eq!(report.deaths, vec![doomed]);
        assert!(village.population.is_empty());
        assert!(!village.entity_indices.contains_key(&doomed));
        assert_eq!(village.known_people.len(), 0);
        assert_eq!(village.action_brains.len(), 0);
        assert_eq!(village.get_to_know_brains.len(), 0);
        assert_eq!(village.readers.len(), 0);
    }
}
