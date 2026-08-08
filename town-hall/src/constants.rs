//! Architectural constants. The social slot count and the action-head layout
//! are deliberate simulation rules; derived counts must not be tuned by hand.

pub const STARTING_POPULATION: usize = 25_001;
pub const DEFAULT_RANDOM_SEED: u64 = 0x5eed_5eed_cafe_babe;
pub const CONSUMPTION_PER_ITERATION: u64 = 25;

pub const KNOWN_PERSON_COUNT: usize = 200;
pub const KNOWN_PERSON_MASK_WORDS: usize = KNOWN_PERSON_COUNT.div_ceil(64);
pub const BASE_INPUT_COUNT: usize = 3;
pub const ACTION_INPUT_COUNT: usize = BASE_INPUT_COUNT + KNOWN_PERSON_COUNT;

pub const CONSERVE_OUTPUT: usize = 0;
pub const PREDICT_OUTPUT_START: usize = CONSERVE_OUTPUT + 1;
pub const GIVE_OUTPUT_START: usize = PREDICT_OUTPUT_START + KNOWN_PERSON_COUNT;
pub const ACTION_OUTPUT_COUNT: usize = 1 + 2 * KNOWN_PERSON_COUNT;

pub const FIRST_HIDDEN_SIZE: usize = 30;
pub const MIDDLE_HIDDEN_SIZE: usize = 75;
pub const LAST_HIDDEN_SIZE: usize = 20;
pub const ACTION_LAYER_SIZES: [usize; 5] = [
    ACTION_INPUT_COUNT,
    FIRST_HIDDEN_SIZE,
    MIDDLE_HIDDEN_SIZE,
    LAST_HIDDEN_SIZE,
    ACTION_OUTPUT_COUNT,
];

/// The minimal Monomial-NFN reader retains fixed input/output coordinates and
/// pools over every hidden-neuron coordinate. For each dense layer it emits:
/// first `(inputs + one bias summary)`, middle `(weight + bias summaries)`, and
/// last `(one weight summary + one bias value per output)`.
pub const fn monomial_feature_count(layer_sizes: &[usize]) -> usize {
    let dense_layers = layer_sizes.len() - 1;
    let middle_layers = dense_layers - 2;
    layer_sizes[0] + 1 + middle_layers * 2 + 2 * layer_sizes[layer_sizes.len() - 1]
}

pub const MONOMIAL_FEATURE_COUNT: usize = monomial_feature_count(&ACTION_LAYER_SIZES);

pub const INPUT_HEAD_SIZE: usize = FIRST_HIDDEN_SIZE;
pub const TARGET_HEAD_SIZE: usize = LAST_HIDDEN_SIZE + 1;
pub const CONNECTION_BUNDLE_SIZE: usize = INPUT_HEAD_SIZE + 2 * TARGET_HEAD_SIZE;

pub const GET_TO_KNOW_HIDDEN_SIZE: usize = 32;
pub const GET_TO_KNOW_LAYER_SIZES: [usize; 3] = [
    MONOMIAL_FEATURE_COUNT,
    GET_TO_KNOW_HIDDEN_SIZE,
    CONNECTION_BUNDLE_SIZE,
];

pub const fn predict_output(slot: usize) -> usize {
    PREDICT_OUTPUT_START + slot
}

pub const fn give_output(slot: usize) -> usize {
    GIVE_OUTPUT_START + slot
}

const _: () = assert!(MONOMIAL_FEATURE_COUNT == 1010);
const _: () = assert!(ACTION_OUTPUT_COUNT == 401);
const _: () = assert!(CONNECTION_BUNDLE_SIZE == 72);
