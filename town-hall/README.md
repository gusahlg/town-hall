# town-hall

`town-hall` owns entities, stable IDs, wealth ranking, fixed known-person slots,
Monomial-NFN readers, connection generation, action decoding, and synchronous
world updates.

The key invariant is that population index aligns the entity row, known-person
row, action brain, Monomial reader, and get-to-know brain. Birth pushes one random
row to every pool; death swap-removes the same row everywhere and repairs the
stable-ID lookup.

`Village::step` performs ranking, slot replacement, whole-brain reading,
get-to-know inference, atomic neural patches, action inference, and staged world
application. It accepts any `neuro_ash::BatchEvaluator`, enabling deterministic
CPU tests and the production Vulkan path.

Action economics are explicitly provisional. See the
[workspace README](../README.md) for the complete implemented contract and the
remaining scaling constraints.
