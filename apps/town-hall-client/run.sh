#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
temporary_flake=""

if [[ "${TOWN_HALL_DEV_SHELL:-}" == "1" ]]; then
    exec cargo run --manifest-path "${project_root}/Cargo.toml" \
        --release --package town-hall-client -- "$@"
fi

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

# eframe opens the native window; Vulkan and window-system libraries come from
# the repository development shell.
nix develop "${flake_ref}" --command \
    cargo run --manifest-path "${project_root}/Cargo.toml" \
    --release --package town-hall-client -- "$@"
