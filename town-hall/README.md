# town-hall

`town-hall` owns the economic state and treats `neuro-ash` as a batched policy
service.

Entities live in dense structure-of-arrays storage so observation construction
is a sequential memory scan. Public `EntityId` values remain stable; removing an
entity fills its dense slot with the last entity and repairs one ID lookup.

Every tick has three phases:

1. Build normalized observations for every entity from one world snapshot.
2. Ask `PolicyEngine` for one action-score row per entity.
3. Interpret all scores and apply investments/work in staged passes.

Births and deaths happen between complete ticks, so a GPU output row always maps
back to the same dense entity row used to create its input. Since behavior
profiles are entity data rather than GPU weight batches, reordering entities
does not require moving model tensors.

`Village::step` consumes the `neuro_ash::Policy` contract. `PolicyEngine`
implements it with tensor-ash, and lightweight CPU policies can implement it for
tests or controlled scenarios. Explicit behavior profiles can be inserted with
`spawn_with_behavior` or changed with `set_behavior`.

`Village::entities()` is the visualization boundary. It exposes stable IDs and
read-only state without coupling the simulation to a windowing or rendering
library.

These APIs describe the shared-policy architecture the crate is converging on;
not all of them are implemented yet. See the [workspace README](../README.md)
for the complete architecture direction and running instructions.
