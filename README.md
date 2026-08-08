# town-hall simulation

This workspace implements the neural architecture for an evolving social/economic
simulation. The economic rules are intentionally placeholders; the stable core is
how entities observe other entities, form first impressions, and choose actions.

## Implemented model

Every entity owns three evolvable parameter sets:

- An action brain with shape `203 -> 30 -> 75 -> 20 -> 401`.
- A minimal ReLU Monomial-NFN reader with observer-specific parameters.
- A multihead get-to-know brain with shape `1010 -> 32 -> 72`.

The 203 action inputs are three provisional world inputs followed by exactly 200
known-person resource inputs. Resources use a fixed `ln(1 + x)` normalization.
The 401 outputs are one `Conserve`, 200 `Predict(slot)`, and 200 `Give(slot)`
logits. Invalid/stale slots are masked. A winning action's amount is its available
resources multiplied by `1 - exp(-winning_logit_margin)`.

`KNOWN_PERSON_COUNT` is a hard architectural constant. Each entity retains the
slot of every person who remains in its current richest-200 set. A newcomer only
replaces an inactive/departing slot, so unrelated learned connections never move.
When fewer than 200 targets exist, dead or displaced occupants may remain as
inactive memory; their inputs are zero and their action logits cannot win.

The Monomial reader consumes the observed entity's complete current action brain.
It follows the HNPS invariant-pooling construction: hidden-neuron permutations
are pooled away, and positive ReLU rescalings are normalized away, while the
semantically fixed input/output coordinates remain visible. All source weights
and biases contribute. Its 1010 values feed one private get-to-know MLP whose
72 outputs are three coordinated semantic heads:

```text
30 input-slot -> first-hidden weights
20 Predict output weights + 1 Predict bias
20 Give output weights    + 1 Give bias
```

There is no input-neuron bias in a dense network, so the input head correctly
generates only connections. The Monomial implementation is isolated behind
`ReaderPool`; adding MAGEP-NFN, ScaleGMN, or another reader later does not alter
the slot or action-brain contracts. The implementation is based on the
[Monomial-NFN paper](https://arxiv.org/abs/2409.11697) and its
[official reference code](https://github.com/MathematicalAI-NUS/Monomial-NFN).

## Tick ordering

One iteration is synchronous:

1. Snapshot wealth and determine the richest candidates with an O(population)
   partial selection plus a small top-201 sort.
2. Reconcile fixed slots and collect replacements.
3. Read every incoming candidate from the unchanged pre-update action-brain pool.
4. Batch each replacement round through all private get-to-know brains.
5. Install every generated connection bundle atomically.
6. Batch all updated action brains, mask invalid targets, and decode actions.
7. Apply actions from one shared snapshot.
8. Consume resources and remove every depleted entity from the aligned pools.
9. Grow world resources and advance the iteration counter.

This prevents population order from leaking into decisions and prevents an entity
updated early in a tick from changing another entity's first impression.

## GPU layout

`neuro-ash` stores equal-shaped private networks layer-major. Each layer is one
contiguous `[entity, input, output]` host array plus `[entity, output]` biases.
The initial population is reserved exactly once across every aligned host pool;
after a population falls to one quarter of its allocation, the pools reclaim
their now-unused capacity together rather than retaining the dead brains.
The Vulkan path uses rank-3 batched matrix multiplication and one operation graph
per chunk. Device capacity is separate from population size: small population
changes reuse padded tensors, major shrink/growth drops old tensors before
replacement allocation, and populations above the configured maximum stream
through a bounded reusable chunk. Full-residency tensors persist and unchanged
layers are not uploaded again. Tensor-ash automatically routes these `M=1`
private-network layers through its warp-sized row kernel instead of mostly-empty
general-GEMM tiles. During first-impression learning, observer chunks
are outermost so each get-to-know parameter chunk remains resident across all of
its replacement rounds.

Get-to-know inference and action inference remain separate because generated
connections must be installed before the action forward. Their device caches are
released at the phase boundary instead of retaining two parameter families that
cannot run concurrently. The current minimal implementation reads Monomial
features and applies sparse connection patches on the CPU between the two GPU
phases; large reader batches are parallelized across CPU cores. This preserves
the same-tick semantics and leaves a clear future seam for a GPU reader/scatter
kernel.

The sibling `tensor-ash` checkout contains the small required extension for
per-batch fused biases (`[B, N]` as well as shared `[N]`). The workspace uses it
through `../tensor-ash` while that change is local.

## Scope and scaling

New entities receive completely random action, reader, and get-to-know parameters.
Mutation, inheritance, reproduction, death rules, and selection pressure are not
invented yet because those world dynamics have not been designed. Mutable
parameter seams and aligned swap-removal are in place for that work.

`Give` currently performs a staged transfer, `Predict` is a no-op, and `Conserve`
does nothing. After actions, every entity consumes up to 25 resources using
saturating subtraction; entities at zero resources then die and are swap-removed
from every aligned pool. These are deliberately disposable adapters around the
implemented neural flow.

With current widths, each entity owns 18,386 action-brain floats, 34,728
get-to-know floats, and 16 Monomial-reader floats: about 212 KB of parameters.
Bounded GPU chunks prevent device allocation from growing with an arbitrarily
large population, but host storage still costs about 21.2 GB per 100,000 entities
and 212 GB per million before simulation state and temporary vectors. Reaching
that scale therefore still requires a later model decision such as reduced
precision, smaller/structured heads, parameter sharing, compression, or
out-of-core storage.

## Run and verify

The Nix launcher supplies Rust, shader compilation, the Vulkan loader, and the
host graphics-driver path. Its optional argument is the iteration count:

```bash
./run.sh 1
```

The optional GPU batch bound trades throughput for lower device memory:

```bash
TOWN_HALL_GPU_BATCH_SIZE=10000 ./run.sh 1
```

CPU tests and linting do not require Vulkan:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The ignored GPU parity test requires the Nix environment and a Vulkan device:

```bash
nix develop .#default --command \
  cargo test --release -p neuro-ash --test gpu_batch -- --ignored --test-threads=1
```
