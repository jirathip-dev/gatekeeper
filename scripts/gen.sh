#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

names() {
    for f in "$1"/*.rs; do
        n=$(basename "$f" .rs)
        [ "$n" = mod ] || echo "$n"
    done
}

module() {
    dir=$1 use=$2 sig=$3 list=$4 arm=$5 none=$6
    {
        for n in $(names "$dir"); do echo "pub mod $n;"; done
        printf '\n%s\n\npub const %s: &[&str] = &[' "$use" "$list"
        for n in $(names "$dir"); do printf '"%s", ' "$(echo "$n" | tr _ -)"; done
        printf '];\n\n%s {\n    match name {\n' "$sig"
        for n in $(names "$dir"); do
            printf "        \"%s\" => $arm,\n" "$(echo "$n" | tr _ -)" "$n"
        done
        printf '        _ => %s,\n    }\n}\n' "$none"
    } > "$dir/mod.rs"
    rustfmt --edition 2024 "$dir/mod.rs"
}

module crates/gatekeeper-engine/src/check \
    'use crate::run::CheckCx;
use crate::{Error, Result};' \
    'pub fn run(name: &str, cx: &CheckCx) -> Result<Vec<String>>' NAMES '%s::run(cx)' \
    'Err(Error(format!("no built-in check `{name}`")))'

module crates/gatekeeper/src/cmd \
    'use std::process::ExitCode;' \
    'pub fn run(name: &str, args: &[String]) -> Option<ExitCode>' VERBS 'Some(%s::run(args))' None

./.gatekeeper/run skill --to skill
