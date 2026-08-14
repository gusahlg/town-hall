#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
temporary_flake=""

# A Git-backed flake naturally excludes Cargo's untracked target directory.
# During initial setup the new flake files may not be tracked yet; using the
# whole repository as a path flake would copy target/ into the Nix store. In
# that case, evaluate the same two files from a tiny temporary flake directory.
if command -v git >/dev/null 2>&1 \
    && git -C "${project_root}" ls-files --error-unmatch \
        flake.nix flake.lock >/dev/null 2>&1; then
    flake_ref="${project_root}#default"
else
    temporary_flake="$(mktemp -d -t town-hall-flake.XXXXXXXX)"
    trap 'rm -rf -- "${temporary_flake}"' EXIT
    cp -- "${project_root}/flake.nix" "${project_root}/flake.lock" \
        "${temporary_flake}/"
    flake_ref="path:${temporary_flake}#default"
fi

# nix develop supplies Rust, glslc, the Vulkan loader, validation layers, and
# the runtime path to the NixOS GPU driver. Arguments are forwarded to the
# town-hall CLI after Cargo's `--` separator.
nix develop "${flake_ref}" --command \
    cargo run --manifest-path "${project_root}/Cargo.toml" \
    --release --package town-hall-cli -- "$@"
