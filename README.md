# Town Hall simulation

Town Hall combines a generative economic domain model with an evolving
neural/social decision architecture.

## Economic foundation

The 256 primitive resources are flat `u8` IDs that each compactly encode one
point of an N-dimensional property lattice; coordinates and normalized geometric
similarity are derived from the ID rather than stored. Goods are exact ordered
resource sequences stored as run-length encoded `(ResourceId, Quantity)` pairs
in an append-only registry. Adjacent identical runs merge before registration:
`[(A, 3), (A, 2)]` and `[(A, 5)]` share one `GoodId`, while separated runs in
`[(A, 3), (B, 1), (A, 2)]` retain their positions. Configurations must be non-empty
and all quantities positive. A merged run exceeding `u32::MAX` panics, including
in release builds; normalization never allocates the expanded sequence.

Inventories contain stable `GoodId`s and counts of separate objects. Fifty units
of `[(A, 1)]` and one unit of `[(A, 50)]` remain different holdings; combining or
splitting goods requires a recipe and a feasible process.

Needs have non-empty ordered targets of ideal property points and positive
quantities. Targets are private, accessible through `Need::target()`, and fixed
for an entity's lifetime; intensity is separate mutable state.

Recipes use canonical unordered input multisets, and registration enforces
flattened primitive-material conservation. Processes record workers, time, exact equipment,
supporting consumables, and extra inputs lost to inefficiency.

The world is a seeded tile grid containing primitive deposits, and every entity
has one location. Similarity, consumption, extraction, production, labor, transport,
prices, innovation, feasibility, and bootstrapping remain deliberately unchosen.

## World generation

`World::default()` generates a 150 × 150 map. `World::with_seed(width, height,
seed)` reproduces a map without consuming the population's random stream;
`Village::with_population(population, seed)` uses that seed for its world too.
For custom density, keep and report the generator settings alongside the map
dimensions when recording an experiment:

```rust
use town_hall::{Generator, RandomGenerator};

let generator = RandomGenerator { seed: 42, deposit_chance: 35 };
let world = generator.generate(150, 150);
println!("150 × 150, {generator:?}: {} primitive units", world.total_deposited_material());
```

Each tile independently has a 35% chance of holding 1–3 deposits. Each deposit
contains 1–3 distinct primitive resources, sampled across all 256 IDs, with
100–10,000 units each. Property-space similarity does not determine geography,
and small maps need not contain every resource. Recovery is per resource: 75%
have rate zero, and 25% have a nominal rate of 1–10 primitive units per tick.

Extraction properties belong to the whole deposit: time 1–16, parallelisation
1–8, gear 0–4, efficiency 25–100%, and batch size 1–16. These are baseline
parameters for future extraction rules. The first spawn tile, `(0, 0)`, always
has a deposit with no gear requirement, time and batch size of one, and 100%
efficiency, even at zero density. This guarantees starting material, while
economic viability still depends on extraction and production being implemented.
Generation does not create owned goods, execute extraction, or apply recovery.
`World::empty(width, height)` remains available for hand-built scenarios.

The native dashboard displays generated material. The browser's local preview
does not run the native generator and currently shows no deposits.

## Neural/social architecture

Each entity owns an action brain (`203 -> 30 -> 75 -> 20 -> 401`), a Monomial-NFN
reader, and a get-to-know brain (`1010 -> 32 -> 72`). The current action names—one
`Conserve`, 200 `Predict`, and 200 `Give` outputs—are temporary; the aligned
private-network, fixed-slot, reader, and generated-connection design is settled.

The 203 inputs currently contain normalized own money, inventory units, world
deposits, and 200 known-person money slots. Each entity retains stable slots for
its current richest 200 other entities. A reader observes a candidate's complete
action brain, and its get-to-know network generates the three connection heads
needed to install that candidate into one slot.

A tick synchronously ranks candidates, learns replacements from the unchanged
brain pool, installs every connection bundle atomically, evaluates actions, and
applies the temporary action adapter from one money snapshot. `Give` transfers
money; `Predict` and `Conserve` currently have no world effect. There are no fake
consumption, production, income, or death rules.

`neuro-ash` evaluates aligned private networks on CPU or Vulkan and streams large
populations through bounded GPU batches. Parameters are stored as f16 and every
forward accumulates in f32 on both backends. Current networks cost about 106 KB
per entity, so very large populations will require a later representation
decision.

## Workspace

- `crates/town-hall` is the pure simulation library.
- `crates/neuro-ash` contains its CPU/Vulkan neural evaluators.
- `crates/town-hall-gui` is the shared eframe dashboard.
- `apps/town-hall-client` launches the native app and builds the web app.
- `apps/town-hall-cli` is the minimal batch-oriented command-line client.

The native dashboard runs the real `Village` on a worker thread and prefers the
Vulkan evaluator, falling back to CPU when Vulkan is unavailable. The browser
uses the same GUI against a deterministic local session with the same starting
population, money, and goods: the simulation core remains native-only because
its unchanged neural stack depends on Vulkan.

## Run and verify

```bash
./run.sh 1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Launch the native dashboard with:

```bash
./apps/town-hall-client/run.sh
```

Run the CLI with its optional iteration count:

```bash
./apps/town-hall-cli/run.sh 100
```

Build and serve the WebAssembly client (the port defaults to `8080`):

```bash
./apps/town-hall-client/web/run.sh
./apps/town-hall-client/web/run.sh 3000
```
