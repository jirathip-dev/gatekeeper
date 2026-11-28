# ts/complexity: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## complexity

Writes `src/case.ts` (not shown).

The gate fails and its log prints `src/case.ts: function tangled: complexity over 20`.

Fix: split the function the line names; once one is fixed, shrink the list: ./.gatekeeper/run baseline ts/complexity. Rerun: `./.gatekeeper/run check --gate ts/complexity`

## max-lines

Writes `src/case.ts` (not shown).

The gate fails and its log prints `src/case.ts: function long: over 60 lines`.

Fix: split the function the line names; once one is fixed, shrink the list: ./.gatekeeper/run baseline ts/complexity. Rerun: `./.gatekeeper/run check --gate ts/complexity`

## second-unnamed-function

Writes `src/case.ts` (not shown).

The gate fails and its log prints `src/case.ts: function #2: complexity over 20`.

Fix: split the function the line names; once one is fixed, shrink the list: ./.gatekeeper/run baseline ts/complexity. Rerun: `./.gatekeeper/run check --gate ts/complexity`
