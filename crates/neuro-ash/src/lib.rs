//! Batched neural-network storage and execution for the simulation.
//!
//! Every entity owns its parameters, but equal-shaped networks are stored
//! layer-by-layer so one rank-3 matrix multiplication evaluates the whole
//! population on the GPU. Parameters are stored as f16 on the host and for
//! GPU weights; every dot product accumulates in f32 on both backends.

mod dense;
mod gpu;

pub use dense::{DenseLayerPool, DenseNetworkPool, NetworkShape, Random};
pub use gpu::{BatchEvaluator, CpuEvaluator, DEFAULT_MAX_GPU_BATCH_SIZE, Gpu};
/// The parameter storage type shared with pool consumers.
pub use half::f16;
