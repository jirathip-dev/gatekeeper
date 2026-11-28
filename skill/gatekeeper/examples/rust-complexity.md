# rust/complexity: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## cognitive

Writes `src/lib.rs` (not shown).

The gate fails and its log prints `src/lib.rs: fn tangled: cognitive complexity over 25`.

Fix: split the function the line names; an allow does not hide it; once one is fixed, shrink the list: ./.gatekeeper/run baseline rust/complexity. Rerun: `./.gatekeeper/run check --gate rust/complexity`

## too-many-lines

Writes `src/lib.rs` (not shown).

The gate fails and its log prints `src/lib.rs: fn long: over 60 lines`.

Fix: split the function the line names; an allow does not hide it; once one is fixed, shrink the list: ./.gatekeeper/run baseline rust/complexity. Rerun: `./.gatekeeper/run check --gate rust/complexity`

## under-an-allow-still-counts

Writes `src/lib.rs` (not shown).

The gate fails and its log prints `src/lib.rs: fn long: over 60 lines`.

Fix: split the function the line names; an allow does not hide it; once one is fixed, shrink the list: ./.gatekeeper/run baseline rust/complexity. Rerun: `./.gatekeeper/run check --gate rust/complexity`
