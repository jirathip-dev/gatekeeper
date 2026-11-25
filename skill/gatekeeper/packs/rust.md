# The rust pack

fmt, clippy with the shared lints, cargo-deny, the banned-API fixtures

## rust/bans

the 12 clippy denies, unsafe_code forbid and pedantic warn are in [workspace.lints] and in any crate's own [lints]; clippy refuses each banned-API file under them

1. A crate named in `[rust] unsafe_crates` of `.gatekeeper/config.toml` (e.g. `unsafe_crates = ["gpu"]`) may set `unsafe_code` to allow, warn or deny; every other crate stays at forbid.
2. A named crate states its lints in its own `[lints]` table, not `workspace = true`: the 12 clippy denies, pedantic warn and `clippy::undocumented_unsafe_blocks = "deny"`.
3. Each `unsafe {` in a named crate has a `// SAFETY:` comment on its line or in the comment lines right above it.
4. A name that is no workspace member is a finding.

- fix: put the lint back at the floor's level; a crate that needs unsafe is named in [rust] unsafe_crates with its own [lints] denying clippy::undocumented_unsafe_blocks and a // SAFETY: line over each unsafe block; a floor that is wrong is changed in gatekeeper
- budget: 30s
- cached: yes
- rerun: `./.gatekeeper/run check --gate rust/bans`
- what a failure looks like: [rust-bans](../examples/rust-bans.md)

## rust/clippy

clippy with the repo's lints (pedantic and the bans), -D warnings, the pack's clippy.toml by reference; tests may unwrap, panic, index and print

- fix: return the error; only tests may unwrap, panic, index or print; never allow a pedantic lint to pass
- budget: 180s
- cached: yes
- rerun: `./.gatekeeper/run check --gate rust/clippy`
- what a failure looks like: [rust-clippy](../examples/rust-clippy.md)

## rust/dead-code

pinned cargo-machete and the pub scan: no unused dependency, no machete ignore, no pub item that no other crate names, that only tests name, or a type nothing names, beyond the baseline

- fix: write pub(crate), and the compiler then says whether it is dead; delete an unused dependency or its machete ignore; a crate others build against goes under [rust.exported] with its reason; once one is fixed, shrink the list: ./.gatekeeper/run baseline rust/dead-code
- budget: 60s
- cached: yes
- rerun: `./.gatekeeper/run check --gate rust/dead-code`
- what a failure looks like: [rust-dead-code](../examples/rust-dead-code.md)

## rust/deny

deny.toml holds the floor (yanked, duplicates, wildcards, unknown sources denied; shared licences), then cargo deny check

- fix: deny what the floor denies, pin the dependency to a version, or drop the crate whose licence is outside the list
- budget: 20s
- cached: no, cache = false
- rerun: `./.gatekeeper/run check --gate rust/deny`
- what a failure looks like: [rust-deny](../examples/rust-deny.md)

## rust/fmt

cargo fmt --check under the repo's rustfmt.toml

- fix: run cargo fmt --all and commit the result
- budget: 10s
- cached: yes
- rerun: `./.gatekeeper/run check --gate rust/fmt`
- what a failure looks like: [rust-fmt](../examples/rust-fmt.md)
