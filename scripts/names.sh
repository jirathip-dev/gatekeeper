#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
for f in crates/gatekeeper/src/cmd/*.rs; do n=$(basename "$f" .rs); [ "$n" = mod ] || printf 'verb\t%s\n' "$n"; done
for f in packs/*/gates/*.toml; do p=${f#packs/}; printf '%s\t%s\n' "${p%%/*}" "$(basename "$f" .toml)"; done
