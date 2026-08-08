use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use anyhow::{Result, ensure};
use tensor_ash::{
    Activation, Epilogue, EpilogueBinary, Executor, MatmulCall, MatmulOp, MatmulPipeline, Tensor,
    VulkanContext,
};

use crate::DenseNetworkPool;

/// Bounds persistent device memory independently of simulation population.
/// Larger populations are evaluated as consecutive chunks through the same
/// tensors. Keeping this at 25,000 preserves full-residency for the current
/// model on an 8 GiB GPU while giving larger runs a non-OOM execution path.
pub const DEFAULT_MAX_GPU_BATCH_SIZE: usize = 25_000;
const CACHE_SHRINK_FACTOR: usize = 4;

pub trait BatchEvaluator {
    /// Evaluate one input row with each private network in `pool`.
    fn evaluate(&mut self, pool: &DenseNetworkPool, inputs: &[f32]) -> Result<Vec<f32>>;

    /// Evaluate a contiguous subset while preserving each input row's mapping
    /// to its private network. Stateful evaluators can keep that subset
    /// resident across repeated calls.
    fn evaluate_range(
        &mut self,
        pool: &DenseNetworkPool,
        networks: Range<usize>,
        inputs: &[f32],
    ) -> Result<Vec<f32>> {
        ensure!(
            networks.start == 0 && networks.end == pool.len(),
            "this evaluator does not support partial network ranges"
        );
        self.evaluate(pool, inputs)
    }

    /// Preferred upper bound for a contiguous private-network batch.
    fn max_batch_size(&self) -> usize {
        usize::MAX
    }

    /// Release evaluator-specific cached state for a pool. The host-side pool
    /// remains valid. CPU and other stateless evaluators need no implementation.
    fn release(&mut self, _pool: &DenseNetworkPool) {}
}

#[derive(Debug, Default)]
pub struct CpuEvaluator;

impl BatchEvaluator for CpuEvaluator {
    fn evaluate(&mut self, pool: &DenseNetworkPool, inputs: &[f32]) -> Result<Vec<f32>> {
        pool.forward_cpu(inputs)
    }

    fn evaluate_range(
        &mut self,
        pool: &DenseNetworkPool,
        networks: Range<usize>,
        inputs: &[f32],
    ) -> Result<Vec<f32>> {
        pool.forward_cpu_range(networks, inputs)
    }
}

struct DeviceLayerPool {
    weights: Tensor,
    biases: Tensor,
    host_revision: Option<u64>,
}

struct DeviceNetworkPool {
    shape: Box<[usize]>,
    capacity: usize,
    layers: Vec<DeviceLayerPool>,
    activations: Vec<Tensor>,
    loaded_range: Option<Range<usize>>,
}

impl DeviceNetworkPool {
    fn new(context: &Arc<VulkanContext>, host: &DenseNetworkPool, capacity: usize) -> Result<Self> {
        ensure!(capacity > 0, "cannot allocate an empty GPU network pool");
        let batch = u32::try_from(capacity)?;
        let mut layers = Vec::with_capacity(host.shape().layer_count());
        for layer in host.layers() {
            layers.push(DeviceLayerPool {
                weights: Tensor::uninit_device(
                    context,
                    &[
                        batch,
                        u32::try_from(layer.input_size())?,
                        u32::try_from(layer.output_size())?,
                    ],
                )?,
                biases: Tensor::uninit_device(
                    context,
                    &[batch, u32::try_from(layer.output_size())?],
                )?,
                host_revision: None,
            });
        }
        let activations = host
            .shape()
            .widths()
            .iter()
            .map(|&width| Tensor::uninit_device(context, &[batch, 1, u32::try_from(width)?]))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            shape: host.shape().widths().into(),
            capacity,
            layers,
            activations,
            loaded_range: None,
        })
    }

    fn has_shape(&self, host: &DenseNetworkPool) -> bool {
        self.shape.as_ref() == host.shape().widths()
    }

    fn needs_rebuild(&self, host: &DenseNetworkPool, max_batch_size: usize) -> bool {
        if !self.has_shape(host) {
            return true;
        }
        let required = host.len().min(max_batch_size);
        self.capacity < required
            || (host.len() <= max_batch_size && host.len() <= self.capacity / CACHE_SHRINK_FACTOR)
    }

    fn sync_range(
        &mut self,
        executor: &Executor,
        host: &DenseNetworkPool,
        networks: Range<usize>,
    ) -> Result<()> {
        ensure!(
            networks.start < networks.end && networks.end <= host.len(),
            "invalid host network range"
        );
        ensure!(
            networks.len() <= self.capacity,
            "host network range exceeds device capacity"
        );
        let same_range = self.loaded_range.as_ref() == Some(&networks);
        for (device, layer) in self.layers.iter_mut().zip(host.layers()) {
            if same_range && device.host_revision == Some(layer.revision()) {
                continue;
            }
            let weight_stride = layer.input_size() * layer.output_size();
            executor.upload(
                &layer.weights()[networks.start * weight_stride..networks.end * weight_stride],
                &device.weights,
            )?;
            executor.upload(
                &layer.biases()
                    [networks.start * layer.output_size()..networks.end * layer.output_size()],
                &device.biases,
            )?;
            device.host_revision = Some(layer.revision());
        }
        self.loaded_range = Some(networks);
        Ok(())
    }

    fn evaluate_into(&self, executor: &Executor, inputs: &[f32], output: &mut [f32]) -> Result<()> {
        let input_size = self.shape[0];
        let output_size = *self.shape.last().expect("validated network shape");
        ensure!(
            inputs.len().is_multiple_of(input_size),
            "GPU input chunk contains a partial row"
        );
        let active_networks = inputs.len() / input_size;
        ensure!(
            active_networks > 0 && active_networks <= self.capacity,
            "GPU input chunk has an invalid network count"
        );
        ensure!(
            output.len() == active_networks * output_size,
            "GPU output chunk has the wrong size"
        );
        executor.upload(inputs, &self.activations[0])?;
        let mut operations = Vec::with_capacity(self.layers.len());
        for (layer_index, layer) in self.layers.iter().enumerate() {
            operations.push(MatmulOp::with_epilogue(
                MatmulCall {
                    a: &self.activations[layer_index],
                    b: &layer.weights,
                    c: &self.activations[layer_index + 1],
                    alpha: 1.0,
                    accumulate: false,
                },
                Epilogue {
                    bias: Some(&layer.biases),
                    activation: if layer_index + 1 == self.layers.len() {
                        Activation::None
                    } else {
                        Activation::Relu
                    },
                    binary: EpilogueBinary::None,
                },
            ));
        }
        executor.run_op_graph(&operations)?;
        executor.download(
            self.activations.last().expect("network has an output"),
            output,
        )?;
        Ok(())
    }
}

pub struct Gpu {
    context: Arc<VulkanContext>,
    executor: Executor,
    pools: HashMap<u64, DeviceNetworkPool>,
    max_batch_size: usize,
}

impl Gpu {
    pub fn new() -> Result<Self> {
        Self::with_max_batch_size(DEFAULT_MAX_GPU_BATCH_SIZE)
    }

    pub fn with_max_batch_size(max_batch_size: usize) -> Result<Self> {
        ensure!(
            max_batch_size > 0,
            "GPU batch size must be greater than zero"
        );
        let context = VulkanContext::new(false)?;
        let pipeline = Arc::new(MatmulPipeline::new(&context)?);
        let executor = Executor::new(context.clone(), pipeline, 2, 64)?;
        Ok(Self {
            context,
            executor,
            pools: HashMap::new(),
            max_batch_size,
        })
    }

    pub fn device_name(&self) -> &str {
        self.context.device_name()
    }

    pub fn max_batch_size(&self) -> usize {
        self.max_batch_size
    }

    pub fn clear_cache(&mut self) {
        self.pools.clear();
    }

    fn ensure_device_pool(&mut self, pool: &DenseNetworkPool) -> Result<()> {
        let pool_id = pool.id();
        let rebuild = self
            .pools
            .get(&pool_id)
            .is_none_or(|device| device.needs_rebuild(pool, self.max_batch_size));
        if rebuild {
            let previous = self.pools.remove(&pool_id);
            let required = pool.len().min(self.max_batch_size);
            let capacity = previous
                .as_ref()
                .filter(|device| device.has_shape(pool) && device.capacity < required)
                .map_or(required, |device| {
                    device
                        .capacity
                        .saturating_mul(2)
                        .max(required)
                        .min(self.max_batch_size)
                });
            // Vulkan buffers free their memory on drop. Do this before the new
            // allocation so a population resize never needs old + new pools at
            // the same time.
            drop(previous);
            let device = DeviceNetworkPool::new(&self.context, pool, capacity)?;
            self.pools.insert(pool_id, device);
        }
        Ok(())
    }

    fn evaluate_range_into(
        &mut self,
        pool: &DenseNetworkPool,
        networks: Range<usize>,
        inputs: &[f32],
        output: &mut [f32],
    ) -> Result<()> {
        self.ensure_device_pool(pool)?;
        let device = self
            .pools
            .get_mut(&pool.id())
            .expect("device pool inserted");
        device.sync_range(&self.executor, pool, networks)?;
        device.evaluate_into(&self.executor, inputs, output)
    }
}

impl BatchEvaluator for Gpu {
    fn evaluate(&mut self, pool: &DenseNetworkPool, inputs: &[f32]) -> Result<Vec<f32>> {
        ensure!(
            inputs.len() == pool.len() * pool.shape().input_size(),
            "received {} inputs, expected {}",
            inputs.len(),
            pool.len() * pool.shape().input_size()
        );
        if pool.is_empty() {
            self.pools.remove(&pool.id());
            return Ok(Vec::new());
        }

        let output_size = pool.shape().output_size();
        let mut output = vec![0.0; pool.len() * output_size];
        let input_size = pool.shape().input_size();
        for start in (0..pool.len()).step_by(self.max_batch_size) {
            let end = start.saturating_add(self.max_batch_size).min(pool.len());
            self.evaluate_range_into(
                pool,
                start..end,
                &inputs[start * input_size..end * input_size],
                &mut output[start * output_size..end * output_size],
            )?;
        }
        Ok(output)
    }

    fn evaluate_range(
        &mut self,
        pool: &DenseNetworkPool,
        networks: Range<usize>,
        inputs: &[f32],
    ) -> Result<Vec<f32>> {
        ensure!(
            networks.start <= networks.end && networks.end <= pool.len(),
            "invalid GPU network range"
        );
        let network_count = networks.len();
        let input_size = pool.shape().input_size();
        ensure!(
            inputs.len() == network_count * input_size,
            "received {} inputs, expected {}",
            inputs.len(),
            network_count * input_size
        );
        if network_count == 0 {
            return Ok(Vec::new());
        }

        let output_size = pool.shape().output_size();
        let mut output = vec![0.0; network_count * output_size];
        for local_start in (0..network_count).step_by(self.max_batch_size) {
            let local_end = local_start
                .saturating_add(self.max_batch_size)
                .min(network_count);
            let start = networks.start + local_start;
            let end = networks.start + local_end;
            self.evaluate_range_into(
                pool,
                start..end,
                &inputs[local_start * input_size..local_end * input_size],
                &mut output[local_start * output_size..local_end * output_size],
            )?;
        }
        Ok(output)
    }

    fn max_batch_size(&self) -> usize {
        self.max_batch_size
    }

    fn release(&mut self, pool: &DenseNetworkPool) {
        self.pools.remove(&pool.id());
    }
}
