#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
web_root="${project_root}/apps/town-hall-client/web"
port="${1:-${TOWN_HALL_WEB_PORT:-8080}}"
temporary_flake=""

if [[ ! "${port}" =~ ^[0-9]+$ ]] || ((port < 1 || port > 65535)); then
    echo "usage: $0 [port]" >&2
    echo "port must be an integer between 1 and 65535" >&2
    exit 2
fi

if (($# > 1)); then
    echo "usage: $0 [port]" >&2
    exit 2
fi

if [[ "${TOWN_HALL_DEV_SHELL:-}" == "1" ]]; then
    wasm-pack build "${project_root}/apps/town-hall-client" \
        --target web --out-dir web/pkg --release
    echo "Town Hall web client: http://127.0.0.1:${port}"
    exec python3 -m http.server --bind 127.0.0.1 \
        --directory "${web_root}" "${port}"
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

# Build the browser package first, then keep a static server in the foreground
# so Ctrl-C cleanly stops the client.
nix develop "${flake_ref}" --command bash -c '
    set -euo pipefail
    project_root="$1"
    web_root="$2"
    port="$3"

    wasm-pack build "${project_root}/apps/town-hall-client" \
        --target web --out-dir web/pkg --release
    echo "Town Hall web client: http://127.0.0.1:${port}"
    exec python3 -m http.server --bind 127.0.0.1 \
        --directory "${web_root}" "${port}"
' town-hall-web "${project_root}" "${web_root}" "${port}"
