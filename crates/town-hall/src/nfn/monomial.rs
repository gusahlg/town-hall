use anyhow::{Result, ensure};
use neuro_ash::{DenseNetworkPool, Random};

use crate::{ACTION_LAYER_SIZES, MONOMIAL_FEATURE_COUNT, monomial_feature_count};

#[derive(Debug, Clone, Copy)]
struct Affine {
    scale: f32,
    bias: f32,
}

impl Affine {
    fn random(random: &mut Random) -> Self {
        Self {
            scale: random.symmetric(1.0),
            bias: random.symmetric(0.25),
        }
    }

    #[inline]
    fn apply(self, value: f32) -> f32 {
        self.scale * value + self.bias
    }
}

#[derive(Debug, Clone, Copy)]
struct LayerParameters {
    weight: Affine,
    bias: Affine,
}

/// Minimal ReLU Monomial-NFN reader: sequential neuron-norm canonicalization
/// followed by observer-specific channel affines and the HNPS invariant
/// pooling pattern. Each hidden neuron's source-scaled incoming weights and
/// bias define an RMS scale that divides its incoming values and multiplies
/// its outgoing weights, so positive hidden-neuron rescaling is removed while
/// every relative magnitude survives; hidden-neuron permutations are removed
/// by pooling. Every source weight and bias is read.
#[derive(Debug)]
pub struct MonomialNfnPool {
    dense_layer_count: usize,
    parameters: Vec<LayerParameters>,
    readers: usize,
}

impl MonomialNfnPool {
    pub fn new(action_brains: &DenseNetworkPool, random: &mut Random) -> Self {
        Self::with_capacity(action_brains, action_brains.len(), random)
    }

    pub fn with_capacity(
        action_brains: &DenseNetworkPool,
        reader_capacity: usize,
        random: &mut Random,
    ) -> Self {
        let dense_layer_count = action_brains.shape().layer_count();
        let parameter_capacity = reader_capacity
            .max(action_brains.len())
            .checked_mul(dense_layer_count)
            .expect("Monomial reader capacity overflows usize");
        let mut pool = Self {
            dense_layer_count,
            parameters: Vec::with_capacity(parameter_capacity),
            readers: 0,
        };
        for _ in 0..action_brains.len() {
            pool.push_random(random);
        }
        pool
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.readers
    }

    #[inline]
    pub fn output_size(&self) -> usize {
        MONOMIAL_FEATURE_COUNT
    }

    pub fn push_random(&mut self, random: &mut Random) {
        self.parameters
            .extend((0..self.dense_layer_count).map(|_| LayerParameters {
                weight: Affine::random(random),
                bias: Affine::random(random),
            }));
        self.readers += 1;
    }

    pub fn swap_remove(&mut self, reader: usize) {
        assert!(reader < self.readers);
        let last = self.readers - 1;
        if reader != last {
            let source = last * self.dense_layer_count;
            let destination = reader * self.dense_layer_count;
            self.parameters
                .copy_within(source..source + self.dense_layer_count, destination);
        }
        self.parameters.truncate(last * self.dense_layer_count);
        self.readers -= 1;
    }

    pub fn shrink_to_fit(&mut self) {
        self.parameters.shrink_to_fit();
    }

    pub fn for_each_parameter_mut(&mut self, reader: usize, mut visitor: impl FnMut(&mut f32)) {
        assert!(reader < self.readers);
        let start = reader * self.dense_layer_count;
        for parameters in &mut self.parameters[start..start + self.dense_layer_count] {
            visitor(&mut parameters.weight.scale);
            visitor(&mut parameters.weight.bias);
            visitor(&mut parameters.bias.scale);
            visitor(&mut parameters.bias.bias);
        }
    }

    pub fn read_into(
        &self,
        observer: usize,
        observed_brains: &DenseNetworkPool,
        observed: usize,
        output: &mut [f32],
    ) -> Result<()> {
        ensure!(
            observer < self.readers,
            "reader index {observer} is out of bounds"
        );
        ensure!(
            observed < observed_brains.len(),
            "observed brain index is out of bounds"
        );
        ensure!(
            observed_brains.shape().widths() == ACTION_LAYER_SIZES,
            "Monomial reader expected the configured action-brain shape"
        );
        let expected = monomial_feature_count(observed_brains.shape().widths());
        ensure!(
            output.len() == expected,
            "reader output has {} values, expected {expected}",
            output.len()
        );

        let parameters = &self.parameters
            [observer * self.dense_layer_count..(observer + 1) * self.dense_layer_count];
        let layers = observed_brains.layers();
        let mut cursor = 0;
        // Canonical scales of the current layer's source neurons; the fixed
        // action inputs are unscaled.
        let mut scales = vec![1.0_f32; layers[0].input_size()];
        let mut next_scales: Vec<f32> = Vec::new();

        for (layer_index, (layer, parameters)) in layers.iter().zip(parameters).enumerate() {
            let weights = layer.network_weights(observed);
            let biases = layer.network_biases(observed);
            let is_first = layer_index == 0;
            let is_last = layer_index + 1 == layers.len();
            let (input_size, output_size) = (layer.input_size(), layer.output_size());

            if !is_last {
                // RMS over a target neuron's source-scaled incoming weights
                // and bias: equivariant to that neuron's positive rescaling,
                // invariant to all earlier ones. Dead neurons keep zero.
                next_scales.clear();
                next_scales.extend((0..output_size).map(|target| {
                    let mut sum = biases[target].to_f32().powi(2);
                    for source in 0..input_size {
                        let value =
                            weights[source * output_size + target].to_f32() * scales[source];
                        sum += value * value;
                    }
                    (sum / (input_size + 1) as f32).sqrt()
                }));
            }
            let canonical = |source: usize, target: usize| -> f32 {
                let value = weights[source * output_size + target].to_f32() * scales[source];
                if is_last {
                    value
                } else if next_scales[target] > 0.0 {
                    value / next_scales[target]
                } else {
                    0.0
                }
            };

            if is_first {
                // The action inputs have fixed meanings, so retain one pooled
                // feature per input while removing first-hidden-neuron order.
                for input in 0..input_size {
                    let mut sum = 0.0;
                    for hidden in 0..output_size {
                        sum += parameters.weight.apply(canonical(input, hidden));
                    }
                    output[cursor] = sum / output_size as f32;
                    cursor += 1;
                }
            } else if is_last {
                // Action outputs also have fixed meanings. Pool only over the
                // final hidden layer and retain one feature per output.
                for action_output in 0..output_size {
                    let mut sum = 0.0;
                    for hidden in 0..input_size {
                        sum += parameters.weight.apply(canonical(hidden, action_output));
                    }
                    output[cursor] = sum / input_size as f32;
                    cursor += 1;
                }
            } else {
                let mut sum = 0.0;
                for source in 0..input_size {
                    for target in 0..output_size {
                        sum += parameters.weight.apply(canonical(source, target));
                    }
                }
                output[cursor] = sum / (input_size * output_size) as f32;
                cursor += 1;
            }

            if is_last {
                // Output biases are outside every hidden symmetry, so retain
                // all of them at full value.
                for &bias in biases {
                    output[cursor] = parameters.bias.apply(bias.to_f32());
                    cursor += 1;
                }
            } else {
                let mut sum = 0.0;
                for (target, &bias) in biases.iter().enumerate() {
                    let canonical_bias = if next_scales[target] > 0.0 {
                        bias.to_f32() / next_scales[target]
                    } else {
                        0.0
                    };
                    sum += parameters.bias.apply(canonical_bias);
                }
                output[cursor] = sum / biases.len() as f32;
                cursor += 1;
            }

            if !is_last {
                std::mem::swap(&mut scales, &mut next_scales);
            }
        }

        ensure!(
            cursor == output.len(),
            "internal Monomial feature-count mismatch"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neuro_ash::{NetworkShape, f16};

    fn action_pool(seed: u64) -> DenseNetworkPool {
        let mut random = Random::new(seed);
        DenseNetworkPool::random(
            NetworkShape::new(ACTION_LAYER_SIZES.to_vec()).unwrap(),
            1,
            &mut random,
        )
    }

    #[test]
    fn output_has_the_derived_fixed_size() {
        let brains = action_pool(3);
        let mut random = Random::new(4);
        let readers = MonomialNfnPool::new(&brains, &mut random);
        let mut output = vec![0.0; MONOMIAL_FEATURE_COUNT];
        readers.read_into(0, &brains, 0, &mut output).unwrap();
        assert!(output.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn positive_hidden_rescaling_does_not_change_features() {
        let original = action_pool(7);
        let mut transformed = action_pool(7);
        let hidden = 2;
        let scale = f16::from_f32(3.5);
        let first_output = ACTION_LAYER_SIZES[1];

        {
            let weights = transformed.network_weights_mut(0, 0).unwrap();
            for input in 0..ACTION_LAYER_SIZES[0] {
                weights[input * first_output + hidden] *= scale;
            }
        }
        transformed.network_biases_mut(0, 0).unwrap()[hidden] *= scale;
        {
            let weights = transformed.network_weights_mut(1, 0).unwrap();
            let next_width = ACTION_LAYER_SIZES[2];
            for output in 0..next_width {
                weights[hidden * next_width + output] /= scale;
            }
        }

        let mut random = Random::new(8);
        let readers = MonomialNfnPool::new(&original, &mut random);
        let mut left = vec![0.0; MONOMIAL_FEATURE_COUNT];
        let mut right = vec![0.0; MONOMIAL_FEATURE_COUNT];
        readers.read_into(0, &original, 0, &mut left).unwrap();
        readers.read_into(0, &transformed, 0, &mut right).unwrap();
        let max_error = left
            .iter()
            .zip(&right)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0_f32, f32::max);
        // The scaled parameters are themselves rounded to f16 storage, so the
        // reading is invariant only up to that rounding.
        assert!(max_error <= 2.0e-2, "rescaling error {max_error}");
    }

    #[test]
    fn uncompensated_magnitude_changes_are_visible() {
        let original = action_pool(13);
        let mut transformed = action_pool(13);
        let hidden = 5;
        let first_output = ACTION_LAYER_SIZES[1];

        // Scale one hidden neuron's incoming weights without touching its
        // bias or compensating downstream. This changes the network function
        // but no weight signs, so a sign-only reading cannot see it.
        {
            let weights = transformed.network_weights_mut(0, 0).unwrap();
            for input in 0..ACTION_LAYER_SIZES[0] {
                weights[input * first_output + hidden] *= f16::from_f32(3.5);
            }
        }

        let mut random = Random::new(14);
        let readers = MonomialNfnPool::new(&original, &mut random);
        let mut left = vec![0.0; MONOMIAL_FEATURE_COUNT];
        let mut right = vec![0.0; MONOMIAL_FEATURE_COUNT];
        readers.read_into(0, &original, 0, &mut left).unwrap();
        readers.read_into(0, &transformed, 0, &mut right).unwrap();
        let max_difference = left
            .iter()
            .zip(&right)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_difference > 1.0e-3,
            "a functional magnitude change was invisible: {max_difference}"
        );
    }

    #[test]
    fn hidden_neuron_permutation_does_not_change_features() {
        let original = action_pool(11);
        let mut transformed = action_pool(11);
        let (first_hidden, next_width) = (ACTION_LAYER_SIZES[1], ACTION_LAYER_SIZES[2]);
        let (left_hidden, right_hidden) = (3, 17);

        {
            let weights = transformed.network_weights_mut(0, 0).unwrap();
            for input in 0..ACTION_LAYER_SIZES[0] {
                weights.swap(
                    input * first_hidden + left_hidden,
                    input * first_hidden + right_hidden,
                );
            }
        }
        transformed
            .network_biases_mut(0, 0)
            .unwrap()
            .swap(left_hidden, right_hidden);
        {
            let weights = transformed.network_weights_mut(1, 0).unwrap();
            for output in 0..next_width {
                weights.swap(
                    left_hidden * next_width + output,
                    right_hidden * next_width + output,
                );
            }
        }

        let mut random = Random::new(12);
        let readers = MonomialNfnPool::new(&original, &mut random);
        let mut left = vec![0.0; MONOMIAL_FEATURE_COUNT];
        let mut right = vec![0.0; MONOMIAL_FEATURE_COUNT];
        readers.read_into(0, &original, 0, &mut left).unwrap();
        readers.read_into(0, &transformed, 0, &mut right).unwrap();
        let max_error = left
            .iter()
            .zip(&right)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0_f32, f32::max);
        assert!(max_error <= 1.0e-6, "permutation error {max_error}");
    }
}
