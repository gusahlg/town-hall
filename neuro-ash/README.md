# neuro-ash

`neuro-ash` is town-hall's GPU policy component. In the shared-policy design,
one persistent MLP evaluates a dense batch containing one row per entity.
Entities carry small behavior profiles rather than complete private networks,
allowing tensor-ash to execute useful matrix sizes while population changes do
not require rebuilding a separate model for every entity.

The crate owns the Vulkan context, model tensors, reusable activation buffers,
batched uploads and downloads, and the tensor-ash operation graph. See the
[workspace README](../README.md) for the complete architecture direction.
