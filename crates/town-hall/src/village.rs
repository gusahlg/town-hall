use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use anyhow::{Result, ensure};
use neuro_ash::{BatchEvaluator, DenseNetworkPool, NetworkShape, Random};
use rayon::prelude::*;

use crate::{
    ACTION_INPUT_COUNT, ACTION_LAYER_SIZES, ACTION_OUTPUT_COUNT, Action, BASE_INPUT_COUNT,
    CONNECTION_BUNDLE_SIZE, CONSERVE_OUTPUT, ConnectionBundle, DEFAULT_RANDOM_SEED, Economy,
    Entity, EntityId, EntityKind, GET_TO_KNOW_LAYER_SIZES, GIVE_OUTPUT_START, Inventory,
    KNOWN_PERSON_COUNT, KNOWN_PERSON_MASK_WORDS, KnownPeoplePool, MONOMIAL_FEATURE_COUNT, Need,
    NeedComponent, PREDICT_OUTPUT_START, Position, PropertyPoint, ReaderKind, ReaderPool,
    STARTING_POPULATION, World, give_output, predict_output,
};

const INITIAL_ENTITY_MONEY: u64 = 100;
const BASELINE_NEED_RECOVERY_RATE: f32 = 0.05;
const PARALLEL_READER_THRESHOLD: usize = 128;

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
    economy: Economy,
    world: World,
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
        let world = World::default();
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
            economy: Economy::default(),
            world,
        };
        for _ in 0..population {
            village.spawn_person(INITIAL_ENTITY_MONEY);
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

    #[inline]
    pub fn economy(&self) -> &Economy {
        &self.economy
    }

    #[inline]
    pub fn economy_mut(&mut self) -> &mut Economy {
        &mut self.economy
    }

    #[inline]
    pub fn world(&self) -> &World {
        &self.world
    }

    #[inline]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn total_money(&self) -> u128 {
        self.population
            .iter()
            .map(|entity| entity.money() as u128)
            .sum()
    }

    pub fn spawn_person(&mut self, money: u64) -> EntityId {
        let location = self.next_spawn_position();
        let needs = self.baseline_person_needs();
        self.spawn(
            EntityKind::Person,
            Inventory::default(),
            money,
            location,
            needs,
        )
    }

    /// Minimal common demand until population-level need generation is chosen.
    /// With no satisfaction, this baseline becomes fatal after roughly 1,000
    /// state updates.
    fn baseline_person_needs(&self) -> Vec<Need> {
        let dimensions = self.economy.resources().dimensions();
        let ideal = PropertyPoint::new(vec![0.5; dimensions])
            .expect("the economy has a non-empty finite property space");
        vec![
            Need::new(
                vec![NeedComponent { ideal, quantity: 1 }],
                0.0,
                BASELINE_NEED_RECOVERY_RATE,
            )
            .expect("the baseline person need is valid"),
        ]
    }

    pub fn remove_entity(&mut self, id: EntityId) -> Result<Entity> {
        self.assert_aligned()?;
        let index = self.entity_index(id)?;
        let removed = self.swap_remove_entity(index)?;
        self.known_people.forget(&HashSet::from([id]));
        self.shrink_storage_if_sparse();
        self.assert_aligned()?;
        Ok(removed)
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
    /// update/prune -> snapshot/rank -> learn replacements -> atomically patch
    /// -> act -> apply.
    pub fn step(&mut self, evaluator: &mut impl BatchEvaluator) -> Result<()> {
        self.assert_aligned()?;
        for entity in &mut self.population {
            entity.update_state();
        }
        self.remove_dead_entities()?;
        if self.population.is_empty() {
            evaluator.release(&self.action_brains);
            evaluator.release(&self.get_to_know_brains);
            self.iterations = self.iterations.saturating_add(1);
            return Ok(());
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

        self.apply_actions(&actions);
        for (entity, &action) in self.population.iter_mut().zip(&actions) {
            entity.set_last_action(action);
        }
        self.iterations = self.iterations.saturating_add(1);
        Ok(())
    }

    fn spawn(
        &mut self,
        kind: EntityKind,
        inventory: Inventory,
        money: u64,
        location: Position,
        needs: Vec<Need>,
    ) -> EntityId {
        assert!(
            self.world.contains(location),
            "entity location is outside the world"
        );
        let id = EntityId(self.next_entity_id);
        self.next_entity_id = self
            .next_entity_id
            .checked_add(1)
            .expect("entity ID space exhausted");
        let index = self.population.len();
        self.population
            .push(Entity::new(kind, inventory, money, location, needs, id));
        self.entity_indices.insert(id, index);
        self.known_people.push_empty();
        self.action_brains.push_random(&mut self.random);
        self.get_to_know_brains.push_random(&mut self.random);
        self.readers.push_random(&mut self.random);
        id
    }

    fn next_spawn_position(&self) -> Position {
        let tile_count = u128::from(self.world.width()) * u128::from(self.world.height());
        let tile = self.next_entity_id % tile_count;
        Position::new(
            (tile % u128::from(self.world.width())) as u32,
            (tile / u128::from(self.world.width())) as u32,
        )
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

    fn remove_dead_entities(&mut self) -> Result<()> {
        let dead = self
            .population
            .iter()
            .filter(|entity| !entity.alive())
            .map(Entity::id)
            .collect::<HashSet<_>>();
        if dead.is_empty() {
            return Ok(());
        }

        let mut index = 0;
        while index < self.population.len() {
            if dead.contains(&self.population[index].id()) {
                self.swap_remove_entity(index)?;
            } else {
                index += 1;
            }
        }
        self.known_people.forget(&dead);
        self.shrink_storage_if_sparse();
        self.assert_aligned()
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
                .money()
                .cmp(&self.population[left].money())
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
        let world_material = log_normalize(self.world.total_deposited_material());

        for (entity_index, entity) in self.population.iter().enumerate() {
            let row = &mut inputs
                [entity_index * ACTION_INPUT_COUNT..(entity_index + 1) * ACTION_INPUT_COUNT];
            row[0] = log_normalize(entity.money() as u128);
            row[1] = log_normalize(entity.inventory().total_units());
            row[2] = world_material;

            for slot in 0..KNOWN_PERSON_COUNT {
                if !self.known_people.is_active(entity_index, slot) {
                    continue;
                }
                let Some(known_id) = self.known_people.occupant(entity_index, slot) else {
                    continue;
                };
                if let Some(&known_index) = self.entity_indices.get(&known_id) {
                    row[BASE_INPUT_COUNT + slot] =
                        log_normalize(self.population[known_index].money() as u128);
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
        let available = self.population[entity].money();
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

    /// Temporary action adapter. Give transfers are staged and Predict has no
    /// world effect; neither behavior is part of the settled economic model.
    fn apply_actions(&mut self, actions: &[Action]) {
        assert_eq!(
            actions.len(),
            self.population.len(),
            "every live entity must contribute exactly one action"
        );
        let before = self
            .population
            .iter()
            .map(Entity::money)
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
            self.population[entity].set_money(updated);
        }
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
            ensure!(
                self.world.contains(entity.location()),
                "entity {:?} is outside the world",
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

    #[derive(Default)]
    struct RecordingEvaluator {
        population_sizes: Vec<usize>,
    }

    impl BatchEvaluator for RecordingEvaluator {
        fn evaluate(&mut self, pool: &DenseNetworkPool, inputs: &[f32]) -> Result<Vec<f32>> {
            self.population_sizes.push(pool.len());
            pool.forward_cpu(inputs)
        }
    }

    #[test]
    fn a_cpu_tick_preserves_alignment() {
        let mut village = Village::with_population(4, 1);
        village.step(&mut CpuEvaluator).unwrap();

        assert_eq!(village.iterations(), 1);
        assert_eq!(village.entities().len(), 4);
        assert!(village.action_brains.all_finite());
        village.assert_aligned().unwrap();
    }

    #[test]
    fn default_people_eventually_die_without_satisfaction() {
        let mut village = Village::with_population(1, 1);
        let entity = &mut village.population[0];
        assert_eq!(entity.needs().len(), 1);
        assert!(!entity.needs()[0].target.is_empty());

        for _ in 0..5_000 {
            entity.update_state();
        }

        assert!(!entity.alive());
    }

    #[test]
    fn dead_entities_are_pruned_before_neural_evaluation() {
        let mut village = Village::with_population(2, 1);
        let dead = village.spawn(
            EntityKind::Person,
            Inventory::default(),
            INITIAL_ENTITY_MONEY,
            village.next_spawn_position(),
            vec![Need::new(Vec::new(), 0.95, 0.1).unwrap()],
        );
        let mut evaluator = RecordingEvaluator::default();

        village.step(&mut evaluator).unwrap();

        assert!(!village.entity_indices.contains_key(&dead));
        assert_eq!(village.entities().len(), 2);
        assert!(!evaluator.population_sizes.is_empty());
        assert!(
            evaluator
                .population_sizes
                .iter()
                .all(|&population| population == 2)
        );
        village.assert_aligned().unwrap();
    }

    #[test]
    fn each_tick_updates_need_intensity_before_survival_is_checked() {
        let mut village = Village::with_population(0, 1);
        let entity = village.spawn(
            EntityKind::Person,
            Inventory::default(),
            INITIAL_ENTITY_MONEY,
            village.next_spawn_position(),
            vec![Need::new(Vec::new(), 0.2, 0.1).unwrap()],
        );

        village.step(&mut CpuEvaluator).unwrap();

        let entity = &village.population[village.entity_indices[&entity]];
        assert!((entity.needs()[0].intensity - 0.3).abs() <= f32::EPSILON);
    }

    #[test]
    fn a_new_person_is_relearnt_after_a_known_person_is_removed() {
        let mut village = Village::with_population(3, 2);
        village.step(&mut CpuEvaluator).unwrap();
        let removed = village.entities()[1].id();

        village.remove_entity(removed).unwrap();
        for row in 0..village.entities().len() {
            assert!(
                (0..KNOWN_PERSON_COUNT)
                    .all(|slot| village.known_people.occupant(row, slot) != Some(removed))
            );
        }

        let newcomer = village.spawn_person(INITIAL_ENTITY_MONEY);
        village.step(&mut CpuEvaluator).unwrap();

        assert!(village.entity_indices.contains_key(&newcomer));
        for (row, entity) in village.entities().iter().enumerate() {
            let expected = village
                .entities()
                .iter()
                .map(Entity::id)
                .filter(|&id| id != entity.id())
                .collect::<HashSet<_>>();
            let actual = (0..KNOWN_PERSON_COUNT)
                .filter(|&slot| village.known_people.is_active(row, slot))
                .filter_map(|slot| village.known_people.occupant(row, slot))
                .collect::<HashSet<_>>();
            assert_eq!(actual, expected);
        }
        village.assert_aligned().unwrap();
    }
}
