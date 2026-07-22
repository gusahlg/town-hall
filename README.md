# town-hall

`town-hall` is a deliberately small economics simulation with a GPU-driven
behavior policy. Entities see the same world snapshot, make decisions, and only
then change the world, so iteration order does not leak into their observations.

## Architecture direction

The workspace is still early in its implementation. The design below is the
shared-policy architecture that both crates are converging on; not every
described API is implemented yet.

The project separates simulation mechanics from numeric policy evaluation:

```text
Village state
    -> normalized observation + behavior profile per entity
    -> dense row-major policy matrix
    -> tensor-ash two-layer MLP on Vulkan
    -> three action scores per entity
    -> staged economy update
```

- `town-hall` owns stable entity IDs, dense entity data, population changes,
  economic rules, statistics, and renderer-neutral entity views.
- `neuro-ash` owns the Vulkan context, persistent model tensors, reusable
  activation buffers, population chunking, uploads, the tensor-ash operation
  graph, and downloads.

`Village::step` accepts the narrow `Policy` contract rather than a concrete GPU
type. The tensor-ash engine is the production implementation, while deterministic
CPU policies can drive tests and experiments without creating a Vulkan device.

Population size is the number of matrix rows, not the neural input width. GPU
buffers grow in power-of-two buckets and shrink only after a 4x drop, preventing
birth/death bursts from causing an allocation on every tick. Extremely large
populations are divided into bounded chunks.

## Why entities do not own complete MLPs

A private 3 -> 32 -> 3 MLP for every entity would turn inference into a batch of
tiny `M=1` matrix multiplications. That wastes tensor-ash's tiled GPU kernels,
uses substantial parameter memory, and makes births/deaths require GPU weight
updates.

Instead, the engine runs one shared MLP over a large matrix and gives every
entity a small persistent behavior profile. Policy rows include both normalized
observations and `observation * sensitivity` interaction features, so profiles
change how entities react to the same world signal. Profiles can later mutate,
be inherited, be trained, or be displayed without changing GPU model shapes.
`spawn_with_behavior` and `set_behavior` are the core hooks for those workflows.

## Visualization seam

`Village::entities()` yields allocation-free `EntityView` values with stable
IDs, resources, behavior, and the last action. A renderer can read these views
directly or collect/sample them for an asynchronous frame. Rendering should run
at its own frequency rather than forcing a full population copy every simulation
tick.

## Running

`tensor-ash` remains a separate project. Cargo fetches the Git revision pinned
in the workspace manifest and lockfile, so a sibling checkout is not required.

On NixOS, the launcher enters the project flake, supplies Rust, `glslc`, the
Vulkan loader and validation layers, exposes the NixOS graphics-driver path, and
runs the optimized simulation:

```bash
./run.sh
```

Arguments given to the script are forwarded to the simulation binary. To work
inside the same environment interactively instead:

```bash
nix develop
cargo run --release -p town-hall
```

The first invocation downloads the pinned Nix dependencies and performs an
optimized Rust/shader build. Nix supplies the Vulkan loader and tools; the NixOS
host must still have its hardware driver enabled through `hardware.graphics`.

CPU-only unit tests do not require a Vulkan runtime:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
