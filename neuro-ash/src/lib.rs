use std::sync::Arc;

use anyhow::Result;
use tensor_ash::{
    Activation, Epilogue, EpilogueBinary, Executor, MatmulCall, MatmulOp, MatmulPipeline, Tensor,
    VulkanContext,
};

#[derive(Debug, Clone, PartialEq)]
pub struct NeuralNetwork {
    pub layer_sizes: Vec<usize>,
    pub weights: Vec<Vec<f32>>,
    pub biases: Vec<Vec<f32>>,
}

impl NeuralNetwork {
    // Currently completely randomized brain
    pub fn new(layer_sizes: &[usize], mut seed: u64) -> Self {
        assert!(layer_sizes.len() >= 2);
        assert!(layer_sizes.iter().all(|&size| size > 0));

        let weights = layer_sizes
            .windows(2)
            .map(|sizes| {
                (0..sizes[0] * sizes[1])
                    .map(|_| random(&mut seed) * 0.1)
                    .collect()
            })
            .collect();
        let biases = layer_sizes[1..]
            .iter()
            .map(|&size| (0..size).map(|_| random(&mut seed) * 0.1).collect())
            .collect();

        Self {
            layer_sizes: layer_sizes.to_vec(),
            weights,
            biases,
        }
    }
}

pub struct Gpu {
    context: Arc<VulkanContext>,
    executor: Executor,
}

impl Gpu {
    pub fn new() -> Result<Self> {
        let context = VulkanContext::new(false)?;
        let pipeline = Arc::new(MatmulPipeline::new(&context)?);
        let executor = Executor::new(context.clone(), pipeline, 2, 8)?;
        Ok(Self { context, executor })
    }

    pub fn device_name(&self) -> &str {
        self.context.device_name()
    }

    pub fn forward(&self, network: &NeuralNetwork, input: &[f32]) -> Result<Vec<f32>> {
        assert_eq!(input.len(), network.layer_sizes[0]);

        let mut values = Tensor::uninit_device(&self.context, &[1, input.len() as u32])?;
        self.executor.upload(input, &values)?;

        for layer in 0..network.weights.len() {
            let input_size = network.layer_sizes[layer] as u32;
            let output_size = network.layer_sizes[layer + 1] as u32;
            let weights = Tensor::uninit_device(&self.context, &[input_size, output_size])?;
            let biases = Tensor::uninit_device(&self.context, &[output_size])?;
            let output = Tensor::uninit_device(&self.context, &[1, output_size])?;
            self.executor.upload(&network.weights[layer], &weights)?;
            self.executor.upload(&network.biases[layer], &biases)?;
            self.executor.run_ops(&[MatmulOp::with_epilogue(
                MatmulCall {
                    a: &values,
                    b: &weights,
                    c: &output,
                    alpha: 1.0,
                    accumulate: false,
                },
                Epilogue {
                    bias: Some(&biases),
                    activation: if layer + 1 == network.weights.len() {
                        Activation::None
                    } else {
                        Activation::Relu
                    },
                    binary: EpilogueBinary::None,
                },
            )])?;
            values = output;
        }

        let mut output = vec![0.0; *network.layer_sizes.last().unwrap()];
        self.executor.download(&values, &mut output)?;
        Ok(output)
    }
}

fn random(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    ((*state >> 40) as f32 / (1_u32 << 24) as f32) * 2.0 - 1.0
}
