use strum::EnumCount;
const STARTING_POPULATION: u128 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Person,
    Company,
}

// What actions should there be?
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount)]
pub enum Action {
    Predict { amount: u64, target: EntityId },
    Conserve,
    Give { amount: u64, target: EntityId },
}

const ACTION_COUNT: usize = <Action as strum::EnumCount>::COUNT;

struct Entity {
    pub kind: EntityKind,
    pub resources: u64,
    pub owners: Option<EntityId>,
    pub id: EntityId,
}
impl Entity {
    pub fn new(kind: EntityKind, resources: u64, owners: Option<EntityId>, id: EntityId) -> Self {
        Self {
            kind,
            resources,
            owners,
            id,
        }
    }

    pub fn action(&self) -> Action {
        Action::Conserve
    }
}
pub struct Village {
    pub population: Vec<Entity>,
    pub brain_pool: BrainPool,
    pub iterations: u128,
    pub resources: u128,
    pub growth_rate: u64,
}
impl Village {
    pub fn new() -> Self {
        let population = (0..STARTING_POPULATION)
            .map(|id| {
                let layer_sizes: &[usize] = &[5, 25, 100, ACTION_COUNT];
                Entity::new(EntityKind::Person, 100, None, EntityId(id))
            })
            .collect();

        let brain_pool = BrainPool::new(STARTING_POPULATION as usize);
        Self {
            population,
            brain_pool,
            iterations: 0,
            resources: 1000,
            growth_rate: 100,
        }
    }
}

const LAYER_SIZES: &[usize] = &[3, 30, 75, 20, ACTION_COUNT];
const fn parameter_counts(layer_sizes: &[usize]) -> (usize, usize) {
    let mut weights = 0;
    let mut biases = 0;
    let mut layer = 1;

    while layer < layer_sizes.len() {
        let previous_size = layer_sizes[layer - 1];
        let current_size = layer_sizes[layer];

        weights += previous_size * current_size;
        biases += current_size;

        layer += 1;
    }

    (weights, biases)
}

const PARAMETER_COUNTS: (usize, usize) = parameter_counts(LAYER_SIZES);

const WEIGHTS_PER_BRAIN: usize = PARAMETER_COUNTS.0;
const BIASES_PER_BRAIN: usize = PARAMETER_COUNTS.1;

pub struct BrainPool {
    // Used for looking up based on the associated EntityId, the id is the index
    pub weights_offsets: Vec<usize>,
    pub biases_offsets: Vec<usize>,

    // The weights and biases of all of the brains
    pub weights: Vec<f32>,
    pub biases: Vec<f32>,
}
impl BrainPool {
    pub fn new(networks: usize) -> Self {
        let weights: Vec<f32> = vec![1.0; networks * WEIGHTS_PER_BRAIN];
        let biases: Vec<f32> = vec![1.0; networks * BIASES_PER_BRAIN];

        let weights_offsets: Vec<usize> = (0..networks)
            .map(|brain_index| brain_index * WEIGHTS_PER_BRAIN)
            .collect();

        let biases_offsets: Vec<usize> = (0..networks)
            .map(|brain_index| brain_index * BIASES_PER_BRAIN)
            .collect();

        Self {
            weights_offsets,
            biases_offsets,
            weights,
            biases,
        }
    }

    pub fn add_brains(&mut self, additions: usize) {
        for _ in 0..additions {
            self.weights_offsets.push(self.weights.len());
            self.biases_offsets.push(self.biases.len());

            for _ in 0..WEIGHTS_PER_BRAIN {
                // NOTE: THIS SHOULD BE RANDOMIZED OR DERIVED LATER
                self.weights.push(0.0);
            }
            for _ in 0..BIASES_PER_BRAIN {
                // NOTE: THIS SHOULD BE RANDOMIZED OR DERIVED LATER
                self.biases.push(0.0);
            }
        }
    }

    // Removes a brain at a specified index
    // NOTE: The EntityId of the last entity has to be switched too for it to be a complete removal.
    pub fn remove_brain(&mut self, brain_index: usize) {
        let weight_index = self.weights_offsets[brain_index];
        let bias_index = self.biases_offsets[brain_index];

        // Replace the brain with the last brain and truncate
        for i in (0..WEIGHTS_PER_BRAIN).rev() {
            self.weights.swap_remove(weight_index + i);
        }
        for i in (0..BIASES_PER_BRAIN).rev() {
            self.biases.swap_remove(bias_index + i);
        }

        self.weights_offsets.pop();
        self.biases_offsets.pop();
    }
}
