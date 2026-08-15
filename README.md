# Town Hall simulation

Town Hall combines a generative economic domain model with an evolving
neural/social decision architecture.

## Economic foundation

The 256 primitive resources are flat `u8` IDs that each compactly encode one
point of an N-dimensional property lattice; coordinates and normalized geometric
similarity are derived from the ID rather than stored. Goods are canonical
ordered resource-and-quantity configurations stored once in an append-only
registry. Inventories contain only stable `GoodId`s and quantities.

Needs are ordered configurations of ideal property points and quantities. Recipes
use canonical unordered input multisets and registration enforces flattened
primitive-material conservation. Processes record workers, time, exact equipment,
supporting consumables, and extra inputs lost to inefficiency.

The world is a tile grid containing primitive deposits, and every entity has one
location. Similarity, consumption, extraction, production, labor, transport,
prices, innovation, feasibility, and bootstrapping remain deliberately unchosen.

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
