#!/usr/bin/env bash
# Publish workspace crates to crates.io in dependency order.
set -euo pipefail
cd "$(dirname "$0")"

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]] && [[ ! -f "${CARGO_HOME:-$HOME/.cargo}/credentials.toml" ]]; then
  echo "No crates.io token. Create one at https://crates.io/settings/tokens"
  echo "then:  cargo login"
  echo "or:    export CARGO_REGISTRY_TOKEN=..."
  exit 1
fi

crates=(
  core-invoice
  core-invoice-formats
  core-invoice-cli
  core-invoice-fixtures
  core-invoice-sys
)

for crate in "${crates[@]}"; do
  echo "=== publishing $crate ==="
  cargo publish -p "$crate" --allow-dirty
  echo "waiting for crates.io index..."
  for i in 1 2 3 4 5 6 7 8 9 10; do
    if cargo info "$crate" >/dev/null 2>&1; then
      break
    fi
    sleep 3
  done
done

echo "done."
echo "  cargo add core-invoice"
echo "  cargo install core-invoice-cli"
