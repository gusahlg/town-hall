# neuro-ash

`neuro-ash` stores many equal-shaped private dense networks as layer-major pools
and evaluates them with true Vulkan batched matrix multiplication.

For a device batch `B`, a layer uses weights `[B, I, O]`, biases `[B, O]`, and
activations `[B, 1, I]`. Parameters are stored as f16 on the host and uploaded
bit-for-bit into f16 weight tensors; device biases and activations remain f32
because every dot product accumulates in f32 on both backends. Hidden ReLU and
per-network bias are fused into each matrix multiplication, and all layers in
one forward are submitted as one tensor-ash operation graph. Device capacity is independent of live population:
small population changes reuse padded tensors, major resizes drop old tensors
before allocating replacements, and populations above the configured bound are
streamed through reusable chunks. Full-residency pools synchronize only when a
host revision changes.

`DenseNetworkPool` also provides aligned random insertion, swap-removal,
evolution-oriented mutable parameter views, targeted input/output-neuron patches,
up-front capacity reservation, explicit capacity reclamation, and a deterministic
CPU evaluator. `BatchEvaluator` lets `town-hall` use either the CPU reference or
`Gpu` without changing simulation ordering.

The default maximum GPU batch is 50,000 networks. `town-hall` exposes it through
`TOWN_HALL_GPU_BATCH_SIZE`; lowering it reduces peak device memory at the cost of
more parameter uploads and dispatches.

See the [workspace README](../README.md) for the complete simulation architecture.
