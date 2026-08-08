use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, ensure};

static NEXT_POOL_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkShape {
    widths: Box<[usize]>,
}

impl NetworkShape {
    pub fn new(widths: impl Into<Box<[usize]>>) -> Result<Self> {
        let widths = widths.into();
        ensure!(
            widths.len() >= 2,
            "a dense network needs at least two layers"
        );
        for (index, &width) in widths.iter().enumerate() {
            ensure!(width > 0, "network layer {index} must not be empty");
            u32::try_from(width)
                .map_err(|_| anyhow::anyhow!("network layer {index} exceeds the GPU u32 limit"))?;
        }
        let weights = widths.windows(2).try_fold(0_usize, |total, pair| {
            let layer = pair[0]
                .checked_mul(pair[1])
                .ok_or_else(|| anyhow::anyhow!("network weight count overflows usize"))?;
            total
                .checked_add(layer)
                .ok_or_else(|| anyhow::anyhow!("network weight count overflows usize"))
        })?;
        let biases = widths.iter().skip(1).try_fold(0_usize, |total, &width| {
            total
                .checked_add(width)
                .ok_or_else(|| anyhow::anyhow!("network bias count overflows usize"))
        })?;
        weights
            .checked_add(biases)
            .ok_or_else(|| anyhow::anyhow!("network parameter count overflows usize"))?;
        Ok(Self { widths })
    }

    #[inline]
    pub fn widths(&self) -> &[usize] {
        &self.widths
    }

    #[inline]
    pub fn input_size(&self) -> usize {
        self.widths[0]
    }

    #[inline]
    pub fn output_size(&self) -> usize {
        *self.widths.last().expect("validated non-empty shape")
    }

    #[inline]
    pub fn layer_count(&self) -> usize {
        self.widths.len() - 1
    }

    pub fn weights_per_network(&self) -> usize {
        self.widths.windows(2).map(|pair| pair[0] * pair[1]).sum()
    }

    pub fn biases_per_network(&self) -> usize {
        self.widths.iter().skip(1).sum()
    }

    pub fn parameters_per_network(&self) -> usize {
        self.weights_per_network() + self.biases_per_network()
    }
}

/// Small deterministic random source. Simulation seeds therefore reproduce
/// every initial brain without introducing a dependency on a global RNG.
#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
}

impl Random {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_f32(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 40) as f32 / (1_u32 << 24) as f32
    }

    pub fn symmetric(&mut self, limit: f32) -> f32 {
        (2.0 * self.next_f32() - 1.0) * limit
    }
}

#[derive(Debug)]
pub struct DenseLayerPool {
    input_size: usize,
    output_size: usize,
    /// `[network, input, output]`, contiguous and ready for rank-3 BMM.
    weights: Vec<f32>,
    /// `[network, output]`.
    biases: Vec<f32>,
    revision: u64,
}

impl DenseLayerPool {
    fn with_network_capacity(
        input_size: usize,
        output_size: usize,
        network_capacity: usize,
    ) -> Self {
        let weight_capacity = network_capacity
            .checked_mul(input_size)
            .and_then(|count| count.checked_mul(output_size))
            .expect("dense-layer weight capacity overflows usize");
        let bias_capacity = network_capacity
            .checked_mul(output_size)
            .expect("dense-layer bias capacity overflows usize");
        Self {
            input_size,
            output_size,
            weights: Vec::with_capacity(weight_capacity),
            biases: Vec::with_capacity(bias_capacity),
            revision: 0,
        }
    }

    #[inline]
    pub fn input_size(&self) -> usize {
        self.input_size
    }

    #[inline]
    pub fn output_size(&self) -> usize {
        self.output_size
    }

    #[inline]
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }

    #[inline]
    pub fn biases(&self) -> &[f32] {
        &self.biases
    }

    pub fn network_weights(&self, network: usize) -> &[f32] {
        let stride = self.input_size * self.output_size;
        &self.weights[network * stride..(network + 1) * stride]
    }

    pub fn network_biases(&self, network: usize) -> &[f32] {
        &self.biases[network * self.output_size..(network + 1) * self.output_size]
    }

    fn push_random(&mut self, random: &mut Random) {
        let limit = (6.0 / (self.input_size + self.output_size) as f32).sqrt();
        self.weights
            .extend((0..self.input_size * self.output_size).map(|_| random.symmetric(limit)));
        // Biases are random as part of an entity's completely random initial
        // brain. A smaller limit keeps the first forwards numerically calm.
        self.biases
            .extend((0..self.output_size).map(|_| random.symmetric(limit * 0.25)));
        self.bump_revision();
    }

    fn swap_remove_network(&mut self, network: usize, network_count: usize) {
        let last = network_count - 1;
        let weight_stride = self.input_size * self.output_size;
        if network != last {
            self.weights.copy_within(
                last * weight_stride..(last + 1) * weight_stride,
                network * weight_stride,
            );
            self.biases.copy_within(
                last * self.output_size..(last + 1) * self.output_size,
                network * self.output_size,
            );
        }
        self.weights.truncate(last * weight_stride);
        self.biases.truncate(last * self.output_size);
        self.bump_revision();
    }

    #[inline]
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    fn shrink_to_fit(&mut self) {
        self.weights.shrink_to_fit();
        self.biases.shrink_to_fit();
    }
}

/// Equal-shaped private networks held as one layer-major parameter pool.
#[derive(Debug)]
pub struct DenseNetworkPool {
    id: u64,
    shape: NetworkShape,
    layers: Vec<DenseLayerPool>,
    network_count: usize,
}

impl DenseNetworkPool {
    pub fn new(shape: NetworkShape) -> Self {
        Self::with_capacity(shape, 0)
    }

    pub fn with_capacity(shape: NetworkShape, network_capacity: usize) -> Self {
        let layers = shape
            .widths()
            .windows(2)
            .map(|pair| DenseLayerPool::with_network_capacity(pair[0], pair[1], network_capacity))
            .collect();
        Self {
            id: NEXT_POOL_ID.fetch_add(1, Ordering::Relaxed),
            shape,
            layers,
            network_count: 0,
        }
    }

    pub fn random(shape: NetworkShape, network_count: usize, random: &mut Random) -> Self {
        let mut pool = Self::with_capacity(shape, network_count);
        for _ in 0..network_count {
            pool.push_random(random);
        }
        pool
    }

    #[inline]
    pub fn shape(&self) -> &NetworkShape {
        &self.shape
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.network_count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.network_count == 0
    }

    #[inline]
    pub fn layers(&self) -> &[DenseLayerPool] {
        &self.layers
    }

    /// Mutable parameters for one layer of one network. Taking this view marks
    /// the pool dirty so the next GPU evaluation uploads the changed values.
    pub fn network_weights_mut(&mut self, layer: usize, network: usize) -> Result<&mut [f32]> {
        ensure!(
            network < self.network_count,
            "network index {network} is out of bounds"
        );
        ensure!(
            layer < self.layers.len(),
            "layer index {layer} is out of bounds"
        );
        let layer = &mut self.layers[layer];
        layer.bump_revision();
        let stride = layer.input_size * layer.output_size;
        Ok(&mut layer.weights[network * stride..(network + 1) * stride])
    }

    /// Mutable biases for one layer of one network. This is the evolution and
    /// experimentation seam; normal slot updates should use the narrower patch
    /// methods below.
    pub fn network_biases_mut(&mut self, layer: usize, network: usize) -> Result<&mut [f32]> {
        ensure!(
            network < self.network_count,
            "network index {network} is out of bounds"
        );
        ensure!(
            layer < self.layers.len(),
            "layer index {layer} is out of bounds"
        );
        let layer = &mut self.layers[layer];
        layer.bump_revision();
        let stride = layer.output_size;
        Ok(&mut layer.biases[network * stride..(network + 1) * stride])
    }

    pub fn push_random(&mut self, random: &mut Random) -> usize {
        let index = self.network_count;
        for layer in &mut self.layers {
            layer.push_random(random);
        }
        self.network_count += 1;
        index
    }

    /// Release host capacity left behind by a large population decrease.
    /// Callers decide when shrinking is worthwhile to avoid allocation churn.
    pub fn shrink_to_fit(&mut self) {
        for layer in &mut self.layers {
            layer.shrink_to_fit();
        }
    }

    pub fn swap_remove(&mut self, network: usize) -> Result<()> {
        ensure!(
            network < self.network_count,
            "network index {network} is out of bounds"
        );
        for layer in &mut self.layers {
            layer.swap_remove_network(network, self.network_count);
        }
        self.network_count -= 1;
        Ok(())
    }

    /// Replace every connection from one input neuron into the first hidden
    /// layer of one network.
    pub fn set_input_connections(
        &mut self,
        network: usize,
        input: usize,
        connections: &[f32],
    ) -> Result<()> {
        ensure!(
            network < self.network_count,
            "network index {network} is out of bounds"
        );
        let first = &mut self.layers[0];
        ensure!(
            input < first.input_size,
            "input index {input} is out of bounds"
        );
        ensure!(
            connections.len() == first.output_size,
            "received {} input connections, expected {}",
            connections.len(),
            first.output_size
        );
        let stride = first.input_size * first.output_size;
        let start = network * stride + input * first.output_size;
        first.weights[start..start + first.output_size].copy_from_slice(connections);
        first.bump_revision();
        Ok(())
    }

    /// Replace every incoming connection and the bias of one output neuron.
    pub fn set_output_neuron(
        &mut self,
        network: usize,
        output: usize,
        connections: &[f32],
        bias: f32,
    ) -> Result<()> {
        ensure!(
            network < self.network_count,
            "network index {network} is out of bounds"
        );
        let last = self.layers.last_mut().expect("validated network shape");
        ensure!(
            output < last.output_size,
            "output index {output} is out of bounds"
        );
        ensure!(
            connections.len() == last.input_size,
            "received {} output connections, expected {}",
            connections.len(),
            last.input_size
        );
        let weight_stride = last.input_size * last.output_size;
        let base = network * weight_stride;
        for (input, &value) in connections.iter().enumerate() {
            last.weights[base + input * last.output_size + output] = value;
        }
        last.biases[network * last.output_size + output] = bias;
        last.bump_revision();
        Ok(())
    }

    pub fn forward_cpu(&self, inputs: &[f32]) -> Result<Vec<f32>> {
        self.forward_cpu_range(0..self.network_count, inputs)
    }

    pub(crate) fn forward_cpu_range(
        &self,
        networks: Range<usize>,
        inputs: &[f32],
    ) -> Result<Vec<f32>> {
        ensure!(
            networks.start <= networks.end && networks.end <= self.network_count,
            "invalid CPU network range"
        );
        let network_count = networks.len();
        let input_size = self.shape.input_size();
        ensure!(
            inputs.len() == network_count * input_size,
            "received {} inputs, expected {} networks * {input_size}",
            inputs.len(),
            network_count
        );
        if network_count == 0 {
            return Ok(Vec::new());
        }

        let mut values = inputs.to_vec();
        for (layer_index, layer) in self.layers.iter().enumerate() {
            let mut outputs = vec![0.0; network_count * layer.output_size];
            let weight_stride = layer.input_size * layer.output_size;
            for local_network in 0..network_count {
                let network = networks.start + local_network;
                let input_row = &values
                    [local_network * layer.input_size..(local_network + 1) * layer.input_size];
                let weights =
                    &layer.weights[network * weight_stride..(network + 1) * weight_stride];
                let biases =
                    &layer.biases[network * layer.output_size..(network + 1) * layer.output_size];
                let output_row = &mut outputs
                    [local_network * layer.output_size..(local_network + 1) * layer.output_size];
                for output in 0..layer.output_size {
                    let mut value = biases[output];
                    for input in 0..layer.input_size {
                        value += input_row[input] * weights[input * layer.output_size + output];
                    }
                    if layer_index + 1 != self.layers.len() {
                        value = value.max(0.0);
                    }
                    output_row[output] = value;
                }
            }
            values = outputs;
        }
        Ok(values)
    }

    pub fn all_finite(&self) -> bool {
        self.layers.iter().all(|layer| {
            layer
                .weights
                .iter()
                .chain(&layer.biases)
                .all(|v| v.is_finite())
        })
    }

    #[inline]
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_counts_parameters() {
        let shape = NetworkShape::new(vec![3, 4, 2]).unwrap();
        assert_eq!(shape.weights_per_network(), 20);
        assert_eq!(shape.biases_per_network(), 6);
        assert_eq!(shape.parameters_per_network(), 26);
    }

    #[test]
    fn shape_rejects_a_total_parameter_count_that_overflows() {
        let width = u32::MAX as usize;
        let error = NetworkShape::new(vec![width, width, width]).unwrap_err();

        assert!(error.to_string().contains("overflows usize"));
    }

    #[test]
    fn network_capacity_is_reserved_before_random_initialization() {
        let shape = NetworkShape::new(vec![3, 4, 2]).unwrap();
        let pool = DenseNetworkPool::with_capacity(shape, 8);

        assert!(pool.is_empty());
        for layer in pool.layers() {
            assert!(layer.weights.capacity() >= 8 * layer.input_size() * layer.output_size());
            assert!(layer.biases.capacity() >= 8 * layer.output_size());
        }
    }

    #[test]
    fn random_pool_is_finite_and_reproducible() {
        let shape = NetworkShape::new(vec![3, 4, 2]).unwrap();
        let mut first_rng = Random::new(9);
        let mut second_rng = Random::new(9);
        let first = DenseNetworkPool::random(shape.clone(), 3, &mut first_rng);
        let second = DenseNetworkPool::random(shape, 3, &mut second_rng);

        assert!(first.all_finite());
        for (left, right) in first.layers().iter().zip(second.layers()) {
            assert_eq!(left.weights(), right.weights());
            assert_eq!(left.biases(), right.biases());
        }
    }

    #[test]
    fn patches_only_the_selected_network_and_neurons() {
        let shape = NetworkShape::new(vec![3, 2, 4]).unwrap();
        let mut random = Random::new(4);
        let mut pool = DenseNetworkPool::random(shape, 2, &mut random);
        let untouched_first = pool.layers()[0].network_weights(1).to_vec();
        let untouched_last = pool.layers()[1].network_weights(1).to_vec();

        pool.set_input_connections(0, 1, &[7.0, 8.0]).unwrap();
        pool.set_output_neuron(0, 2, &[9.0, 10.0], 11.0).unwrap();

        assert_eq!(&pool.layers()[0].network_weights(0)[2..4], &[7.0, 8.0]);
        assert_eq!(pool.layers()[1].network_weights(0)[2], 9.0);
        assert_eq!(pool.layers()[1].network_weights(0)[6], 10.0);
        assert_eq!(pool.layers()[1].network_biases(0)[2], 11.0);
        assert_eq!(pool.layers()[0].network_weights(1), untouched_first);
        assert_eq!(pool.layers()[1].network_weights(1), untouched_last);
    }

    #[test]
    fn swap_remove_keeps_layer_major_pool_aligned() {
        let shape = NetworkShape::new(vec![2, 3, 1]).unwrap();
        let mut random = Random::new(5);
        let mut pool = DenseNetworkPool::random(shape, 3, &mut random);
        let last: Vec<_> = pool
            .layers()
            .iter()
            .map(|layer| {
                (
                    layer.network_weights(2).to_vec(),
                    layer.network_biases(2).to_vec(),
                )
            })
            .collect();

        pool.swap_remove(0).unwrap();

        assert_eq!(pool.len(), 2);
        for (layer, (weights, biases)) in pool.layers().iter().zip(last) {
            assert_eq!(layer.network_weights(0), weights);
            assert_eq!(layer.network_biases(0), biases);
        }
    }

    #[test]
    fn ranged_cpu_forward_uses_the_matching_private_networks() {
        let shape = NetworkShape::new(vec![3, 4, 2]).unwrap();
        let mut random = Random::new(17);
        let pool = DenseNetworkPool::random(shape, 5, &mut random);
        let inputs = (0..15).map(|index| index as f32 / 10.0).collect::<Vec<_>>();
        let full = pool.forward_cpu(&inputs).unwrap();

        let ranged = pool.forward_cpu_range(1..4, &inputs[3..12]).unwrap();

        assert_eq!(ranged, full[2..8]);
    }

    #[test]
    fn shrinking_storage_preserves_the_remaining_network() {
        let shape = NetworkShape::new(vec![2, 3, 1]).unwrap();
        let mut random = Random::new(29);
        let mut pool = DenseNetworkPool::random(shape, 8, &mut random);
        for _ in 0..7 {
            pool.swap_remove(0).unwrap();
        }
        let expected = pool
            .layers()
            .iter()
            .map(|layer| {
                (
                    layer.network_weights(0).to_vec(),
                    layer.network_biases(0).to_vec(),
                )
            })
            .collect::<Vec<_>>();

        pool.shrink_to_fit();

        assert_eq!(pool.len(), 1);
        for (layer, (weights, biases)) in pool.layers().iter().zip(expected) {
            assert_eq!(layer.network_weights(0), weights);
            assert_eq!(layer.network_biases(0), biases);
            assert!(layer.weights.capacity() >= layer.weights.len());
            assert!(layer.biases.capacity() >= layer.biases.len());
        }
    }
}
