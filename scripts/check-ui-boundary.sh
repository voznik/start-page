#!/usr/bin/env bash
# CLAUDE.md invariant 2: sp-ui must not depend on crossterm, ratzilla, tokio,
# or any provider crate. Fails (exit 1) if any of those show up in its tree.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

forbidden='crossterm|ratzilla|tokio'
tree="$(cargo tree -p sp-ui)"

if echo "$tree" | grep -qE "$forbidden"; then
    echo "sp-ui boundary violated: found forbidden dependency in tree:" >&2
    echo "$tree" | grep -E "$forbidden" >&2
    exit 1
fi

echo "sp-ui boundary OK: no crossterm/ratzilla/tokio in dependency tree."
