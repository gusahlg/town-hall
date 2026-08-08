//! Batched neural-network storage and execution for the simulation.
//!
//! Every entity owns its parameters, but equal-shaped networks are stored
//! layer-by-layer so one rank-3 matrix multiplication evaluates the whole
//! population on the GPU.

mod dense;
mod gpu;

pub use dense::{DenseLayerPool, DenseNetworkPool, NetworkShape, Random};
pub use gpu::{BatchEvaluator, CpuEvaluator, DEFAULT_MAX_GPU_BATCH_SIZE, Gpu};
